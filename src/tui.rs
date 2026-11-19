//! Terminal interface: raw ANSI, colours by role, one chat column.
//!
//! The worker thread owns the session; the UI thread owns the screen.
//! They talk over two channels, and confirmations block the worker until
//! the user answers.

use crate::agent::{Hooks, Session, Ui};
use crate::config::{ColorMode, Config};
use std::ffi::{c_int, c_ulong};
use std::io::{Read, Write};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

const ALT_ON: &str = "\x1b[?1049h\x1b[?25l\x1b[>1u";
const ALT_OFF: &str = "\x1b[<u\x1b[?25h\x1b[?1049l";

#[repr(C)]
struct Winsize {
    rows: u16,
    cols: u16,
    x: u16,
    y: u16,
}

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

#[cfg(target_os = "macos")]
const TIOCGWINSZ: c_ulong = 0x4008_7468;
#[cfg(not(target_os = "macos"))]
const TIOCGWINSZ: c_ulong = 0x5413;

fn size() -> (usize, usize) {
    let mut ws = Winsize {
        rows: 0,
        cols: 0,
        x: 0,
        y: 0,
    };
    let ok = unsafe { ioctl(1, TIOCGWINSZ, &mut ws as *mut Winsize) } == 0;
    if ok && ws.cols > 0 && ws.rows > 0 {
        (ws.cols as usize, ws.rows as usize)
    } else {
        (80, 24)
    }
}

/// The platform's `struct termios`, field for field.
///
/// glibc uses 32-bit flags, a `c_line` byte, and 32 control bytes; Darwin uses
/// 64-bit flags and 20 control bytes with no `c_line`. Getting this wrong makes
/// `tcsetattr` apply garbage, which is why raw mode silently failed on Linux.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    iflag: u64,
    oflag: u64,
    cflag: u64,
    lflag: u64,
    cc: [u8; 20],
    ispeed: u64,
    ospeed: u64,
}

#[cfg(not(target_os = "macos"))]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    iflag: u32,
    oflag: u32,
    cflag: u32,
    lflag: u32,
    line: u8,
    cc: [u8; 32],
    ispeed: u32,
    ospeed: u32,
}

#[cfg(target_os = "macos")]
type Flag = u64;
#[cfg(not(target_os = "macos"))]
type Flag = u32;

/// The flags `stty raw -echo` clears, by platform.
mod flags {
    #[cfg(target_os = "macos")]
    pub const IFLAGS: super::Flag = 0x1 | 0x2 | 0x8 | 0x20 | 0x40 | 0x80 | 0x100 | 0x200;
    #[cfg(not(target_os = "macos"))]
    pub const IFLAGS: super::Flag = 0x1 | 0x2 | 0x8 | 0x20 | 0x40 | 0x80 | 0x100 | 0x400;

    #[cfg(target_os = "macos")]
    pub const LFLAGS: super::Flag = 0x8 | 0x10 | 0x100 | 0x80 | 0x400;
    #[cfg(not(target_os = "macos"))]
    pub const LFLAGS: super::Flag = 0x8 | 0x40 | 0x2 | 0x1 | 0x8000;

    #[cfg(target_os = "macos")]
    pub const CSIZE: super::Flag = 0x300;
    #[cfg(not(target_os = "macos"))]
    pub const CSIZE: super::Flag = 0x30;

    #[cfg(target_os = "macos")]
    pub const CS8: super::Flag = 0x300;
    #[cfg(not(target_os = "macos"))]
    pub const CS8: super::Flag = 0x30;

    #[cfg(target_os = "macos")]
    pub const PARENB: super::Flag = 0x1000;
    #[cfg(not(target_os = "macos"))]
    pub const PARENB: super::Flag = 0x100;

    #[cfg(target_os = "macos")]
    pub const VMIN: usize = 16;
    #[cfg(not(target_os = "macos"))]
    pub const VMIN: usize = 6;

    #[cfg(target_os = "macos")]
    pub const VTIME: usize = 17;
    #[cfg(not(target_os = "macos"))]
    pub const VTIME: usize = 5;
}

const OPOST: Flag = 0x1;
const TCSANOW: c_int = 0;

static SAVED: std::sync::Mutex<Option<Termios>> = std::sync::Mutex::new(None);
const RESTORE: &str = "\x1b[<u\x1b[?25h\x1b[?1049l";

extern "C" {
    fn write(fd: std::ffi::c_int, buf: *const u8, len: usize) -> isize;
    fn _exit(code: std::ffi::c_int) -> !;
    fn signal(sig: std::ffi::c_int, handler: usize) -> usize;
    fn tcgetattr(fd: c_int, termios: *mut Termios) -> c_int;
    fn tcsetattr(fd: c_int, action: c_int, termios: *const Termios) -> c_int;
}

/// Put the terminal back the way it was found. Safe to call more than once.
fn restore_terminal() {
    if let Ok(mut slot) = SAVED.lock() {
        if let Some(saved) = slot.take() {
            unsafe {
                tcsetattr(0, TCSANOW, &saved);
            }
        }
    }
}

extern "C" fn on_signal(_: std::ffi::c_int) {
    if let Ok(mut slot) = SAVED.try_lock() {
        if let Some(saved) = slot.take() {
            unsafe {
                tcsetattr(0, TCSANOW, &saved);
            }
        }
    }
    unsafe {
        write(1, RESTORE.as_ptr(), RESTORE.len());
        _exit(1);
    }
}

/// Restore on panic and on SIGTERM/SIGINT, not only on a clean drop.
fn install_guards() {
    std::panic::set_hook(Box::new(|info| {
        restore_terminal();
        unsafe {
            write(1, RESTORE.as_ptr(), RESTORE.len());
        }
        eprintln!("{info}");
    }));
    unsafe {
        signal(15, on_signal as *const () as usize);
        signal(2, on_signal as *const () as usize);
    }
}

/// Raw mode through termios, so imp owns the mode it changed.
struct RawMode;

fn raw_of(saved: Termios) -> Termios {
    let mut raw = saved;
    raw.iflag &= !flags::IFLAGS;
    raw.oflag &= !OPOST;
    raw.lflag &= !flags::LFLAGS;
    raw.cflag &= !(flags::CSIZE | flags::PARENB);
    raw.cflag |= flags::CS8;
    raw.cc[flags::VMIN] = 1;
    raw.cc[flags::VTIME] = 0;
    raw
}

impl RawMode {
    fn enter() -> Self {
        // Plain old data, so zeroing is a valid starting point for tcgetattr.
        let mut saved: Termios = unsafe { std::mem::zeroed() };
        if unsafe { tcgetattr(0, &mut saved) } == 0 {
            unsafe {
                tcsetattr(0, TCSANOW, &raw_of(saved));
            }
            if let Ok(mut slot) = SAVED.lock() {
                *slot = Some(saved);
            }
        }
        print!("{ALT_ON}");
        std::io::stdout().flush().ok();
        Self
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        restore_terminal();
        print!("{ALT_OFF}");
        std::io::stdout().flush().ok();
    }
}

/// Keep Enter readable in line mode.
///
/// Canonical mode holds input until it sees a line delimiter, so a terminal
/// with its CR-to-NL mapping off never delivers a typed Enter. Turn the
/// mapping on for the session and put the original mode back on exit.
pub struct LineMode {
    saved: Option<Termios>,
}

const ICRNL: Flag = 0x100;
const IGNCR: Flag = 0x80;

impl LineMode {
    pub fn enter() -> Self {
        // Plain old data, so zeroing is a valid starting point for tcgetattr.
        let mut saved: Termios = unsafe { std::mem::zeroed() };
        if unsafe { tcgetattr(0, &mut saved) } != 0 {
            return Self { saved: None };
        }
        let mut tuned = saved;
        tuned.iflag |= ICRNL;
        tuned.iflag &= !IGNCR;
        unsafe {
            tcsetattr(0, TCSANOW, &tuned);
        }
        Self { saved: Some(saved) }
    }
}

