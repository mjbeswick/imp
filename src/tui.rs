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
use std::sync::mpsc::{channel, Sender, TryRecvError};
use std::time::{Duration, Instant};

const ALT_ON: &str = "\x1b[?1049h\x1b[?25l";
const ALT_OFF: &str = "\x1b[?25h\x1b[?1049l";

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

#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    iflag: u64,
    oflag: u64,
    cflag: u64,
    lflag: u64,
    cc: [u8; NUM_CC],
    ispeed: u64,
    ospeed: u64,
}

#[cfg(target_os = "macos")]
const NUM_CC: usize = 20;
#[cfg(not(target_os = "macos"))]
const NUM_CC: usize = 32;

/// The flags `stty raw -echo` clears, by platform.
mod flags {
    #[cfg(target_os = "macos")]
    pub const IFLAGS: u64 = 0x1 | 0x2 | 0x8 | 0x20 | 0x40 | 0x80 | 0x100 | 0x200;
    #[cfg(not(target_os = "macos"))]
    pub const IFLAGS: u64 = 0x1 | 0x2 | 0x8 | 0x20 | 0x40 | 0x80 | 0x100 | 0x400;

    #[cfg(target_os = "macos")]
    pub const LFLAGS: u64 = 0x8 | 0x10 | 0x100 | 0x80 | 0x400;
    #[cfg(not(target_os = "macos"))]
    pub const LFLAGS: u64 = 0x8 | 0x40 | 0x2 | 0x1 | 0x8000;

    #[cfg(target_os = "macos")]
    pub const CSIZE: u64 = 0x300;
    #[cfg(not(target_os = "macos"))]
    pub const CSIZE: u64 = 0x30;

    #[cfg(target_os = "macos")]
    pub const CS8: u64 = 0x300;
    #[cfg(not(target_os = "macos"))]
    pub const CS8: u64 = 0x30;

    #[cfg(target_os = "macos")]
    pub const PARENB: u64 = 0x1000;
    #[cfg(not(target_os = "macos"))]
    pub const PARENB: u64 = 0x100;

    #[cfg(target_os = "macos")]
    pub const VMIN: usize = 16;
    #[cfg(not(target_os = "macos"))]
    pub const VMIN: usize = 6;

    #[cfg(target_os = "macos")]
    pub const VTIME: usize = 17;
    #[cfg(not(target_os = "macos"))]
    pub const VTIME: usize = 5;
}

const OPOST: u64 = 0x1;
const TCSANOW: c_int = 0;

static SAVED: std::sync::Mutex<Option<Termios>> = std::sync::Mutex::new(None);
const RESTORE: &str = "\x1b[?25h\x1b[?1049l";

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
        let mut saved = Termios {
            iflag: 0,
            oflag: 0,
            cflag: 0,
            lflag: 0,
            cc: [0; NUM_CC],
            ispeed: 0,
            ospeed: 0,
        };
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

const ICRNL: u64 = 0x100;
const IGNCR: u64 = 0x80;