impl Drop for LineMode {
    fn drop(&mut self) {
        if let Some(saved) = self.saved {
            unsafe {
                tcsetattr(0, TCSANOW, &saved);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    Thinking,
    Tool,
    Error,
    Info,
}

pub(crate) struct Palette {
    on: bool,
    truecolor: bool,
    theme: &'static crate::theme::Theme,
}

impl Palette {
    pub(crate) fn detect(cfg: &Config) -> Self {
        let tty = unsafe { isatty(1) } != 0;
        let theme = crate::theme::load();
        let mut on = match cfg.color {
            ColorMode::Never => false,
            ColorMode::Always => true,
            ColorMode::Auto => tty && std::env::var_os("NO_COLOR").is_none(),
        };
        if theme.name == "mono" {
            on = false;
        }
        let truecolor = std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false);
        Self {
            on,
            truecolor,
            theme,
        }
    }

    pub(crate) fn theme_name(&self) -> &'static str {
        self.theme.name
    }

    pub(crate) fn paint(&self, role: Role, text: &str) -> String {
        if !self.on {
            return text.to_string();
        }
        let index = match role {
            Role::User => 0,
            Role::Assistant => 1,
            Role::Thinking => 2,
            Role::Tool => 3,
            Role::Error => 4,
            Role::Info => 5,
        };
        let [r, g, b] = self.theme.rgb[index];
        let fallback = self.theme.ansi[index];
        if self.truecolor {
            format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
        } else {
            format!("\x1b[38;5;{fallback}m{text}\x1b[0m")
        }
    }
}


extern "C" {
    fn isatty(fd: c_int) -> c_int;
}

#[derive(Debug, Clone)]
pub enum Key {
    Char(char),
    Enter,
    Newline,
    Tab,
    BackTab,
    Backspace,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Ctrl(char),
    Unknown,
}

pub fn decode(buf: &mut Vec<u8>) -> Vec<Key> {
    let mut keys = Vec::new();
    while !buf.is_empty() {
        let b = buf[0];
        if b == 0x1b {
            if buf.len() >= 2 && buf[1] == b'[' {
                if let Some((len, key)) = control_sequence(buf) {
                    buf.drain(..len);
                    keys.push(key);
                    continue;
                }
            }
            if buf.len() >= 2 && buf[1] == b'\r' {
                buf.drain(..2);
                keys.push(Key::Newline);
                continue;
            }
            buf.remove(0);
            keys.push(Key::Esc);
            continue;
        }
        buf.remove(0);
        keys.push(match b {
            b'\r' => Key::Enter,
            b'\n' => Key::Newline,
            b'\t' => Key::Tab,
            0x7f | 0x08 => Key::Backspace,
            0x01..=0x1a => Key::Ctrl((b - 1 + b'a') as char),
            0x00 => Key::Unknown,
            _ => {
                // Multi-byte UTF-8: collect continuation bytes.
                let need = utf8_len(b);
                let mut bytes = vec![b];
                while bytes.len() < need && !buf.is_empty() {
                    bytes.push(buf.remove(0));
                }
                match std::str::from_utf8(&bytes) {
                    Ok(s) => Key::Char(s.chars().next().unwrap_or('?')),
                    Err(_) => Key::Unknown,
                }
            }
        });
    }
    keys
}

/// One complete `ESC [ ... <final>` sequence: its length and the key it means.
///
/// Scanning to the final byte keeps longer forms working, such as
/// `CSI 1 ; 2 A` for a modified arrow, instead of leaking the parameters as
/// typed characters.
fn control_sequence(buf: &[u8]) -> Option<(usize, Key)> {
    let mut i = 2;
    while i < buf.len() {
        let b = buf[i];
        if (0x40..=0x7e).contains(&b) {
            let params = &buf[2..i];
            let key = match b {
                b'A' => Key::Up,
                b'B' => Key::Down,
                b'C' => Key::Right,
                b'D' => Key::Left,
                b'H' => Key::Home,
                b'F' => Key::End,
                b'Z' => Key::BackTab,
                b'~' => match params {
                    b"5" => Key::PageUp,
                    b"6" => Key::PageDown,
                    _ => Key::Unknown,
                },
                b'u' => disambiguated(params),
                _ => Key::Unknown,
            };
            return Some((i + 1, key));
        }
        i += 1;
    }
    None
}

/// `CSI <codepoint> ; <modifier> u`, the disambiguated form of a key.
///
/// The modifier is one plus a bit set: 1 shift, 2 alt, 4 ctrl.
fn disambiguated(params: &[u8]) -> Key {
    let text = std::str::from_utf8(params).unwrap_or("");
    let mut parts = text.split(';');
    let code: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let modifiers: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    let bits = modifiers.saturating_sub(1);
    let shift = bits & 1 == 1;
    // Ctrl is bit 4 of the modifier set; the terminal reports it as the letter's
    // codepoint, so map it back to the control key the handler knows.
    if bits & 4 == 4 {
        if let Some(c) = char::from_u32(code) {
            let lower = c.to_ascii_lowercase();
            if lower.is_ascii_lowercase() {
                return Key::Ctrl(lower);
            }
        }
    }
    match code {
        13 if shift => Key::Newline,
        13 => Key::Enter,
        9 => Key::Tab,
        27 => Key::Esc,
        127 => Key::Backspace,
        _ => Key::Unknown,
    }
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

pub enum Cmd {
    Prompt(String),
    Attach(Vec<crate::api::Image>),
    SetThinking(bool),
    SetAgent(String),
    ShowAgent,
    Fork,
    SetRoot(std::path::PathBuf),
    Compact,
    Resume(String),
    Reload,
    Yolo,
    Rename(String, String),
    Delete(String),
    NewSession,
    Reset,
    Quit,
}

pub enum Ev {
    Token(String),
    Root(std::path::PathBuf),
    Think(String),
    ToolStart {
        name: String,
        summary: String,
        args: String,
    },
    ToolEnd {
        ok: bool,
        preview: String,
        output: String,
        ms: u128,
    },
    Info(String),
    Error(String),
    Usage {
        tokens: u64,
        hit: u64,
        miss: u64,
        output: u64,
        model: Option<String>,
    },
    Confirm { what: String, reply: Sender<bool> },
    Question { q: String, opts: Vec<String>, reply: Sender<Option<String>> },
    Stop(String),
    Done(Option<String>),
}

/// How long a confirmation waits before it gives up and stops the turn.
const CONFIRM_WAIT: Duration = Duration::from_secs(60);

struct WorkerHooks {
    tx: Sender<Ev>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    waited: Duration,
}

impl Hooks for WorkerHooks {
    fn ui(&mut self, event: Ui) {
        let ev = match event {
            Ui::Content(t) => Ev::Token(t),
            Ui::Reasoning(t) => Ev::Think(t),
            Ui::ToolStart { name, summary, args } => Ev::ToolStart {
                name,
                summary,
                args,
            },
            Ui::ToolEnd { ok, preview, output, ms } => Ev::ToolEnd {
                ok,
                preview,
                output,
                ms,
            },
            Ui::Info(t) => Ev::Info(t),
            Ui::Error(t) => Ev::Error(t),
        };
        let _ = self.tx.send(ev);
    }

    fn last_wait(&self) -> Duration {
        self.waited
    }

    fn confirm(&mut self, what: &str) -> bool {
        self.waited = Duration::ZERO;
        let opened = Instant::now();
        let (tx, rx) = channel();
        if self
            .tx
            .send(Ev::Confirm {
                what: what.to_string(),
                reply: tx,
            })
            .is_err()
        {
            return false;
        }
        let answered = wait_for_answer(&rx, CONFIRM_WAIT);
        self.waited = opened.elapsed();
        match answered {
            Some(answer) => answer,
            None => {
                self.cancel.store(true, Ordering::Relaxed);
                let _ = self.tx.send(Ev::Stop(format!(
                    "no answer within {}s; nothing ran and the turn stopped",
                    CONFIRM_WAIT.as_secs()
                )));
                false
            }
        }
    }

    fn run_attached(&mut self, command: &str) -> Option<i32> {
        print!("\x1b[?25h\x1b[?1049l");
        std::io::stdout().flush().ok();
        let saved = SAVED.lock().ok().and_then(|slot| *slot);
        if let Some(saved) = saved {
            unsafe {
                tcsetattr(0, TCSANOW, &saved);
            }
        }
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .status();
        if let Some(saved) = saved {
            unsafe {
                tcsetattr(0, TCSANOW, &raw_of(saved));
            }
        }
        print!("\x1b[?1049h\x1b[?25l");
        std::io::stdout().flush().ok();
        Some(status.ok().and_then(|s| s.code()).unwrap_or(-1))
    }

    fn ask(&mut self, question: &str, options: &[String]) -> Option<String> {
        let (tx, rx) = channel();
        if self
            .tx
            .send(Ev::Question {
                q: question.to_string(),
                opts: options.to_vec(),
                reply: tx,
            })
            .is_err()
        {
            return None;
        }
        rx.recv_timeout(Duration::from_secs(600)).ok().flatten()
    }
}

/// Wait for an answer. `None` means none arrived in time.
fn wait_for_answer(rx: &Receiver<bool>, wait: Duration) -> Option<bool> {
    rx.recv_timeout(wait).ok()
}

struct Entry {
    role: Role,
    text: String,
}

/// One drawn row: plain text, or styled spans from the markdown renderer.
enum Line {
    Text(String),
    Spans(Vec<crate::render::Span>),
}

/// The tool call on screen: one row, updated in place when it finishes.
struct ToolRow {
    index: usize,
    summary: String,
    args: String,
    preview: String,
    output: String,
    ok: bool,
    ms: u128,
    expanded: bool,
}

fn tool_row_text(row: &ToolRow, verbose: bool) -> String {
    let mark = if row.ok { "✓" } else { "✗" };
    let mut text = format!("{mark} {} · {} ms", row.summary, row.ms);
    if row.expanded || verbose {
        text.push_str("\n  args: ");
        text.push_str(&row.args);
        if !row.output.is_empty() {
            text.push_str("\n  ");
            text.push_str(&row.output.lines().take(20).collect::<Vec<_>>().join("\n  "));
        }
    } else if !row.preview.is_empty() && !row.ok {
        text.push('\n');
        text.push_str(&row.preview);
    }
    text
}

/// Bold the leading mark after the row has been coloured. Only the weight is
/// reset, so the row keeps its role colour.
fn bold_mark(painted: String) -> String {
    let Some(at) = painted.find(['✓', '✗']) else {
        return painted;
    };
    let mark = painted[at..].chars().next().unwrap_or('?');
    let mut out = String::with_capacity(painted.len() + 8);
    out.push_str(&painted[..at]);
    out.push_str("\x1b[1m");
    out.push(mark);
    out.push_str("\x1b[22m");
    out.push_str(&painted[at + mark.len_utf8()..]);
    out
}

pub struct App {
    cfg: Config,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    cols: std::sync::Arc<std::sync::atomic::AtomicU16>,
    palette: Palette,
    entries: Vec<Entry>,
    input: String,
    cursor: usize,
    history: Vec<String>,
    hist_pos: Option<usize>,
    follow: bool,
    top: usize,
    body_rows: usize,
    max_top: usize,
    unseen: usize,
    menu_open: bool,
    menu_index: usize,
    pending: Option<Pending>,
    pending_since: Option<Instant>,
    bell_pending: bool,
    status: Status,
    verbose: bool,
    root: std::path::PathBuf,
    agents: Vec<String>,
    sessions_dir: std::path::PathBuf,
    last_redraw: Instant,
    last_size: (usize, usize),
    dirty: bool,
    quit: bool,
    spinner: usize,
    tool: Option<ToolRow>,
}

enum Pending {
    Choose { title: String, options: Vec<String>, index: usize, action: ChooseKind },
    Confirm { what: String, reply: Sender<bool> },
    Prompt { label: String, value: String, rename_id: String },
    ConfirmDelete { id: String },
    Question { q: String, opts: Vec<String>, reply: Sender<Option<String>>, mode: AskMode, typed: String },
}

enum AskMode {
    Choose,
    Custom,
}

#[derive(Clone, Copy, PartialEq)]
enum ChooseKind {
    Agent,
    Session,
}

struct Status {
    tokens: u64,
    hit: u64,
    miss: u64,
    output: u64,
    served: String,
    streaming: bool,
}

const COMMANDS: &[(&str, &str)] = crate::help::COMMANDS;

impl App {
    pub fn new(
        cfg: Config,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
        cols: std::sync::Arc<std::sync::atomic::AtomicU16>,
        root: std::path::PathBuf,
    ) -> Self {
        let palette = Palette::detect(&cfg);
        let history = load_history();
        let agents = crate::store::load_agents(&cfg, &root)
            .into_iter()
            .map(|a| a.name)
            .collect();
        let sessions_dir = crate::config::dirs_state().join("imp").join("sessions");
        Self {
            cfg,
            cancel,
            cols,
            palette,
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            history,
            hist_pos: None,
            follow: true,
            top: 0,
            body_rows: 1,
            max_top: 0,
            unseen: 0,
            menu_open: false,
            menu_index: 0,
            pending: None,
            pending_since: None,
            bell_pending: false,
            status: Status {
                tokens: 0,
                hit: 0,
                miss: 0,
                output: 0,
                served: String::new(),
                streaming: false,
            },
            verbose: false,
            root,
            agents,
            sessions_dir,
            last_redraw: Instant::now() - Duration::from_secs(1),
            last_size: (0, 0),
            dirty: true,
            quit: false,
            spinner: 0,
            tool: None,
        }
    }

    fn push(&mut self, role: Role, text: impl Into<String>) {
        self.entries.push(Entry {
            role,
            text: text.into(),
        });
        self.dirty = true;
    }

    /// Return to the newest row after the user has scrolled away.
    fn follow_tail(&mut self) {
        self.follow = true;
        self.top = 0;
    }

    fn page(&self) -> usize {
        self.body_rows.saturating_sub(1).max(1)
    }

    fn apply(&mut self, ev: Ev) {
        match ev {
            Ev::Token(t) => {
                if let Some(last) = self.entries.last_mut() {
                    if last.role == Role::Assistant {
                        last.text.push_str(&t);
                        self.status.streaming = true;
                        self.dirty = true;
                        return;
                    }
                }
                self.push(Role::Assistant, t);
                self.status.streaming = true;
            }
            Ev::Think(t) => {
                if let Some(last) = self.entries.last_mut() {
                    if last.role == Role::Thinking {
                        last.text.push_str(&t);
                        self.dirty = true;
                        return;
                    }
                }
                self.push(Role::Thinking, t);
            }
            Ev::ToolStart { name, summary, args } => {
                let _ = name;
                self.tool = Some(ToolRow {
                    index: self.entries.len(),
                    summary: summary.clone(),
                    args,
                    preview: String::new(),
                    output: String::new(),
                    ok: true,
                    ms: 0,
                    expanded: false,
                });
                self.push(Role::Tool, format!("⠹ {summary}"))
            }
            Ev::ToolEnd {
                ok,
                preview,
                output,
                ms,
            } => {
                let role = if ok { Role::Tool } else { Role::Error };
                match self.tool.take() {
                    Some(mut row) => {
                        row.ok = ok;
                        row.ms = ms;
                        row.output = output;
                        row.preview = preview;
                        let text = tool_row_text(&row, self.verbose);
                        if let Some(entry) = self.entries.get_mut(row.index) {
                            entry.role = role;
                            entry.text = text;
                        }
                        self.tool = Some(row);
                    }
                    None => self.push(
                        role,
                        format!("  {} {preview}  · {ms} ms", if ok { "✓" } else { "✗" }),
                    ),
                }
            }
            Ev::Info(t) => self.push(Role::Info, t),
            Ev::Error(t) => self.push(Role::Error, t),
            Ev::Root(path) => {
                self.root = path;
                self.dirty = true;
            }
            Ev::Usage {
                tokens,
                hit,
                miss,
                output,
                model,
            } => {
                self.status.tokens = tokens;
                self.status.hit += hit;
                self.status.miss += miss;
                self.status.output += output;
                if let Some(model) = model {
                    self.status.served = model;
                }
                self.dirty = true;
            }
            Ev::Confirm { what, reply } => {
                self.pending = Some(Pending::Confirm { what, reply });
                self.pending_since = Some(Instant::now());
                self.bell_pending = true;
                self.dirty = true;
            }
            Ev::Question { q, opts, reply } => {
                self.pending = Some(Pending::Question {
                    q,
                    opts,
                    reply,
                    mode: AskMode::Choose,
                    typed: String::new(),
                });
                self.pending_since = Some(Instant::now());
                self.bell_pending = true;
                self.dirty = true;
            }
            Ev::Stop(reason) => {
                self.pending = None;
                self.pending_since = None;
                self.status.streaming = false;
                self.cancel.store(true, Ordering::Relaxed);
                self.push(Role::Error, reason);
                self.dirty = true;
            }
            Ev::Done(err) => {
                self.status.streaming = false;
                if let Some(e) = err {
                    self.push(Role::Error, e);
                }
                self.dirty = true;
            }
        }
    }

    fn menu_matches(&self) -> Vec<(&'static str, &'static str)> {
        let Some(rest) = self.input.strip_prefix('/') else {
            return Vec::new();
        };
        if rest.contains(' ') {
            return Vec::new();
        }
        let mut exact: Vec<_> = COMMANDS
            .iter()
            .filter(|(n, _)| n.trim_start_matches('/').starts_with(rest))
            .copied()
            .collect();
        let others: Vec<_> = COMMANDS
            .iter()
            .filter(|(n, _)| {
                let name = n.trim_start_matches('/');
                !name.starts_with(rest) && name.contains(rest)
            })
            .copied()
            .collect();
        exact.extend(others);
        exact.truncate(8);
        exact
    }

    fn handle_key(&mut self, key: Key, tx: &Sender<Cmd>) {
        // Ctrl+C means exit, whatever is on screen. Esc cancels a turn and n or
        // Esc refuses a prompt, so nothing else needs this key.
        if matches!(key, Key::Ctrl('c')) {
            self.quit = true;
            return;
        }
        if let Some(pending) = self.pending.take() {
            self.answer(pending, key, tx);
            if self.pending.is_none() {
                self.pending_since = None;
            }
            return;
        }

        if self.menu_open {
            let menu = self.menu_matches();
            match key {
                Key::Up => {
                    self.menu_index = self.menu_index.saturating_sub(1);
                    self.dirty = true;
                    return;
                }
                Key::Down => {
                    if self.menu_index + 1 < menu.len() {
                        self.menu_index += 1;
                    }
                    self.dirty = true;
                    return;
                }
                Key::Tab => {
                    if let Some((name, _)) = menu.get(self.menu_index) {
                        self.input = name.to_string();
                        self.cursor = self.input.len();
                    }
                    self.menu_open = false;
                    self.dirty = true;
                    return;
                }
                Key::Esc => {
                    self.menu_open = false;
                    self.dirty = true;
                    return;
                }
                Key::Enter => {
                    if let Some((name, _)) = menu.get(self.menu_index) {
                        self.input = name.to_string();
                        self.cursor = self.input.len();
                    }
                    self.menu_open = false;
                    self.submit(tx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            Key::Ctrl('c') => self.quit = true,
            Key::Ctrl('d') => self.quit = true,
            Key::Ctrl('o') => {
                self.toggle_tool();
                self.dirty = true;
            }
            Key::Ctrl('l') => {
                self.entries.clear();
                self.follow_tail();
                self.dirty = true;
            }
            Key::Char(c) => {
                self.follow_tail();
                self.input.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                self.sync_menu();
            }
            Key::Backspace => {
                if self.cursor > 0 {
                    self.follow_tail();
                    let prev = previous_boundary(&self.input, self.cursor);
                    self.input.replace_range(prev..self.cursor, "");
                    self.cursor = prev;
                    self.sync_menu();
                }
            }
            Key::Left => {
                self.cursor = previous_boundary(&self.input, self.cursor);
                self.dirty = true;
            }
            Key::Right => {
                if self.cursor < self.input.len() {
                    let mut c = self.cursor + 1;
                    while c < self.input.len() && !self.input.is_char_boundary(c) {
                        c += 1;
                    }
                    self.cursor = c;
                }
                self.dirty = true;
            }
            Key::Home => {
                self.cursor = 0;
                self.dirty = true;
            }
            Key::End => {
                self.cursor = self.input.len();
                self.dirty = true;
            }
            Key::PageUp => {
                self.follow = false;
                self.top = self.top.saturating_sub(self.page());
                self.dirty = true;
            }
            Key::PageDown => {
                self.top = self.top.saturating_add(self.page());
                if self.top >= self.max_top {
                    self.follow_tail();
                }
                self.dirty = true;
            }
            Key::Up => {
                let (row, col) = self.caret_position();
                if row > 0 {
                    self.cursor = self.cursor_at(row - 1, col);
                    self.dirty = true;
                } else if !self.history.is_empty() {
                    let pos = match self.hist_pos {
                        None => self.history.len() - 1,
                        Some(0) => 0,
                        Some(p) => p - 1,
                    };
                    self.hist_pos = Some(pos);
                    self.input = self.history[pos].clone();
                    self.cursor = self.input.len();
                    self.dirty = true;
                }
            }
            Key::Down => {
                let (row, col) = self.caret_position();
                if row + 1 < self.input.matches('\n').count() + 1 {
                    self.cursor = self.cursor_at(row + 1, col);
                    self.dirty = true;
                } else if let Some(p) = self.hist_pos {
                    if p + 1 < self.history.len() {
                        self.hist_pos = Some(p + 1);
                        self.input = self.history[p + 1].clone();
                    } else {
                        self.hist_pos = None;
                        self.input.clear();
                    }
                    self.cursor = self.input.len();
                    self.dirty = true;
                }
            }
            Key::Tab => {
                self.sync_menu();
                if self.menu_open {
                    let menu = self.menu_matches();
                    if let Some((name, _)) = menu.get(self.menu_index) {
                        self.input = name.to_string();
                        self.cursor = self.input.len();
                    }
                }
                self.dirty = true;
            }
            Key::Esc => {
                if self.status.streaming {
                    self.cancel.store(true, Ordering::Relaxed);
                }
                self.dirty = true;
            }
            Key::Enter => self.submit(tx),
            Key::Newline => {
                self.follow_tail();
                self.input.insert(self.cursor, '\n');
                self.cursor += 1;
                self.dirty = true;
            }
            _ => {}
        }
    }

    /// Show or hide the full arguments and output of the last tool call.
    fn toggle_tool(&mut self) {
        let Some(mut row) = self.tool.take() else {
            return;
        };
        row.expanded = !row.expanded;
        let text = tool_row_text(&row, self.verbose);
        if let Some(entry) = self.entries.get_mut(row.index) {
            entry.text = text;
        }
        self.tool = Some(row);
    }

    fn sync_menu(&mut self) {
        self.menu_open = self.input.starts_with('/') && !self.input.contains(' ');
        self.menu_index = 0;
        self.dirty = true;
    }

    fn submit(&mut self, tx: &Sender<Cmd>) {
        if self.input.trim().is_empty() {
            return;
        }
        let text = self.input.trim().to_string();
        self.input.clear();
        self.cursor = 0;
        self.menu_open = false;
        self.hist_pos = None;
        self.follow_tail();

        if let Some(cmd) = text.strip_prefix('/') {
            if !cmd.contains(' ') {
                self.run_command(cmd, tx);
                return;
            }
        }

        if let Some(rest) = text.strip_prefix('!') {
            self.push(Role::User, text.clone());
            self.run_local(rest);
            return;
        }

        let (mut text, images) = extract_images(&text, &self.root);
        if !images.is_empty() && text.trim().is_empty() {
            text = "Look at the attached image.".into();
        }
        for image in &images {
            self.push(Role::Thinking, crate::images::marker(image));
        }
        if !images.is_empty() {
            let _ = tx.send(Cmd::Attach(images));
        }

        self.history.push(text.clone());
        save_history(&self.history);
        if self.history.len() > 1000 {
            self.history.drain(..self.history.len() - 1000);
        }
        self.push(Role::User, text.clone());
        self.status.streaming = true;
        let _ = tx.send(Cmd::Prompt(text));
    }

    fn run_local(&mut self, command: &str) {
        // Same runner as the bash tool, one implementation of the limits.
        let mut tools = crate::tools::Tools::new(
            self.root.clone(),
            crate::config::dirs_state().join("imp").join("out"),
        );
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let out = tools.bash(command, &cancel);
        let role = if out.ok { Role::Tool } else { Role::Error };
        self.push(role, format!("$ {command}\n{}", out.text));
    }

    fn run_command(&mut self, cmd: &str, tx: &Sender<Cmd>) {
        match cmd {
            "help" => self.push(Role::Info, crate::help::commands()),
            "quit" | "exit" => self.quit = true,
            other if other == "agent" || other.starts_with("agent ") => {
                let rest = other.strip_prefix("agent").unwrap_or("").trim();
                match rest {
                    "" => {
                        let options = self.agents.clone();
                        self.pending = Some(Pending::Choose {
                            title: "agent".into(),
                            options,
                            index: 0,
                            action: ChooseKind::Agent,
                        });
                        self.dirty = true;
                    }
                    "current" => {
                        let _ = tx.send(Cmd::ShowAgent);
                    }
                    name => {
                        let _ = tx.send(Cmd::SetAgent(name.to_string()));
                    }
                }
            }
            "yolo" => {
                let _ = tx.send(Cmd::Yolo);
            }
            "reload" => {
                let _ = tx.send(Cmd::Reload);
                self.push(Role::Info, "reloading instructions and skills…");
            }
            "sessions" => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let mut options: Vec<String> = crate::store::Log::list(&self.sessions_dir)
                    .into_iter()
                    .map(|m| crate::store::format_session(&m, now))
                    .collect();
                options.truncate(20);
                self.pending = Some(Pending::Choose {
                    title: "session (Enter resumes after restart)".into(),
                    options,
                    index: 0,
                    action: ChooseKind::Session,
                });
                self.dirty = true;
            }
            "compact" => {
                let _ = tx.send(Cmd::Compact);
                self.push(Role::Info, "compacting…");
            }
            "think" => {
                self.cfg.thinking = !self.cfg.thinking;
                let _ = tx.send(Cmd::SetThinking(self.cfg.thinking));
                self.push(
                    Role::Info,
                    format!("thinking {} for the next turn", if self.cfg.thinking { "on" } else { "off" }),
                );
            }
            "new" => {
                let _ = tx.send(Cmd::Reset);
                self.entries.clear();
                self.push(Role::Info, "new conversation");
            }
            "cwd" => self.push(Role::Info, self.root.display().to_string()),
            "fork" => {
                let _ = tx.send(Cmd::Fork);
            }
            other if other == "theme" || other.starts_with("theme ") => {
                let rest = other.strip_prefix("theme").unwrap_or("").trim();
                if rest.is_empty() {
                    let names: Vec<String> = crate::theme::names()
                        .into_iter()
                        .map(|n| {
                            if n == self.palette.theme_name() {
                                format!("*{n}")
                            } else {
                                n.to_string()
                            }
                        })
                        .collect();
                    self.push(Role::Info, names.join("  "));
                } else if crate::theme::by_name(rest).is_some() {
                    let _ = crate::theme::save(rest);
                    self.palette = Palette::detect(&self.cfg);
                    self.push(Role::Info, format!("theme: {rest}"));
                } else {
                    self.push(
                        Role::Error,
                        format!("no theme {rest}; use {}", crate::theme::names().join(", ")),
                    );
                }
            }
            other if other == "image" || other.starts_with("image ") => {
                let raw = other.strip_prefix("image").unwrap_or("").trim();
                if raw.is_empty() {
                    self.push(Role::Info, "image <path>");
                } else {
                    match crate::images::attach(raw, &self.root) {
                        Ok(image) => {
                            self.push(Role::Thinking, crate::images::marker(&image));
                            let _ = tx.send(Cmd::Attach(vec![image]));
                        }
                        Err(e) => self.push(Role::Error, e),
                    }
                }
            }
            other if other == "cd" || other.starts_with("cd ") => {
                let raw = other[2..].trim();
                if raw.is_empty() {
                    self.push(Role::Info, "cd <path>");
                } else {
                    match crate::config::resolve_root(Some(raw), &self.root) {
                        Ok(path) => {
                            let _ = tx.send(Cmd::SetRoot(path.clone()));
                            self.push(Role::Info, format!("cwd: {}", path.display()));
                        }
                        Err(e) => self.push(Role::Error, e),
                    }
                }
            }
            other if other == "verbosity" || other.starts_with("verbosity ") => {
                let level = other.strip_prefix("verbosity").unwrap_or("").trim();
                match level {
                    "" => self.push(
                        Role::Info,
                        format!(
                            "verbosity: {}",
                            if self.verbose { "high" } else { "low" }
                        ),
                    ),
                    "high" => {
                        self.verbose = true;
                        self.push(Role::Info, "verbosity: high");
                    }
                    "low" => {
                        self.verbose = false;
                        self.push(Role::Info, "verbosity: low");
                    }
                    _ => self.push(
                        Role::Error,
                        format!("unknown verbosity {level:?}; use low or high"),
                    ),
                }
            }
            other => self.push(Role::Info, crate::help::unknown(other)),
        }
    }

    fn answer(&mut self, pending: Pending, key: Key, tx: &Sender<Cmd>) {
        match pending {
            Pending::Prompt { label, mut value, rename_id } => {
                match key {
                    Key::Esc => {
                        self.push(Role::Info, "rename cancelled");
                        self.dirty = true;
                    }
                    Key::Enter => {
                        let _ = tx.send(Cmd::Rename(rename_id, value.clone()));
                        self.push(Role::Info, format!("renamed to {value:?}"));
                        self.dirty = true;
                    }
                    Key::Char(c) => {
                        value.push(c);
                        self.pending = Some(Pending::Prompt { label, value, rename_id });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        value.pop();
                        self.pending = Some(Pending::Prompt { label, value, rename_id });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::Prompt { label, value, rename_id });
                    }
                }
                return;
            }
            Pending::ConfirmDelete { id } => {
                match key {
                    Key::Char('y') | Key::Char('Y') | Key::Enter => {
                        let _ = tx.send(Cmd::Delete(id.clone()));
                        self.push(Role::Info, format!("deleted {id}"));
                    }
                    _ => self.push(Role::Info, "delete cancelled"),
                }
                self.dirty = true;
                return;
            }
            Pending::Choose { title, options, mut index, action } => {
                match key {
                    Key::Up => index = index.saturating_sub(1),
                    Key::Down => {
                        if index + 1 < options.len() {
                            index += 1
                        }
                    }
                    Key::Char(c) if c.is_ascii_digit() => {
                        let n = c.to_digit(10).unwrap_or(0) as usize;
                        if n >= 1 && n <= options.len().min(9) {
                            index = n - 1;
                        }
                    }
                    Key::Char('n') => {
                        let _ = tx.send(Cmd::NewSession);
                        self.entries.clear();
                        self.push(Role::Info, "new conversation");
                        self.dirty = true;
                        return;
                    }
                    Key::Char('r') => {
                        if let Some(choice) = options.get(index) {
                            let id = choice.split_whitespace().next().unwrap_or("").to_string();
                            self.pending = Some(Pending::Prompt {
                                label: format!("new title for {id}"),
                                value: String::new(),
                                rename_id: id,
                            });
                        }
                        self.dirty = true;
                        return;
                    }
                    Key::Char('d') => {
                        if let Some(choice) = options.get(index) {
                            let id = choice.split_whitespace().next().unwrap_or("").to_string();
                            self.pending = Some(Pending::ConfirmDelete { id });
                        }
                        self.dirty = true;
                        return;
                    }
                    Key::Enter => {
                        if let Some(choice) = options.get(index) {
                            match action {
                                ChooseKind::Agent => {
                                    let name = choice.clone();
                                    let _ = tx.send(Cmd::SetAgent(name.clone()));
                                    self.push(Role::Info, format!("agent: {name}"));
                                }
                                ChooseKind::Session => {
                                    let id = choice
                                        .split_whitespace()
                                        .next()
                                        .unwrap_or("")
                                        .to_string();
                                    self.entries.clear();
                                    self.push(Role::Info, format!("resuming {id}"));
                                    let _ = tx.send(Cmd::Resume(id));
                                }
                            }
                        }
                        self.dirty = true;
                        return;
                    }
                    Key::Esc => {
                        self.dirty = true;
                        return;
                    }
                    _ => {}
                }
                self.pending = Some(Pending::Choose { title, options, index, action });
                self.dirty = true;
            }
            Pending::Confirm { what, reply } => {
                let yes = matches!(key, Key::Char('y') | Key::Char('Y') | Key::Enter);
                if matches!(key, Key::Esc) || matches!(key, Key::Char('n') | Key::Char('N')) {
                    self.push(Role::Info, format!("declined: {what}"));
                } else if yes {
                    // The tool row carries the result, so approval needs no row of its own.
                } else {
                    self.pending = Some(Pending::Confirm { what, reply });
                    return;
                }
                let _ = reply.send(yes);
                self.dirty = true;
            }
            Pending::Question { q, opts, reply, mode, mut typed } => {
                match mode {
                    AskMode::Choose => match key {
                        Key::Char('e') => {
                            self.pending = Some(Pending::Question {
                                q,
                                opts,
                                reply,
                                mode: AskMode::Custom,
                                typed,
                            });
                            self.dirty = true;
                        }
                        Key::Esc => {
                            let _ = reply.send(None);
                            self.dirty = true;
                        }
                        Key::Char('y') | Key::Char('Y') if opts.len() == 2 => {
                            let _ = reply.send(Some(opts[0].clone()));
                            self.dirty = true;
                        }
                        Key::Char('n') | Key::Char('N') if opts.len() == 2 => {
                            let _ = reply.send(Some(opts[1].clone()));
                            self.dirty = true;
                        }
                        Key::Char(c) if c.is_ascii_digit() => {
                            let n = c.to_digit(10).unwrap_or(0) as usize;
                            if n >= 1 && n <= opts.len() {
                                let _ = reply.send(Some(opts[n - 1].clone()));
                                self.dirty = true;
                            }
                        }
                        _ => {
                            self.pending = Some(Pending::Question {
                                q,
                                opts,
                                reply,
                                mode: AskMode::Choose,
                                typed,
                            });
                        }
                    },
                    AskMode::Custom => match key {
                        Key::Esc => {
                            let _ = reply.send(None);
                            self.dirty = true;
                        }
                        Key::Enter => {
                            let _ = reply.send(Some(typed.clone()));
                            self.dirty = true;
                        }
                        Key::Char(c) => {
                            typed.push(c);
                            self.pending = Some(Pending::Question {
                                q,
                                opts,
                                reply,
                                mode: AskMode::Custom,
                                typed,
                            });
                            self.dirty = true;
                        }
                        Key::Backspace => {
                            typed.pop();
                            self.pending = Some(Pending::Question {
                                q,
                                opts,
                                reply,
                                mode: AskMode::Custom,
                                typed,
                            });
                            self.dirty = true;
                        }
                        _ => {
                            self.pending = Some(Pending::Question {
                                q,
                                opts,
                                reply,
                                mode: AskMode::Custom,
                                typed,
                            });
                        }
                    },
                }
            }
        }
    }

    /// A frame is due only when a key or event arrived or something visible
    /// changed. Nothing redraws on a timer.
    fn wants_render(&self, redraw: bool) -> bool {
        redraw || self.dirty
    }

    /// The spinner advances at most ten times a second.
    fn spinner_due(&self) -> bool {
        self.status.streaming && self.last_redraw.elapsed() >= Duration::from_millis(100)
    }

    fn resized(&self, cols: usize, rows: usize) -> bool {
        self.last_size != (cols, rows)
    }

    pub fn render(&mut self) {
        let (cols, rows) = size();
        self.last_size = (cols, rows);
        self.cols.store(cols as u16, Ordering::Relaxed);
        if rows < 5 || cols < 20 {
            return;
        }
        let screen = self.frame(cols, rows);
        let mut out = std::io::stdout();
        out.write_all(screen.as_bytes()).ok();
        out.flush().ok();
        self.bell_pending = false;
        self.dirty = false;
        self.last_redraw = Instant::now();
    }

    /// One frame as bytes. No terminal access, so it can be tested.
    pub fn frame(&mut self, cols: usize, rows: usize) -> String {
        let menu = if self.menu_open { self.menu_matches() } else { Vec::new() };
        let menu_rows = menu.len().min(6);
        let composer = self.composer(cols, rows.saturating_sub(1 + menu_rows).max(1));
        let body_rows = rows
            .saturating_sub(1 + menu_rows + composer.len())
            .max(1);

        let markdown = self.cfg.links.supported();
        let mut lines: Vec<(Role, Line)> = Vec::new();
        for entry in &self.entries {
            if entry.role == Role::Assistant {
                for row in crate::render::lines(&entry.text, cols, markdown) {
                    lines.push((entry.role, Line::Spans(row)));
                }
                continue;
            }
            for (i, part) in wrap(&entry.text, cols.saturating_sub(2)).into_iter().enumerate() {
                let prefix = if i == 0 {
                    match entry.role {
                        Role::User => "> ",
                        Role::Thinking => "~ ",
                        Role::Tool => "  ",
                        _ => "",
                    }
                } else {
                    "  "
                };
                lines.push((entry.role, Line::Text(format!("{prefix}{part}"))));
            }
        }

        let total = lines.len();
        let max_top = total.saturating_sub(body_rows);
        self.body_rows = body_rows;
        self.max_top = max_top;
        if self.follow {
            self.top = max_top;
        }
        self.top = self.top.min(max_top);
        self.unseen = total.saturating_sub(self.top + body_rows);
        let start = self.top;
        let end = (start + body_rows).min(total);
        let visible = &lines[start..end];

        let mut screen = String::from("\x1b[?7l\x1b[H");
        for _ in 0..body_rows.saturating_sub(visible.len()) {
            screen.push_str("\x1b[2K\r\n");
        }
        for (role, line) in visible.iter().take(body_rows) {
            screen.push_str("\x1b[2K");
            match line {
                Line::Text(text) => {
                    let painted = self.palette.paint(*role, &fit(text, cols));
                    if self.palette.on && matches!(role, Role::Tool | Role::Error) {
                        screen.push_str(&bold_mark(painted));
                    } else {
                        screen.push_str(&painted);
                    }
                }
                Line::Spans(spans) => screen.push_str(&self.paint_spans(*role, spans)),
            }
            screen.push_str("\r\n");
        }

        for row in &composer {
            screen.push_str("\x1b[2K");
            screen.push_str(row);
            screen.push_str("\r\n");
        }

        for (i, (name, desc)) in menu.iter().take(6).enumerate() {
            let line = fit(&format!(" {name:<10} {desc}"), cols);
            let painted = if i == self.menu_index {
                self.palette.paint(Role::User, &line)
            } else {
                line
            };
            screen.push_str("\x1b[2K");
            screen.push_str(&painted);
            screen.push_str("\r\n");
        }

        screen.push_str("\x1b[2K\x1b[J");
        screen.push_str(&self.palette.paint(Role::Thinking, &self.status_line(cols)));
        screen.push_str("\x1b[?7h");

        screen
    }


    /// Paint one row of styled spans, with a hyperlink around each linked run.
    fn paint_spans(&self, role: Role, spans: &[crate::render::Span]) -> String {
        let mut out = String::new();
        for span in spans {
            let colour = if span.style.code {
                Role::Tool
            } else if span.style.dim {
                Role::Thinking
            } else {
                role
            };
            let mut text = self.palette.paint(colour, &span.text);
            if self.palette.on {
                if span.style.bold || span.style.code {
                    text = format!("\x1b[1m{text}");
                }
                if span.style.italic {
                    text = format!("\x1b[3m{text}");
                }
            }
            if let Some(url) = &span.link {
                text = crate::render::hyperlink(url, &text);
            }
            out.push_str(&text);
        }
        out
    }

    /// A single row for an open question or confirmation, if any.
    fn pending_row(&self, cols: usize) -> Option<String> {
        if let Some(pending) = &self.pending {
            let text = match pending {
                Pending::Choose { title, options, index, .. } => {
                    let shown: Vec<String> = options
                        .iter()
                        .enumerate()
                        .map(|(i, o)| {
                            let marker = if i == *index { "▸" } else { " " };
                            format!("{marker}{}. {o}", i + 1)
                        })
                        .collect();
                    format!("{title}: {}", shown.join("  "))
                }
                Pending::Confirm { what, .. } => {
                    format!("{} y/N  ({}s)", verb(what), self.seconds_left())
                }
                Pending::ConfirmDelete { id } => format!("delete {id}? y/N"),
                Pending::Prompt { label, value, .. } => format!("{label}: {value}_"),
                Pending::Question { q, opts, mode, typed, .. } => {
                    let mut text = format!("{q}   ");
                    for (i, o) in opts.iter().enumerate() {
                        text.push_str(&format!("[{}] {o}  ", i + 1));
                    }
                    match mode {
                        AskMode::Choose => text.push_str("[e] answer   Esc skip"),
                        AskMode::Custom => text.push_str(&format!("> {typed}_")),
                    }
                    text
                }
            };
            let text = if self.bell_pending {
                format!("\u{7}{text}")
            } else {
                text
            };
            return Some(self.palette.paint(Role::Info, &fit(&text, cols)));
        }
        None
    }

    /// The input area: one painted row per newline in the input.
    ///
    /// `space` is what is left after the status line and the menu. The caret
    /// keeps its own row visible, and the area never takes more than eight rows
    /// or half the space.
    fn composer(&self, cols: usize, space: usize) -> Vec<String> {
        if let Some(row) = self.pending_row(cols) {
            return vec![row];
        }
        let budget = (space / 2).clamp(1, 8);
        // A shell draft hides the `!` behind the prompt glyph.
        let shell = self.input.starts_with('!');
        let (text, cursor) = if shell {
            (&self.input[1..], self.cursor.saturating_sub(1))
        } else {
            (&self.input[..], self.cursor)
        };
        let cursor = cursor.min(text.len());
        let upto = &text[..cursor];
        let caret_row = upto.matches('\n').count();
        let caret_col = cursor - upto.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let mut plain = Vec::new();
        for (i, line) in text.split('\n').enumerate() {
            let mut body = line.to_string();
            if i == caret_row {
                body.insert(caret_col.min(body.len()), CARET);
            }
            let prefix = if i != 0 {
                "  "
            } else if shell {
                "$ "
            } else {
                "> "
            };
            plain.push(format!("{prefix}{body}"));
        }
        let mut start = plain.len().saturating_sub(budget);
        if caret_row < start {
            start = caret_row;
        }
        let end = (start + budget).min(plain.len());
        plain[start..end]
            .iter()
            .map(|row| self.paint_composer_row(row, cols))
            .collect()
    }

    /// Paint one composer row, keeping the tail when it is wider than the screen.
    fn paint_composer_row(&self, row: &str, cols: usize) -> String {
        let prefix = row.get(..2).unwrap_or(row);
        let body = row.get(2..).unwrap_or("");
        let fitted = tail(body, cols.saturating_sub(2));
        self.palette.paint(Role::User, prefix) + &fitted.replace(CARET, "\x1b[7m \x1b[0m")
    }

    /// Row and byte column of the caret within the input.
    fn caret_position(&self) -> (usize, usize) {
        let cursor = self.cursor.min(self.input.len());
        let upto = &self.input[..cursor];
        let row = upto.matches('\n').count();
        let col = upto.rfind('\n').map(|i| i + 1).unwrap_or(0);
        (row, cursor - col)
    }

    /// Seconds left before an open confirmation gives up.
    fn seconds_left(&self) -> u64 {
        let spent = self.pending_since.map(|t| t.elapsed()).unwrap_or_default();
        CONFIRM_WAIT.saturating_sub(spent).as_secs()
    }

    /// Byte offset of a row and column, clamped to what the row holds.
    fn cursor_at(&self, row: usize, col: usize) -> usize {
        let mut offset = 0;
        for (i, line) in self.input.split('\n').enumerate() {
            if i == row {
                let mut at = col.min(line.len());
                while at > 0 && !line.is_char_boundary(at) {
                    at -= 1;
                }
                return offset + at;
            }
            offset += line.len() + 1;
        }
        self.input.len()
    }

    fn status_line(&self, cols: usize) -> String {
        if self.input.starts_with('!') && !self.status.streaming {
            return fit(" shell · local · not sent to the model", cols);
        }
        let spin = if self.status.streaming {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
            frames[self.spinner % frames.len()].to_string()
        } else {
            "·".into()
        };
        let thinking = if self.cfg.thinking { " · think" } else { "" };
        let unseen = if self.unseen > 0 {
            format!(" · ↑ {}", self.unseen)
        } else {
            String::new()
        };
        let money = crate::pricing::cost(
            &self.cfg.model,
            crate::pricing::now(),
            self.status.hit,
            self.status.miss,
            self.status.output,
        )
        .map(|c| format!(" · ${c:.2}"))
        .unwrap_or_default();
        let model = if self.status.served.is_empty() || self.status.served == self.cfg.model {
            self.cfg.model.clone()
        } else {
            format!("{} ({})", self.cfg.model, self.status.served)
        };
        fit(
            &format!(
                " {spin} {model} · {} tok{money}{unseen}{thinking}",
                compact_tokens(self.status.tokens),
            ),
            cols,
        )
    }
}

/// 1234 draws as `1.2K`, 1_200_000 as `1.2M`. Below a thousand stays exact.
fn compact_tokens(n: u64) -> String {
    fn scaled(value: f64, unit: &str) -> String {
        let text = format!("{value:.1}");
        let text = text.strip_suffix(".0").unwrap_or(&text);
        format!("{text}{unit}")
    }
    if n >= 1_000_000 {
        scaled(n as f64 / 1_000_000.0, "M")
    } else if n >= 1_000 {
        scaled(n as f64 / 1_000.0, "K")
    } else {
        n.to_string()
    }
}

fn extract_images(text: &str, root: &std::path::Path) -> (String, Vec<crate::api::Image>) {
    let mut keep = Vec::new();
    let mut images = Vec::new();
    for token in text.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        let looks_image = [".png", ".jpg", ".jpeg", ".gif", ".webp"]
            .iter()
            .any(|ext| lower.ends_with(ext));
        if looks_image {
            if let Ok(image) = crate::images::attach(token, root) {
                images.push(image);
                continue;
            }
        }
        keep.push(token);
    }
    (keep.join(" "), images)
}

fn fit(text: &str, cols: usize) -> String {
    if text.chars().count() <= cols {
        return text.to_string();
    }
    let mut out: String = text.chars().take(cols.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Marks the caret cell before colour escapes are added, so truncating a row
/// can never cut a control sequence in half.
const CARET: char = '\u{0}';

/// Keep the end of a row, dropping the left and marking the cut with an ellipsis.
fn tail(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let keep: String = text.chars().skip(count - (width - 1)).collect();
    format!("…{keep}")
}

/// The prompt asks with the verb alone; the tool row above names the command.
fn verb(what: &str) -> &'static str {
    match what.split_whitespace().next().unwrap_or("") {
        "run" => "run it?",
        "write" => "write it?",
        "save" => "save it?",
        "delete" => "delete it?",
        _ => "approve?",
    }
}


fn previous_boundary(s: &str, mut i: usize) -> usize {
    if i == 0 {
        return 0;
    }
    i -= 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut out = Vec::new();
    for raw in text.split('\n') {
        if raw.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in raw.split(' ') {
            if line.is_empty() {
                line.push_str(word);
            } else if line.chars().count() + 1 + word.chars().count() <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(std::mem::take(&mut line));
                line.push_str(word);
            }
            while line.chars().count() > width {
                let cut = line
                    .char_indices()
                    .nth(width)
                    .map(|(i, _)| i)
                    .unwrap_or(line.len());
                let rest = line.split_off(cut);
                out.push(std::mem::take(&mut line));
                line = rest;
            }
        }
        out.push(line);
    }
    out
}

fn history_path() -> std::path::PathBuf {
    crate::config::dirs_state().join("imp").join("history")
}

fn load_history() -> Vec<String> {
    std::fs::read_to_string(history_path())
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn save_history(history: &[String]) {
    let path = history_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut deduped: Vec<&String> = Vec::new();
    for h in history {
        if deduped.last() != Some(&h) {
            deduped.push(h);
        }
    }
    let body: String = deduped.iter().map(|l| format!("{l}\n")).collect();
    let _ = std::fs::write(path, body);
}

/// Run the interface until the user quits. Returns the final session id.
pub fn run(session: Session, cfg: Config) -> Result<Option<String>, String> {
    install_guards();
    let _raw = RawMode::enter();
    if std::env::var_os("IMP_FORCE_PANIC").is_some() {
        panic!("forced panic for the terminal-guard test");
    }
    let mut app = App::new(
        cfg,
        session.cancel.clone(),
        session.cols.clone(),
        session.tools.root.clone(),
    );

    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (ev_tx, ev_rx) = channel::<Ev>();

    let sessions_dir = crate::config::dirs_state().join("imp").join("sessions");
    let mut worker_session = session;
    let session_id = std::sync::Arc::new(std::sync::Mutex::new(
        worker_session
            .log
            .as_ref()
            .map(|l| l.id.clone())
            .unwrap_or_default(),
    ));
    std::thread::spawn({
        let ev_tx = ev_tx.clone();
        let session_id = session_id.clone();
        move || loop {
            match cmd_rx.recv() {
                Ok(Cmd::Attach(images)) => {
                    for image in images {
                        if let Err(e) = worker_session.attach_image(image) {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                }
                Ok(Cmd::Prompt(text)) => {
                    let mut hooks = WorkerHooks {
                        tx: ev_tx.clone(),
                        cancel: worker_session.cancel.clone(),
                        waited: Duration::ZERO,
                    };
                    let result = worker_session.turn(&text, &mut hooks);
                    let _ = ev_tx.send(Ev::Usage {
                        tokens: crate::api::estimate_tokens(&worker_session.history),
                        hit: worker_session.last_usage.0,
                        miss: worker_session.last_usage.1,
                        output: worker_session.last_usage.2,
                        model: worker_session.last_model.clone(),
                    });
                    let _ = ev_tx.send(Ev::Done(result.err()));
                }
                Ok(Cmd::SetThinking(on)) => worker_session.cfg.thinking = on,
                Ok(Cmd::SetAgent(name)) => {
                    let found = worker_session
                        .roster
                        .iter()
                        .find(|a| a.name == name)
                        .cloned();
                    if let Some(agent) = found {
                        let mut hooks = WorkerHooks {
                            tx: ev_tx.clone(),
                            cancel: worker_session.cancel.clone(),
                            waited: Duration::ZERO,
                        };
                        let applied = worker_session.switch_agent(agent);
                        let _ = ev_tx.send(Ev::Info(format!("agent: {applied}")));
                        let _ = &mut hooks;
                    } else {
                        let names: Vec<String> =
                            worker_session.roster.iter().map(|a| a.name.clone()).collect();
                        let _ = ev_tx.send(Ev::Error(format!(
                            "no agent {name}; available: {}",
                            names.join(", ")
                        )));
                    }
                }
                Ok(Cmd::Fork) => match worker_session.fork() {
                    Ok((new_id, old_id)) => {
                        if let Ok(mut slot) = session_id.lock() {
                            *slot = new_id.clone();
                        }
                        let _ = ev_tx.send(Ev::Info(format!("fork: {new_id} (from {old_id})")));
                    }
                    Err(e) => {
                        let _ = ev_tx.send(Ev::Error(e));
                    }
                },
                Ok(Cmd::ShowAgent) => {
                    let _ = ev_tx.send(Ev::Info(crate::agent::describe_agent(
                        &worker_session.agent,
                        &worker_session.cfg,
                    )));
                }
                Ok(Cmd::SetRoot(path)) => match worker_session.set_root(path.clone()) {
                    Ok(()) => {
                        let _ = ev_tx.send(Ev::Root(path.clone()));
                        let _ = ev_tx.send(Ev::Info(format!(
                            "cwd: {} (prompt cache reset)",
                            path.display()
                        )));
                    }
                    Err(e) => {
                        let _ = ev_tx.send(Ev::Error(e));
                    }
                },
                Ok(Cmd::Resume(id)) => {
                    match worker_session.resume_id(&sessions_dir, &id) {
                        Ok(()) => {
                            if let Ok(mut slot) = session_id.lock() {
                                *slot = id.clone();
                            }
                            let _ = ev_tx.send(Ev::Info(format!("resumed {id}")));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                    let _ = ev_tx.send(Ev::Done(None));
                }
                Ok(Cmd::NewSession) => {
                    let log = crate::store::Log::create(
                        &sessions_dir,
                        &worker_session.tools.root,
                        &worker_session.cfg.model,
                    );
                    worker_session.reset();
                    worker_session.attach_log(log);
                    worker_session.tools.clear_buffers();
                    if let (Some(log), Ok(mut slot)) = (&worker_session.log, session_id.lock()) {
                        *slot = log.id.clone();
                    }
                    let _ = ev_tx.send(Ev::Info("new session".into()));
                    let _ = ev_tx.send(Ev::Done(None));
                }
                Ok(Cmd::Rename(id, title)) => {
                    match crate::store::Log::rename(&sessions_dir, &id, &title) {
                        Ok(()) => {
                            let _ = ev_tx.send(Ev::Info(format!("renamed {id}")));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                    let _ = ev_tx.send(Ev::Done(None));
                }
                Ok(Cmd::Delete(id)) => {
                    match crate::store::Log::delete(&sessions_dir, &id) {
                        Ok(()) => {
                            let _ = ev_tx.send(Ev::Info(format!("deleted {id}")));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                    let _ = ev_tx.send(Ev::Done(None));
                }
                Ok(Cmd::Yolo) => {
                    let line = worker_session.toggle_yolo();
                    let _ = ev_tx.send(Ev::Info(format!("{line} ({})", worker_session.cfg.permissions.as_str())));
                }
                Ok(Cmd::Reload) => {
                    match worker_session.reload() {
                        Ok(()) => {
                            let _ = ev_tx.send(Ev::Info(
                                "reloaded instructions and skills; prompt cache reset".to_string(),
                            ));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                    let _ = ev_tx.send(Ev::Done(None));
                }
                Ok(Cmd::Compact) => {
                    let mut hooks = WorkerHooks {
                        tx: ev_tx.clone(),
                        cancel: worker_session.cancel.clone(),
                    waited: Duration::ZERO,
                    };
                    if let Err(e) = worker_session.compact(&mut hooks) {
                        let _ = ev_tx.send(Ev::Error(e));
                    }
                    let _ = ev_tx.send(Ev::Done(None));
                }
                Ok(Cmd::Reset) => {
                    let system = worker_session
                        .history
                        .first()
                        .filter(|m| m.role == crate::api::Role::System)
                        .cloned();
                    worker_session.history.clear();
                    worker_session.tools.clear_buffers();
                    if let Some(m) = system {
                        worker_session.history.push(m);
                    }
                }
                Ok(Cmd::Quit) | Err(_) => break,
            }
        }
    });

    // Key reader.
    let (key_tx, key_rx) = channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 64];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if key_tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut pending_bytes: Vec<u8> = Vec::new();

    app.render();
    while !app.quit {
        let mut redraw = false;
        match key_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(bytes) => {
                pending_bytes.extend(bytes);
                for key in decode(&mut pending_bytes) {
                    app.handle_key(key, &cmd_tx);
                }
                redraw = true;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Ten frames a second is enough for a spinner. Repainting faster
                // flickers under software terminal renderers.
                if app.spinner_due() {
                    app.spinner += 1;
                    redraw = true;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        loop {
            match ev_rx.try_recv() {
                Ok(ev) => {
                    app.apply(ev);
                    redraw = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    app.quit = true;
                    break;
                }
            }
        }

        // A resize has to be noticed without repainting on a timer.
        let (cols, rows) = size();
        if app.resized(cols, rows) {
            app.dirty = true;
        }

        if app.wants_render(redraw) {
            app.render();
        }
    }

    let _ = cmd_tx.send(Cmd::Quit);
    let id = session_id
        .lock()
        .ok()
        .map(|slot| slot.clone())
        .filter(|id| !id.is_empty());
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_arrows_and_text() {
        let mut buf = b"\x1b[Aa\x7f\r".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 4);
        assert!(matches!(keys[0], Key::Up));
        assert!(matches!(keys[1], Key::Char('a')));
        assert!(matches!(keys[2], Key::Backspace));
        assert!(matches!(keys[3], Key::Enter));
        assert!(buf.is_empty());
    }

    #[test]
    fn decodes_utf8() {
        let mut buf = "é".as_bytes().to_vec();
        assert!(matches!(decode(&mut buf)[0], Key::Char('é')));
    }

    #[test]
    fn alt_enter_is_newline() {
        let mut buf = b"\x1b\r".to_vec();
        assert!(matches!(decode(&mut buf)[0], Key::Newline));
    }

    #[test]
    fn shift_enter_is_newline() {
        let mut buf = b"\x1b[13;2u".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 1);
        assert!(matches!(keys[0], Key::Newline));
        assert!(buf.is_empty());
    }

    #[test]
    fn disambiguated_enter_and_escape() {
        let mut buf = b"\x1b[13u\x1b[27u".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 2);
        assert!(matches!(keys[0], Key::Enter));
        assert!(matches!(keys[1], Key::Esc));
    }

    #[test]
    fn modified_arrow_takes_no_parameters_as_text() {
        let mut buf = b"\x1b[1;2A".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 1);
        assert!(matches!(keys[0], Key::Up));
    }

    #[test]
    fn wrap_respects_width() {
        let lines = wrap("alpha beta gamma delta", 10);
        assert!(lines.iter().all(|l| l.chars().count() <= 10), "{lines:?}");
        assert!(lines.len() > 1);
    }

    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut it = s.chars().peekable();
        while let Some(c) = it.next() {
            if c == '\x1b' {
                while let Some(n) = it.next() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn app() -> App {
        let mut cfg = Config::default();
        cfg.color = ColorMode::Never;
        App::new(
            cfg,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
            std::env::temp_dir(),
        )
    }

    fn colour_app() -> App {
        let mut cfg = Config::default();
        cfg.color = ColorMode::Always;
        App::new(
            cfg,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
            std::env::temp_dir(),
        )
    }

    #[test]
    fn esc_stops_a_running_turn() {
        let mut a = app();
        let flag = a.cancel.clone();
        let (tx, _rx) = channel();
        a.status.streaming = true;
        a.handle_key(Key::Esc, &tx);
        assert!(
            flag.load(Ordering::Relaxed),
            "Esc did not ask the running turn to stop"
        );
    }

    #[test]
    fn up_moves_between_rows_before_history() {
        let mut a = app();
        a.history = vec!["old".to_string()];
        a.input = "aa\nbbb".into();
        a.cursor = a.input.len();
        let (tx, _rx) = channel();
        a.handle_key(Key::Up, &tx);
        assert_eq!(a.input, "aa\nbbb", "Up left the composer");
        assert_eq!(a.cursor, 2, "Up did not land on the row above");
        assert!(a.hist_pos.is_none(), "Up recalled history from the last row");
    }

    #[test]
    fn a_long_prompt_keeps_its_answer_hint() {
        let mut a = app();
        let long = format!("run `{}`", "cat openspec/changes/thing.md; ".repeat(8));
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm { what: long, reply });
        let out = a.frame(80, 12);
        let row = out
            .lines()
            .find(|l| l.contains("y/N"))
            .unwrap_or_else(|| panic!("no answer hint in {out:?}"));
        assert!(row.contains('s'), "no seconds left: {row:?}");
        assert!(
            strip(row).chars().count() <= 80,
            "the prompt row is wider than the terminal"
        );
    }

    #[test]
    fn esc_refuses_a_prompt_without_quitting() {
        let mut a = app();
        let (reply, rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `ls`".into(),
            reply,
        });
        a.status.streaming = true;
        let (tx, _cmds) = channel();
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none(), "the prompt stayed open");
        assert_eq!(rx.try_recv().ok(), Some(false), "the prompt was not refused");
        assert!(!a.quit, "Esc quit the whole interface");
    }

    #[test]
    fn a_new_prompt_rings_once() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `ls`".into(),
            reply,
        });
        assert!(
            a.frame(80, 12).contains('\u{7}'),
            "no bell on a new prompt"
        );
        a.bell_pending = false;
        assert!(
            !a.frame(80, 12).contains('\u{7}'),
            "the same prompt rings twice"
        );
    }

    #[test]
    fn the_confirm_row_counts_down() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `rm -rf build`".into(),
            reply,
        });
        let first = countdown(&a.frame(80, 12));
        a.pending_since = Some(a.pending_since.expect("pending") - Duration::from_secs(10));
        let second = countdown(&a.frame(80, 12));
        assert!(first <= 60 && first >= 59, "deadline starts at {first}");
        assert!(
            second <= first - 9,
            "countdown did not fall: {first} then {second}"
        );
    }

    fn countdown(frame: &str) -> u64 {
        let at = frame.rfind('(').expect("countdown in the prompt row");
        let rest = &frame[at + 1..];
        rest[..rest.find('s').expect("seconds marker")]
            .parse()
            .expect("seconds")
    }

    #[test]
    fn an_unanswered_prompt_times_out() {
        let (_tx, rx) = channel::<bool>();
        assert_eq!(
            wait_for_answer(&rx, Duration::from_millis(50)),
            None,
            "a silent prompt did not time out"
        );
    }

    #[test]
    fn an_answered_prompt_returns_at_once() {
        let (tx, rx) = channel::<bool>();
        tx.send(true).expect("send answer");
        assert_eq!(wait_for_answer(&rx, Duration::from_secs(5)), Some(true));
    }

    #[test]
    fn a_stop_event_ends_the_turn() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `ls`".into(),
            reply,
        });
        a.status.streaming = true;
        a.apply(Ev::Stop("no answer within 60s".into()));
        assert!(a.cancel.load(Ordering::Relaxed), "the turn kept running");
        assert!(!a.status.streaming);
        assert!(a.pending.is_none(), "a stale prompt stayed on screen");
        assert!(a.pending_since.is_none());
        assert!(a.entries.last().expect("entry").text.contains("no answer"));
    }

    #[test]
    fn up_recalls_history_from_the_first_row() {
        let mut a = app();
        a.history = vec!["old".to_string()];
        a.input = "aa".into();
        a.cursor = 2;
        let (tx, _rx) = channel();
        a.handle_key(Key::Up, &tx);
        assert_eq!(a.input, "old");
    }

    #[test]
    fn down_moves_between_rows() {
        let mut a = app();
        a.input = "aa\nbbb".into();
        a.cursor = 0;
        let (tx, _rx) = channel();
        a.handle_key(Key::Down, &tx);
        assert_eq!(a.cursor, 3, "Down did not land on the row below");
        assert_eq!(a.input, "aa\nbbb");
    }

    #[test]
    fn frame_fits_the_terminal() {
        let mut a = app();
        a.push(Role::User, "a".repeat(200));
        a.push(Role::Assistant, "word ".repeat(100));
        let out = a.frame(40, 12);
        for line in out.lines() {
            let visible = strip(line);
            assert!(visible.chars().count() <= 40, "{visible:?}");
        }
    }

    #[test]
    fn frame_shows_status_and_prompt() {
        let mut a = app();
        let out = a.frame(100, 20);
        assert!(out.contains("deepseek-flash"));
        assert!(out.contains("> "));
    }

    #[test]
    fn frame_uses_crlf_for_every_row_boundary() {
        let mut a = app();
        a.push(Role::User, "hello");
        a.push(Role::Assistant, "hi");
        let out = a.frame(80, 10);
        assert!(!out.replace("\r\n", "").contains('\n'), "bare line feed in frame");
    }