impl LineMode {
    pub fn enter() -> Self {
        let mut saved = Termios {
            iflag: 0,
            oflag: 0,
            cflag: 0,
            lflag: 0,
            cc: [0; NUM_CC],
            ispeed: 0,
            ospeed: 0,
        };
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

struct Palette {
    on: bool,
    truecolor: bool,
}

impl Palette {
    fn detect(cfg: &Config) -> Self {
        let tty = unsafe { isatty(1) } != 0;
        let on = match cfg.color {
            ColorMode::Never => false,
            ColorMode::Always => true,
            ColorMode::Auto => tty && std::env::var_os("NO_COLOR").is_none(),
        };
        let truecolor = std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false);
        Self { on, truecolor }
    }

    fn paint(&self, role: Role, text: &str) -> String {
        if !self.on {
            return text.to_string();
        }
        let (r, g, b, fallback) = match role {
            Role::User => (0x8f, 0xbf, 0xff, 111),
            Role::Assistant => (0xe6, 0xe6, 0xe6, 255),
            Role::Thinking => (0x80, 0x80, 0x80, 244),
            Role::Tool => (0x7f, 0xc8, 0xa0, 108),
            Role::Error => (0xff, 0x6b, 0x6b, 203),
            Role::Info => (0xd7, 0xa6, 0x5a, 214),
        };
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
    let shift = modifiers.saturating_sub(1) & 1 == 1;
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
    SetThinking(bool),
    SetAgent(String),
    Compact,
    Resume(String),
    Reload,
    Rename(String, String),
    Delete(String),
    NewSession,
    Reset,
    Quit,
}

pub enum Ev {
    Token(String),
    Think(String),
    ToolStart { name: String, args: String },
    ToolEnd { ok: bool, preview: String },
    Info(String),
    Error(String),
    Usage {
        tokens: u64,
        hit: u64,
        miss: u64,
        output: u64,
    },
    Confirm { what: String, reply: Sender<bool> },
    Question { q: String, opts: Vec<String>, reply: Sender<Option<String>> },
    Done(Option<String>),
}

struct WorkerHooks {
    tx: Sender<Ev>,
}

impl Hooks for WorkerHooks {
    fn ui(&mut self, event: Ui) {
        let ev = match event {
            Ui::Content(t) => Ev::Token(t),
            Ui::Reasoning(t) => Ev::Think(t),
            Ui::ToolStart { name, args } => Ev::ToolStart { name, args },
            Ui::ToolEnd { ok, preview } => Ev::ToolEnd { ok, preview },
            Ui::Info(t) => Ev::Info(t),
            Ui::Error(t) => Ev::Error(t),
        };
        let _ = self.tx.send(ev);
    }

    fn confirm(&mut self, what: &str) -> bool {
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
        rx.recv_timeout(Duration::from_secs(300)).unwrap_or(false)
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

struct Entry {
    role: Role,
    text: String,
}

pub struct App {
    cfg: Config,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    palette: Palette,
    entries: Vec<Entry>,
    input: String,
    cursor: usize,
    history: Vec<String>,
    hist_pos: Option<usize>,
    scroll: usize,
    menu_open: bool,
    menu_index: usize,
    pending: Option<Pending>,
    status: Status,
    agents: Vec<String>,
    sessions_dir: std::path::PathBuf,
    last_redraw: Instant,
    dirty: bool,
    quit: bool,
    spinner: usize,
    tool_started: Option<Instant>,
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
    streaming: bool,
}

const COMMANDS: &[(&str, &str)] = crate::help::COMMANDS;

impl App {
    pub fn new(cfg: Config, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        let palette = Palette::detect(&cfg);
        let history = load_history();
        let root = std::env::current_dir().unwrap_or_default();
        let agents = crate::store::load_agents(&cfg, &root)
            .into_iter()
            .map(|a| a.name)
            .collect();
        let sessions_dir = crate::config::dirs_state().join("imp").join("sessions");
        Self {
            cfg,
            cancel,
            palette,
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            history,
            hist_pos: None,
            scroll: 0,
            menu_open: false,
            menu_index: 0,
            pending: None,
            status: Status {
                tokens: 0,
                hit: 0,
                miss: 0,
                output: 0,
                streaming: false,
            },
            agents,
            sessions_dir,
            last_redraw: Instant::now() - Duration::from_secs(1),
            dirty: true,
            quit: false,
            spinner: 0,
            tool_started: None,
        }
    }

    fn push(&mut self, role: Role, text: impl Into<String>) {
        self.entries.push(Entry {
            role,
            text: text.into(),
        });
        self.scroll = 0;
        self.dirty = true;
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
            Ev::ToolStart { name, args } => {
                self.tool_started = Some(Instant::now());
                self.push(Role::Tool, format!("{name}  {args}"))
            }
            Ev::ToolEnd { ok, preview } => {
                let ms = self
                    .tool_started
                    .take()
                    .map(|t| t.elapsed().as_millis())
                    .unwrap_or(0);
                let role = if ok { Role::Tool } else { Role::Error };
                self.push(role, format!("  {} {preview}  · {ms} ms", if ok { "✓" } else { "✗" }))
            }
            Ev::Info(t) => self.push(Role::Info, t),
            Ev::Error(t) => self.push(Role::Error, t),
            Ev::Usage {
                tokens,
                hit,
                miss,
                output,
            } => {
                self.status.tokens = tokens;
                self.status.hit = hit;
                self.status.miss = miss;
                self.status.output = output;
                self.dirty = true;
            }
            Ev::Confirm { what, reply } => {
                self.pending = Some(Pending::Confirm { what, reply });
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
        if let Some(pending) = self.pending.take() {
            self.answer(pending, key, tx);
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
            Key::Ctrl('c') => {
                if self.status.streaming {
                    self.cancel.store(true, Ordering::Relaxed);
                } else {
                    self.quit = true;
                }
            }
            Key::Ctrl('d') => self.quit = true,
            Key::Ctrl('l') => {
                self.entries.clear();
                self.dirty = true;
            }
            Key::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                self.sync_menu();
            }
            Key::Backspace => {
                if self.cursor > 0 {
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
            Key::Up => {
                if !self.history.is_empty() {
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
                if let Some(p) = self.hist_pos {
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
                self.input.insert(self.cursor, '\n');
                self.cursor += 1;
                self.dirty = true;
            }
            _ => {}
        }
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
        self.scroll = 0;

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
            std::env::current_dir().unwrap_or_default(),
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
            "agent" => {
                let options = self.agents.clone();
                self.pending = Some(Pending::Choose {
                    title: "agent".into(),
                    options,
                    index: 0,
                    action: ChooseKind::Agent,
                });
                self.dirty = true;
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
                    self.push(Role::Info, format!("approved: {what}"));
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

    pub fn render(&mut self) {
        let (cols, rows) = size();
        if rows < 5 || cols < 20 {
            return;
        }
        let screen = self.frame(cols, rows);
        let mut out = std::io::stdout();
        out.write_all(screen.as_bytes()).ok();
        out.flush().ok();
        self.dirty = false;
        self.last_redraw = Instant::now();
    }

    /// One frame as bytes. No terminal access, so it can be tested.
    pub fn frame(&self, cols: usize, rows: usize) -> String {
        let menu = if self.menu_open { self.menu_matches() } else { Vec::new() };
        let menu_rows = menu.len().min(6);
        let body_rows = rows.saturating_sub(2 + menu_rows).max(1);

        let mut lines: Vec<(Role, String)> = Vec::new();
        for entry in &self.entries {
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
                lines.push((entry.role, format!("{prefix}{part}")));
            }
        }

        let total = lines.len();
        let scroll = self.scroll.min(total.saturating_sub(1));
        let end = total.saturating_sub(scroll);
        let start = end.saturating_sub(body_rows);
        let visible = &lines[start..end];

        let mut screen = String::from("\x1b[?7l\x1b[H");
        for _ in 0..body_rows.saturating_sub(visible.len()) {
            screen.push_str("\x1b[2K\r\n");
        }
        for (role, text) in visible.iter().take(body_rows) {
            screen.push_str("\x1b[2K");
            screen.push_str(&self.palette.paint(*role, &fit(text, cols)));
            screen.push_str("\r\n");
        }

        screen.push_str("\x1b[2K");
        screen.push_str(&self.input_line(cols));
        screen.push_str("\r\n");

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

        screen.push_str("\x1b[2K");
        screen.push_str(&self.palette.paint(Role::Thinking, &self.status_line(cols)));
        screen.push_str("\x1b[J\x1b[?7h");

        screen
    }

    fn input_line(&self, cols: usize) -> String {
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
                Pending::Confirm { what, .. } => format!("{what}? y/N"),
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
            return self.palette.paint(Role::Info, &fit(&text, cols));
        }

        let cursor = self.cursor.min(self.input.len());
        let before = &self.input[..cursor];
        let after = &self.input[cursor..];
        let mut body = format!("{before}\x1b[7m \x1b[0m{after}");
        // Keep the tail visible when the line is longer than the screen.
        let visible: String = body.chars().rev().take(cols.saturating_sub(2)).collect::<Vec<_>>().into_iter().rev().collect();
        if visible.len() < body.len() {
            body = format!("…{}", visible.chars().skip(1).collect::<String>());
        }
        self.palette.paint(Role::User, "> ") + &body
    }

    fn status_line(&self, cols: usize) -> String {
        let hit_rate = if self.status.hit + self.status.miss > 0 {
            format!(
                "{}%",
                100 * self.status.hit / (self.status.hit + self.status.miss)
            )
        } else {
            "—".into()
        };
        let spin = if self.status.streaming {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
            frames[self.spinner % frames.len()].to_string()
        } else {
            "·".into()
        };
        let thinking = if self.cfg.thinking { " · think" } else { "" };
        let money = crate::pricing::cost(
            &self.cfg.model,
            crate::pricing::now(),
            self.status.hit,
            self.status.miss,
            self.status.output,
        )
        .map(|c| {
            let micros = (c * 1_000_000.0).round() as u64;
            format!(" · ${}.{:04}", micros / 1_000_000, micros % 1_000_000)
        })
        .unwrap_or_default();
        fit(
            &format!(
                " {spin} {} · {} tok · cache {hit_rate}{money} · {}{thinking}",
                self.cfg.model,
                self.status.tokens,
                self.cfg.permissions.as_str()
            ),
            cols,
        )
    }
}

fn fit(text: &str, cols: usize) -> String {
    if text.chars().count() <= cols {
        return text.to_string();
    }
    let mut out: String = text.chars().take(cols.saturating_sub(1)).collect();
    out.push('…');
    out
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

/// Run the interface until the user quits.
pub fn run(session: Session, cfg: Config) -> Result<(), String> {
    install_guards();
    let _raw = RawMode::enter();
    if std::env::var_os("IMP_FORCE_PANIC").is_some() {
        panic!("forced panic for the terminal-guard test");
    }
    let mut app = App::new(cfg, session.cancel.clone());

    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (ev_tx, ev_rx) = channel::<Ev>();

    let sessions_dir = crate::config::dirs_state().join("imp").join("sessions");
    let mut worker_session = session;
    std::thread::spawn({
        let ev_tx = ev_tx.clone();
        move || loop {
            match cmd_rx.recv() {
                Ok(Cmd::Prompt(text)) => {
                    let mut hooks = WorkerHooks {
                        tx: ev_tx.clone(),
                                };
                    let result = worker_session.turn(&text, &mut hooks);
                    let _ = ev_tx.send(Ev::Usage {
                        tokens: crate::api::estimate_tokens(&worker_session.history),
                        hit: worker_session.last_usage.0,
                        miss: worker_session.last_usage.1,
                        output: worker_session.last_usage.2,
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
                                        };
                        let applied = worker_session.switch_agent(agent);
                        let _ = ev_tx.send(Ev::Info(format!("agent: {applied}")));
                        let _ = &mut hooks;
                    } else {
                        let _ = ev_tx.send(Ev::Error(format!("no agent {name}")));
                    }
                }
                Ok(Cmd::Resume(id)) => {
                    match worker_session.resume_id(&sessions_dir, &id) {
                        Ok(()) => {
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
                if app.status.streaming {
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

        if redraw || app.dirty || app.last_redraw.elapsed() > Duration::from_millis(120) {
            app.render();
        }
    }

    let _ = cmd_tx.send(Cmd::Quit);
    Ok(())
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
        App::new(cfg, std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)))
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
        let a = app();
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
    fn no_colour_emits_no_colour_escapes() {
        let mut a = app();
        a.push(Role::User, "hello");
        a.push(Role::Assistant, "hi");
        let out = a.frame(80, 10);
        assert!(!out.contains("38;"), "colour escaped in no-colour mode");
    }

    #[test]
    fn colour_roles_use_256_and_truecolor() {
        let plain = Palette { on: true, truecolor: false };
        assert!(plain.paint(Role::Error, "x").contains("38;5;"));
        let rgb = Palette { on: true, truecolor: true };
        assert!(rgb.paint(Role::Error, "x").contains("38;2;"));
        let off = Palette { on: false, truecolor: true };
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
    fn status_reports_tokens_cache_and_cost() {
        let mut a = app();
        a.status.tokens = 1234;
        a.status.hit = 900;
        a.status.miss = 100;
        let line = a.status_line(120);
        assert!(line.contains("1234 tok"), "{line}");
        assert!(line.contains("cache 90%"), "{line}");
        assert!(line.contains('$'), "{line}");
    }

    #[test]
    fn failed_tools_render_in_the_error_role() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            args: "false".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: false,
            preview: "exit 1".into(),
        });
        assert_eq!(a.entries.last().unwrap().role, Role::Error);
        assert!(a.entries.last().unwrap().text.contains("ms"));
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

    #[test]
    fn scroll_detaches_from_the_tail() {
        let mut a = app();
        for i in 0..40 {
            a.push(Role::User, format!("line {i}"));
        }
        let tail = a.frame(60, 8);
        a.scroll = 10;
        let earlier = a.frame(60, 8);
        assert_ne!(tail, earlier);
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
}