    #[test]
    #[test]
    fn ctrl_o_expands_the_last_tool_call() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `ls -la`".into(),
            args: "{\"command\":\"ls -la\"}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: "exit 0".into(),
            output: "total 8\nfoo".into(),
            ms: 4,
        });
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('o'), &tx);
        let text = a.entries[0].text.clone();
        assert!(text.contains("args:"), "{text}");
        assert!(text.contains("ls -la"), "{text}");
        assert!(text.contains("total 8"), "{text}");
        a.handle_key(Key::Ctrl('o'), &tx);
        assert!(!a.entries[0].text.contains("args:"), "it did not collapse");
    }

    #[test]
    fn inline_code_is_bold_in_the_tool_colour() {
        let mut a = colour_app();
        a.push(Role::Assistant, "could not `bash` it");
        let out = a.frame(60, 12);
        assert!(out.contains("\x1b[1m"), "not bold: {out:?}");
        assert!(
            out.contains("38;5;108") || out.contains("38;2;127;200;160"),
            "not the tool colour: {out:?}"
        );
    }

    #[test]
    fn a_fenced_block_stays_dim() {
        let mut a = colour_app();
        a.push(Role::Assistant, "```\nfn x() {}\n```");
        let out = a.frame(60, 12);
        assert!(
            out.contains("38;5;244") || out.contains("38;2;128;128;128"),
            "fence not dim: {out:?}"
        );
        assert!(!out.contains("\x1b[1mfn"), "fence got bold: {out:?}");
    }

    #[test]
    fn assistant_rows_render_markdown() {
        let mut a = app();
        a.push(Role::Assistant, "**bold** and `code`\n\n- item one");
        let out = a.frame(60, 14);
        assert!(!out.contains("**"), "markers kept: {out:?}");
        assert!(!out.contains("`code`"), "backticks kept: {out:?}");
        assert!(out.contains("bold"), "{out:?}");
        assert!(out.contains("item one"), "{out:?}");
        assert!(out.contains('•'), "no bullet marker: {out:?}");
    }

    #[test]
    fn frame_clears_below_the_status_row_before_drawing_it() {
        let mut a = app();
        let out = a.frame(40, 10);
        let row = &out[out.rfind("\x1b[2K").expect("status row")..];
        let erase = row.find("\x1b[J").expect("nothing clears below the status");
        let text = row.find('·').expect("no status text");
        assert!(erase < text, "erase below runs after the status text: {row:?}");
    }
    fn composer_draws_one_row_per_line() {
        let mut a = app();
        a.input = "line one\nline two".into();
        a.cursor = a.input.len();
        let out = a.frame(60, 12);
        assert!(out.contains("> line one\r\n"), "{out:?}");
        assert!(out.contains("  line two"), "{out:?}");
        assert!(!out.contains('\u{0}'), "caret marker leaked into the frame");
    }

    #[test]
    fn composer_keeps_the_caret_row_visible() {
        let mut a = app();
        a.input = (1..=12)
            .map(|i| format!("row {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        a.cursor = a.input.len();
        let out = a.frame(60, 12);
        assert!(out.contains("  row 12"), "last row missing: {out:?}");
        assert!(!out.contains("row 1\r\n"), "composer ignored its row budget");
    }

    #[test]
    fn composer_rows_fit_the_terminal() {
        let mut a = app();
        a.input = format!("{}\n{}", "x".repeat(90), "y".repeat(9));
        a.cursor = a.input.len();
        let out = a.frame(40, 10);
        for line in out.lines() {
            let visible = strip(line);
            assert!(visible.chars().count() <= 40, "{visible:?}");
        }
        assert!(out.contains("…"), "a wide row is not marked as cut");
    }

    #[test]
    fn no_colour_emits_no_colour_escapes() {
        let mut a = app();
        a.push(Role::User, "hello");
        a.push(Role::Assistant, "hi");
        let out = a.frame(80, 10);
        assert!(!out.contains("38;"), "colour escaped in no-colour mode");
    }

    #[test]
    fn colour_roles_use_256_and_truecolor() {
        let plain = Palette { on: true, truecolor: false, theme: &crate::theme::THEMES[0] };
        assert!(plain.paint(Role::Error, "x").contains("38;5;"));
        let rgb = Palette { on: true, truecolor: true, theme: &crate::theme::THEMES[0] };
        assert!(rgb.paint(Role::Error, "x").contains("38;2;"));
        let off = Palette { on: false, truecolor: true, theme: &crate::theme::THEMES[0] };
        assert_eq!(off.paint(Role::Error, "x"), "x");
    }

    #[test]
    fn thinking_and_tool_lines_render() {
        let mut a = app();
        a.push(Role::Thinking, "considering options");
        a.push(Role::Tool, "bash  ls -la");
        let out = a.frame(80, 12);
        assert!(out.contains("~ considering"), "{out}");
        assert!(out.contains("bash  ls -la"), "{out}");
    }

    #[test]
    fn question_picker_renders_choices() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.pending = Some(Pending::Question {
            q: "which database?".into(),
            opts: vec!["sqlite".into(), "postgres".into()],
            reply: tx,
            mode: AskMode::Choose,
            typed: String::new(),
        });
        let out = a.frame(100, 12);
        assert!(out.contains("which database?"), "{out}");
        assert!(out.contains("[1] sqlite"), "{out}");
        assert!(out.contains("[e]"), "{out}");
    }

    #[test]
    fn yes_no_shortcuts_answer() {
        let mut a = app();
        let (tx, rx) = channel();
        a.pending = Some(Pending::Question {
            q: "delete?".into(),
            opts: vec!["Yes".into(), "No".into()],
            reply: tx,
            mode: AskMode::Choose,
            typed: String::new(),
        });
        let (cmd_tx, _cmd_rx) = channel();
        a.handle_key(Key::Char('y'), &cmd_tx);
        assert_eq!(rx.recv().unwrap().as_deref(), Some("Yes"));
    }

    #[test]
    fn status_reports_tokens_and_cost() {
        let mut a = app();
        a.status.tokens = 3400;
        a.status.hit = 900;
        a.status.miss = 100;
        a.status.output = 50;
        let line = a.status_line(120);
        assert!(line.contains("3.4K tok"), "{line}");
        let cost = crate::pricing::cost(
            "deepseek-flash",
            crate::pricing::now(),
            900,
            100,
            50,
        )
        .unwrap();
        assert!(line.contains(&format!("${cost:.2}")), "{line}");
        assert!(!line.contains("cache"), "{line}");
        assert!(!line.contains("normal"), "{line}");
    }

    #[test]
    fn status_abbreviates_large_token_counts() {
        let mut a = app();
        a.status.tokens = 940;
        assert!(a.status_line(120).contains("940 tok"));
        a.status.tokens = 1_200_000;
        assert!(a.status_line(120).contains("1.2M tok"));
        assert_eq!(compact_tokens(12_345), "12.3K");
        assert_eq!(compact_tokens(1_000), "1K");
    }

    #[test]
    fn status_sums_cost_across_turns() {
        let mut a = app();
        a.apply(Ev::Usage {
            tokens: 100,
            hit: 900,
            miss: 100,
            output: 50,
            model: None,
        });
        a.apply(Ev::Usage {
            tokens: 200,
            hit: 400,
            miss: 200,
            output: 25,
            model: None,
        });
        assert_eq!(a.status.hit, 1300);
        assert_eq!(a.status.miss, 300);
        assert_eq!(a.status.output, 75);
        let total = crate::pricing::cost(
            "deepseek-flash",
            crate::pricing::now(),
            1300,
            300,
            75,
        )
        .unwrap();
        let line = a.status_line(120);
        assert!(line.contains(&format!("${total:.2}")), "{line}");
    }

    #[test]
    fn status_shows_the_served_model_when_it_differs() {
        let mut a = app();
        a.status.served = "deepseek-flash-2026-11".into();
        assert!(
            a.status_line(120)
                .contains("deepseek-flash (deepseek-flash-2026-11)"),
            "{}",
            a.status_line(120)
        );
        a.status.served = "deepseek-flash".into();
        assert!(!a.status_line(120).contains('('), "{}", a.status_line(120));
    }

    #[test]
    fn an_idle_app_does_not_render() {
        let mut a = app();
        a.dirty = false;
        assert!(!a.wants_render(false), "an idle screen queued a frame");
        assert!(a.wants_render(true), "a key did not queue a frame");
        a.dirty = true;
        assert!(a.wants_render(false), "a dirty screen did not queue a frame");
    }

    #[test]
    fn the_spinner_runs_at_ten_frames_a_second() {
        let mut a = app();
        a.status.streaming = true;
        a.last_redraw = Instant::now();
        assert!(!a.spinner_due(), "spinner ran faster than 100 ms");
        a.last_redraw = Instant::now() - Duration::from_millis(101);
        assert!(a.spinner_due(), "spinner stalled after 100 ms");
        a.status.streaming = false;
        assert!(!a.spinner_due(), "idle spinner still ticking");
    }

    #[test]
    fn a_resize_is_noticed_without_a_timer() {
        let mut a = app();
        a.last_size = (80, 24);
        assert!(!a.resized(80, 24));
        assert!(a.resized(81, 24));
        assert!(a.resized(80, 25));
    }

    #[test]
    fn cwd_commands_report_and_change_the_root() {
        let mut a = app();
        let (tx, rx) = channel();
        assert_eq!(a.root, std::env::temp_dir(), "constructor root ignored");

        a.run_command("cwd", &tx);
        assert_eq!(
            a.entries.last().unwrap().text,
            a.root.display().to_string()
        );

        a.run_command("cd", &tx);
        assert_eq!(a.entries.last().unwrap().text, "cd <path>");

        a.run_command("cd .", &tx);
        match rx.try_recv() {
            Ok(Cmd::SetRoot(path)) => assert!(path.is_dir()),
            _ => panic!("cd did not send SetRoot"),
        }

        let dir = std::env::temp_dir();
        a.apply(Ev::Root(dir.clone()));
        assert_eq!(a.root, dir);
        a.run_local("pwd");
        let shown = dir.canonicalize().unwrap().display().to_string();
        assert!(
            a.entries.last().unwrap().text.contains(&shown),
            "typed shell did not follow the root: {:?}",
            a.entries.last().unwrap().text
        );

        a.run_command("cd /nope/imp-missing", &tx);
        assert_eq!(a.entries.last().unwrap().role, Role::Error);
    }

    #[test]
    fn agent_command_switches_by_name_and_reports_current() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("agent reviewer", &tx);
        match rx.try_recv() {
            Ok(Cmd::SetAgent(name)) => assert_eq!(name, "reviewer"),
            _ => panic!("no direct switch"),
        }
        a.run_command("agent current", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::ShowAgent)));
    }

    #[test]
    fn fork_command_sends_the_fork() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("fork", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Fork)));
    }

    #[test]
    fn verbosity_command_controls_tool_detail() {
        let mut a = app();
        let (tx, _cmds) = channel();
        assert!(!a.verbose, "default is not low");
        a.run_command("verbosity high", &tx);
        assert!(a.verbose);

        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `ls`".into(),
            args: "{\"command\":\"ls\"}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: "total 8".into(),
            output: "total 8\nfoo".into(),
            ms: 4,
        });
        let text = &a.entries[1].text;
        assert!(text.contains("args:"), "{text:?}");
        assert!(text.contains("foo"), "{text:?}");

        a.run_command("verbosity low", &tx);
        assert!(!a.verbose);
        a.run_command("verbosity", &tx);
        assert!(a.entries.last().unwrap().text.contains("verbosity: low"));
        a.run_command("verbosity loud", &tx);
        assert!(a.entries.last().unwrap().text.contains("use low or high"));
    }

    #[test]
    fn a_failed_call_puts_its_preview_on_its_own_line() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `openspec list`".into(),
            args: "{\"command\":\"openspec list\"}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: false,
            preview: "Changes: render-on-change".into(),
            output: "Changes: render-on-change".into(),
            ms: 7,
        });
        let text = &a.entries[0].text;
        assert!(text.contains("\nChanges: render-on-change"), "{text:?}");
        assert!(!text.contains(" · Changes:"), "{text:?}");
        let out = a.frame(60, 12);
        assert!(out.contains("  Changes: render-on-change"), "{out:?}");
    }

    #[test]
    fn tool_marks_are_bold() {
        let mut a = colour_app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `true`".into(),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: String::new(),
            output: String::new(),
            ms: 1,
        });
        let out = a.frame(80, 12);
        assert!(out.contains("\x1b[1m✓"), "mark not bold: {out:?}");
        assert!(out.contains("\x1b[22m"), "bold not closed: {out:?}");

        a.palette = Palette {
            on: false,
            truecolor: false, theme: &crate::theme::THEMES[0] };
        let plain = a.frame(80, 12);
        assert!(!plain.contains("\x1b[1m"), "bold with colour off: {plain:?}");
    }

    #[test]
    fn a_wrapped_tool_row_keeps_its_indent() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: format!("run `echo {}`", "a".repeat(60)),
            args: "{}".into(),
        });
        let out = a.frame(30, 10);
        let mut seen = 0;
        for line in out.split("\r\n") {
            if line.contains("aaaa") {
                let body = line.trim_start_matches("\x1b[2K");
                assert!(body.starts_with("  "), "lost indent: {body:?}");
                seen += 1;
            }
        }
        assert!(seen > 1, "summary did not wrap: {out:?}");
    }

    #[test]
    fn failed_tools_render_in_the_error_role() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `false`".into(),
            args: "{\"command\":\"false\"}".into(),
        });
        assert_eq!(a.entries.len(), 1, "a call should start as one row");
        assert!(a.entries[0].text.contains("⠹"), "no spinner while running");
        a.apply(Ev::ToolEnd {
            ok: false,
            preview: "exit 1".into(),
            output: "exit 1".into(),
            ms: 7,
        });
        assert_eq!(a.entries.len(), 1, "the finish added a second row");
        assert_eq!(a.entries[0].role, Role::Error);
        assert!(a.entries[0].text.contains("✗"), "{}", a.entries[0].text);
        assert!(a.entries[0].text.contains("run `false`"));
        assert!(a.entries[0].text.contains("ms"));
    }

    fn picker() -> App {
        let mut a = app();
        a.pending = Some(Pending::Choose {
            title: "session".into(),
            options: vec!["abc123  3m  old title  4 msgs".into()],
            index: 0,
            action: ChooseKind::Session,
        });
        a
    }

    #[test]
    fn picker_new_session_key() {
        let mut a = picker();
        let (tx, rx) = channel();
        a.handle_key(Key::Char('n'), &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::NewSession)));
    }

    #[test]
    fn picker_delete_needs_confirmation() {
        let mut a = picker();
        let (tx, rx) = channel();
        a.handle_key(Key::Char('d'), &tx);
        assert!(matches!(a.pending, Some(Pending::ConfirmDelete { .. })));
        a.handle_key(Key::Char('y'), &tx);
        match rx.try_recv() {
            Ok(Cmd::Delete(id)) => assert_eq!(id, "abc123"),
            _ => panic!("expected delete"),
        }
    }

    #[test]
    fn picker_rename_edits_a_title() {
        let mut a = picker();
        let (tx, rx) = channel();
        a.handle_key(Key::Char('r'), &tx);
        for c in "new name".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
        match rx.try_recv() {
            Ok(Cmd::Rename(id, title)) => {
                assert_eq!(id, "abc123");
                assert_eq!(title, "new name");
            }
            _ => panic!("expected rename"),
        }
    }

    #[test]
    fn choosing_a_session_sends_resume() {
        let mut a = app();
        a.pending = Some(Pending::Choose {
            title: "session".into(),
            options: vec!["abc123  title  3 msgs".into()],
            index: 0,
            action: ChooseKind::Session,
        });
        let (tx, rx) = channel();
        a.handle_key(Key::Enter, &tx);
        match rx.try_recv() {
            Ok(Cmd::Resume(id)) => assert_eq!(id, "abc123"),
            _ => panic!("expected a resume command"),
        }
    }

    fn long_app() -> App {
        let mut a = app();
        for i in 0..40 {
            a.push(Role::User, format!("line {i}"));
        }
        a
    }

    #[test]
    fn scroll_detaches_from_the_tail() {
        let mut a = long_app();
        let (tx, _cmds) = channel();
        let tail = a.frame(60, 8);
        a.handle_key(Key::PageUp, &tx);
        let earlier = a.frame(60, 8);
        assert_ne!(tail, earlier, "PgUp drew the same rows");
        assert!(!a.follow, "PgUp did not detach");
        assert!(
            a.status_line(120).contains("↑"),
            "no unseen count: {}",
            a.status_line(120)
        );
    }

    #[test]
    fn a_detached_view_holds_when_output_arrives() {
        let mut a = long_app();
        let (tx, _cmds) = channel();
        a.frame(60, 8);
        a.handle_key(Key::PageUp, &tx);
        let before = a.frame(60, 8);
        a.push(Role::User, "brand new line");
        let after = a.frame(60, 8);
        assert!(!after.contains("brand new line"), "view jumped to the tail");
        assert!(before.contains("line 29"), "{before:?}");
        assert!(after.contains("line 29"), "rows moved: {after:?}");
        assert!(a.unseen > 5, "unseen count did not grow: {}", a.unseen);
    }

    #[test]
    fn pgdn_at_the_newest_row_resumes_following() {
        let mut a = long_app();
        let (tx, _cmds) = channel();
        a.frame(60, 8);
        a.handle_key(Key::PageUp, &tx);
        a.frame(60, 8);
        a.handle_key(Key::PageDown, &tx);
        let out = a.frame(60, 8);
        assert!(a.follow, "PgDn did not resume following");
        assert!(out.contains("line 39"), "newest row missing: {out:?}");
        assert!(!a.status_line(120).contains("↑"));
    }

    #[test]
    fn typing_resumes_following() {
        let mut a = long_app();
        let (tx, _cmds) = channel();
        a.frame(60, 8);
        a.handle_key(Key::PageUp, &tx);
        a.handle_key(Key::Char('x'), &tx);
        let out = a.frame(60, 8);
        assert!(a.follow, "editing did not resume following");
        assert!(out.contains("line 39"), "{out:?}");
    }

    #[test]
    fn decodes_page_keys() {
        let mut buf = b"\x1b[5~\x1b[6~".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 2);
        assert!(matches!(keys[0], Key::PageUp));
        assert!(matches!(keys[1], Key::PageDown));
        assert!(buf.is_empty(), "page sequences leaked as text");
    }

    #[test]
    fn decodes_control_keys() {
        let mut buf = vec![0x03, 0x0c, b'\t', 0x1b, b'[', b'H', 0x1b, b'[', b'F'];
        let keys = decode(&mut buf);
        assert!(matches!(keys[0], Key::Ctrl('c')));
        assert!(matches!(keys[1], Key::Ctrl('l')));
        assert!(matches!(keys[2], Key::Tab));
        assert!(matches!(keys[3], Key::Home));
        assert!(matches!(keys[4], Key::End));
    }

    #[test]
    fn fit_truncates_to_width() {
        assert_eq!(fit("abcdef", 10), "abcdef");
        let short = fit("abcdefghij", 5);
        assert_eq!(short.chars().count(), 5);
        assert!(short.ends_with('…'));
    }

    #[test]
    fn wrap_keeps_blank_lines() {
        assert_eq!(wrap("a\n\nb", 20).len(), 3);
    }
    #[test]
    fn a_shell_draft_shows_a_dollar_prompt() {
        let mut a = app();
        a.input = "!ls".into();
        a.cursor = 3;
        let out = a.frame(60, 12);
        assert!(out.contains("$ ls"), "no shell prompt: {out:?}");
        assert!(out.contains("shell · local"), "no shell status: {out:?}");
    }

    #[test]
    fn a_normal_draft_keeps_the_arrow() {
        let mut a = app();
        a.input = "hello".into();
        a.cursor = 5;
        let out = a.frame(60, 12);
        assert!(out.contains("> hello"), "{out:?}");
        assert!(out.contains("deepseek-flash"), "{out:?}");
    }

    #[test]
    fn ctrl_c_exits_while_streaming() {
        let mut a = app();
        a.status.streaming = true;
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(a.quit, "Ctrl+C did not exit mid-turn");
    }

    fn ctrl_c_exits_at_once_when_idle() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(a.quit, "an idle Ctrl+C did not quit");
    }

    #[test]
    fn a_confirm_asks_with_the_verb_alone() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `nmap -sV -T4 192.168.0.56`".into(),
            args: "{}".into(),
        });
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `nmap -sV -T4 192.168.0.56`".into(),
            reply,
        });
        let out = a.frame(90, 12);
        assert!(out.contains("run it? y/N"), "no verb prompt: {out:?}");
        assert_eq!(
            out.matches("nmap -sV -T4 192.168.0.56").count(),
            1,
            "the command appears more than once: {out:?}"
        );
    }

    #[test]
    fn an_unknown_verb_falls_back_to_approve() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "overwrite /etc/hosts".into(),
            reply,
        });
        assert!(a.frame(80, 12).contains("approve? y/N"));
    }

    #[test]
    fn ctrl_c_exits_from_a_picker() {
        let mut a = app();
        a.pending = Some(Pending::Choose {
            title: "session".into(),
            options: vec!["one".into()],
            index: 0,
            action: ChooseKind::Session,
        });
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(a.quit, "Ctrl+C did not exit from a picker");
    }

    #[test]
    fn ctrl_c_exits_from_a_question() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.pending = Some(Pending::Question {
            q: "which?".into(),
            opts: vec!["a".into(), "b".into()],
            reply,
            mode: AskMode::Choose,
            typed: String::new(),
        });
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(a.quit, "Ctrl+C did not exit from a question");
    }

    #[test]
    fn ctrl_c_exits_on_a_confirmation() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `ls`".into(),
            reply,
        });
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(a.quit, "Ctrl+C did not exit on a confirmation");
    }

    #[test]
    fn ctrl_letters_survive_the_keyboard_protocol() {
        let mut buf = b"\x1b[99;5u\x1b[108;5u\x1b[100;5u".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 3, "{keys:?}");
        assert!(matches!(keys[0], Key::Ctrl('c')), "{:?}", keys[0]);
        assert!(matches!(keys[1], Key::Ctrl('l')), "{:?}", keys[1]);
        assert!(matches!(keys[2], Key::Ctrl('d')), "{:?}", keys[2]);
        assert!(buf.is_empty());
    }

    #[test]
    fn ctrl_shift_letter_still_maps_to_ctrl() {
        let mut buf = b"\x1b[67;6u".to_vec();
        let keys = decode(&mut buf);
        assert!(matches!(keys[0], Key::Ctrl('c')), "{:?}", keys[0]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn termios_matches_the_glibc_layout() {
        assert_eq!(std::mem::size_of::<Termios>(), 60);
        assert_eq!(std::mem::align_of::<Termios>(), 4);
    }

}
