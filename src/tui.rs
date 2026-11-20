//! Terminal interface: raw ANSI, colours by role, one chat column.
//!
//! The worker thread owns the session; the UI thread owns the screen.
//! They talk over two channels, and confirmations block the worker until
//! the user answers.

use crate::agent::{Answer, Hooks, Session, Ui};
use crate::config::{ColorMode, Config};
use std::ffi::{c_int, c_ulong};
use std::io::{Read, Write};
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

const ALT_ON: &str = "\x1b[?1049h\x1b[?25l\x1b[>1u";
const ALT_OFF: &str = "\x1b[?1006l\x1b[?1000l\x1b[<u\x1b[?25h\x1b[?1049l";
/// SGR mouse reports, so the wheel reaches imp instead of the scrollback.
const MOUSE_ON: &str = "\x1b[?1000h\x1b[?1006h";
const MOUSE_OFF: &str = "\x1b[?1006l\x1b[?1000l";
/// Set from the `mouse` setting before raw mode starts.
static MOUSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What raw mode writes on entry: the alternate screen, then the mouse if on.
fn enter_sequence(mouse: bool) -> String {
    if mouse {
        format!("{ALT_ON}{MOUSE_ON}")
    } else {
        ALT_ON.to_string()
    }
}

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
const RESTORE: &str = "\x1b[?1006l\x1b[?1000l\x1b[<u\x1b[?25h\x1b[?1049l";

extern "C" {
    fn write(fd: std::ffi::c_int, buf: *const u8, len: usize) -> isize;
    fn _exit(code: std::ffi::c_int) -> !;
    fn signal(sig: std::ffi::c_int, handler: usize) -> usize;
    fn tcgetattr(fd: c_int, termios: *mut Termios) -> c_int;
    fn tcsetattr(fd: c_int, action: c_int, termios: *const Termios) -> c_int;
    fn read(fd: c_int, buf: *mut u8, len: usize) -> isize;
    fn pipe(fds: *mut c_int) -> c_int;
}

/// Ask the terminal for its background with OSC 11, waiting at most about
/// 100 ms. Runs in raw mode, before the input thread starts.
fn query_light() -> Option<bool> {
    let mut saved = std::mem::MaybeUninit::<Termios>::uninit();
    if unsafe { tcgetattr(0, saved.as_mut_ptr()) } != 0 {
        return None;
    }
    let saved = unsafe { saved.assume_init() };
    let mut timed = saved;
    timed.cc[flags::VMIN] = 0;
    timed.cc[flags::VTIME] = 1;
    unsafe { tcsetattr(0, TCSANOW, &timed) };
    let ask = b"\x1b]11;?\x07";
    unsafe { write(1, ask.as_ptr(), ask.len()) };
    let start = Instant::now();
    let mut reply = Vec::new();
    let mut buf = [0u8; 64];
    while start.elapsed() < Duration::from_millis(100) {
        let n = unsafe { read(0, buf.as_mut_ptr(), buf.len()) };
        if n <= 0 {
            break;
        }
        reply.extend_from_slice(&buf[..n as usize]);
        if reply.ends_with(b"\x07") || reply.ends_with(b"\x1b\\") {
            break;
        }
    }
    unsafe { tcsetattr(0, TCSANOW, &saved) };
    crate::theme::light_from_osc(&reply)
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
/// The write end of the resize pipe, for the signal handler.
static WINCH_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

extern "C" fn on_winch(_: std::ffi::c_int) {
    let fd = WINCH_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        // write(2) is async-signal-safe; a full pipe already means "wake".
        unsafe {
            write(fd, [1u8].as_ptr(), 1);
        }
    }
}

/// Turn SIGWINCH into an empty message on the key channel, so a resize wakes
/// an idle loop at once.
fn wake_on_resize(tx: Sender<Vec<u8>>) {
    let mut fds = [0 as c_int; 2];
    if unsafe { pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    WINCH_FD.store(fds[1], Ordering::Relaxed);
    unsafe {
        signal(SIGWINCH, on_winch as *const () as usize);
    }
    std::thread::spawn(move || {
        let mut byte = [0u8; 1];
        while unsafe { read(fds[0], byte.as_mut_ptr(), 1) } > 0 {
            if tx.send(Vec::new()).is_err() {
                break;
            }
        }
    });
}

/// SIGWINCH is 28 on macOS and Linux.
const SIGWINCH: c_int = 28;

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
        print!("{}", enter_sequence(MOUSE.load(Ordering::Relaxed)));
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
    Add,
    Delete,
    Border,
    Success,
    Code,
    Link,
}

pub(crate) struct Palette {
    on: bool,
    /// Colour as the terminal and settings allow it, before `mono` turns it off.
    allowed: bool,
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
        let allowed = on;
        if theme.name == "mono" {
            on = false;
        }
        let truecolor = std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false);
        Self {
            on,
            allowed,
            truecolor,
            theme,
        }
    }

    /// The same terminal and settings, painting with another theme.
    pub(crate) fn with_theme(&self, theme: &'static crate::theme::Theme) -> Self {
        Self {
            on: self.allowed && theme.name != "mono",
            allowed: self.allowed,
            truecolor: self.truecolor,
            theme,
        }
    }

    /// The highlighted picker row: user text on the select background.
    pub(crate) fn select(&self, text: &str) -> String {
        if !self.on {
            return text.to_string();
        }
        let back = self.theme.rgb(crate::theme::Slot::Select);
        let bg = if self.truecolor {
            format!("\x1b[48;2;{};{};{}m", back[0], back[1], back[2])
        } else {
            format!("\x1b[48;5;{}m", crate::theme::to_256(back))
        };
        // The fg reset inside `paint` keeps the background until the end.
        format!("{bg}{}\x1b[0m", self.paint(Role::User, text).replace("\x1b[0m", "\x1b[39m"))
    }

    pub(crate) fn theme_name(&self) -> &'static str {
        self.theme.name
    }

    pub(crate) fn paint(&self, role: Role, text: &str) -> String {
        if !self.on {
            return text.to_string();
        }
        use crate::theme::Slot;
        let slot = match role {
            Role::User => Slot::User,
            Role::Assistant => Slot::Text,
            Role::Thinking => Slot::Muted,
            Role::Tool => Slot::Tool,
            Role::Error | Role::Delete => Slot::Error,
            Role::Info => Slot::Accent,
            Role::Add | Role::Success => Slot::Success,
            Role::Border => Slot::Border,
            Role::Code => Slot::Code,
            Role::Link => Slot::Link,
        };
        let [r, g, b] = self.theme.rgb(slot);
        let fallback = crate::theme::to_256([r, g, b]);
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
    WheelUp,
    WheelDown,
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
            if let (Some(sgr), b'M' | b'm') = (params.strip_prefix(b"<"), b) {
                return Some((i + 1, mouse(sgr)));
            }
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

/// An SGR mouse report `b;x;y`. Only the wheel matters; clicks are ignored.
fn mouse(params: &[u8]) -> Key {
    let text = std::str::from_utf8(params).unwrap_or("");
    let button: u32 = text.split(';').next().and_then(|b| b.parse().ok()).unwrap_or(0);
    // Bits 4, 8, and 16 are shift, alt, and ctrl held during the event.
    match button & !0b11100 {
        64 => Key::WheelUp,
        65 => Key::WheelDown,
        _ => Key::Unknown,
    }
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
    SetModel(String),
    ListModels,
    ShowAgent,
    Fork,
    Shell,
    RenameCurrent(String),
    UseProvider {
        name: String,
        dialect: String,
        base: String,
        model: String,
        key_env: String,
    },
    SetRoot(std::path::PathBuf),
    Compact,
    Resume(String),
    Reload,
    SetPermissions(crate::config::Permissions),
    Rename(String, String),
    Delete(String),
    NewSession,
    Reset,
    Quit,
}

pub enum Ev {
    Token(String),
    /// A restored conversation, drawn in place of the transcript.
    Replay(Vec<(Role, String)>),
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
        detail: String,
        ms: u128,
    },
    Info(String),
    Error(String),
    Models(Vec<String>),
    Usage {
        tokens: u64,
        hit: u64,
        miss: u64,
        output: u64,
        model: Option<String>,
    },
    Confirm {
        what: String,
        reply: Sender<Answer>,
        /// Whether `a` (always) is offered.
        always: bool,
    },
    Question {
        q: String,
        opts: Vec<String>,
        reply: Sender<Option<String>>,
    },
    Stop(String),
    Done(Option<String>),
}

/// How long a confirmation waits before it gives up and stops the turn.
const CONFIRM_WAIT: Duration = Duration::from_secs(60);

struct WorkerHooks {
    tx: Sender<Ev>,
    waited: Duration,
}

impl Hooks for WorkerHooks {
    fn ui(&mut self, event: Ui) {
        let ev = match event {
            Ui::Content(t) => Ev::Token(t),
            Ui::Reasoning(t) => Ev::Think(t),
            Ui::ToolStart {
                name,
                summary,
                args,
            } => Ev::ToolStart {
                name,
                summary,
                args,
            },
            Ui::ToolEnd {
                ok,
                preview,
                output,
                detail,
                ms,
            } => Ev::ToolEnd {
                ok,
                preview,
                output,
                detail,
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
        self.answer(what, false) == Answer::Yes
    }

    fn confirm_always(&mut self, what: &str) -> Answer {
        self.answer(what, true)
    }

    fn run_attached(&mut self, command: &str) -> Option<i32> {
        // The shell gets the mouse back, and imp takes it again after.
        print!("{MOUSE_OFF}\x1b[?25h\x1b[?1049l");
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
        print!("{}", enter_sequence(MOUSE.load(Ordering::Relaxed)).replace("\x1b[>1u", ""));
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

impl WorkerHooks {
    fn answer(&mut self, what: &str, always: bool) -> Answer {
        self.waited = Duration::ZERO;
        let opened = Instant::now();
        let (tx, rx) = channel();
        if self
            .tx
            .send(Ev::Confirm {
                what: what.to_string(),
                reply: tx,
                always,
            })
            .is_err()
        {
            return Answer::No;
        }
        // The deadline is the UI's: it counts only while the prompt is in view.
        let answered = rx.recv().unwrap_or(Answer::No);
        self.waited = opened.elapsed();
        answered
    }
}


struct Entry {
    role: Role,
    text: String,
}

/// One drawn row: plain text, or styled spans from the markdown renderer.
enum Line {
    Text(String),
    /// A tool row's last summary line, with its duration for the right edge.
    Timed(String, String),
    Spans(Vec<crate::render::Span>),
}

/// One entry's rows at `cols`, led by a blank row when `gap` opens a turn.
fn entry_lines(entry: &Entry, cols: usize, markdown: bool, gap: bool) -> Vec<(Role, Line)> {
    let mut lines = Vec::new();
    if gap {
        lines.push((Role::User, Line::Text(String::new())));
    }
    if entry.role == Role::Assistant {
        for row in crate::render::lines(&entry.text, cols, markdown) {
            lines.push((entry.role, Line::Spans(row)));
        }
        return lines;
    }
    // A finished tool row: wrap the summary, then set its duration
    // at the right edge of the last summary line, or under it.
    let mut timed: Option<String> = None;
    // Tool output and thinking never drive the terminal; TIME stays.
    let mut text: String = entry
        .text
        .split(TIME)
        .map(|part| crate::render::clean(part).replace('\t', "    "))
        .collect::<Vec<_>>()
        .join(&TIME.to_string());
    let mut head_rows = 0;
    if let Some(at) = text.find(TIME) {
        let end = text[at..].find('\n').map_or(text.len(), |n| at + n);
        timed = Some(text[at + TIME.len_utf8()..end].to_string());
        text.replace_range(at..end, "");
        // The summary may span lines, as a heredoc does.
        head_rows = wrap(&text[..at], cols.saturating_sub(2)).len();
    }
    for (i, part) in wrap(&text, cols.saturating_sub(2))
        .into_iter()
        .enumerate()
    {
        let prefix = if i == 0 {
            match entry.role {
                Role::User => "> ",
                Role::Thinking => "~ ",
                _ => "",
            }
        } else if entry.role == Role::Info {
            // Info text lays itself out, as `/help` does.
            ""
        } else {
            "  "
        };
        let row = format!("{prefix}{part}");
        match &timed {
            Some(time) if i + 1 == head_rows => {
                if row.chars().count() + 2 + time.chars().count() <= cols {
                    lines.push((entry.role, Line::Timed(row, time.clone())));
                } else {
                    lines.push((entry.role, Line::Text(row)));
                    lines.push((entry.role, Line::Timed(String::new(), time.clone())));
                }
            }
            _ => lines.push((entry.role, Line::Text(row))),
        }
    }
    lines
}

/// Separates a tool summary from its duration in the stored row text.
const TIME: char = '\u{1}';

/// Animated frames for a running tool row.
const SPINNER: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

fn spinner_frame(tick: usize) -> char {
    SPINNER[tick % SPINNER.len()]
}

/// Put a spinner frame before the count of each working session's row, and
/// two spaces before the rest, so titles keep their width.
fn spin_rows(items: &mut [PickItem], working: &[String], frame: char) {
    for item in items {
        let count = item.desc.trim_start_matches(|c| SPINNER.contains(&c) || c == ' ');
        item.desc = if working.contains(&item.value) { format!("{frame} {count}") } else { format!("  {count}") };
    }
}

/// The tool call on screen: one row, updated in place when it finishes.
struct ToolRow {
    index: usize,
    summary: String,
    args: String,
    preview: String,
    output: String,
    detail: String,
    detail_start: Option<usize>,
    detail_count: usize,
    ok: bool,
    ms: u128,
    running: bool,
    expanded: bool,
}

fn preview_detail(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().take(12).map(str::to_string).collect();
    let total = text.lines().count();
    if total > 12 {
        lines.push(format!("… +{} lines (Ctrl+E)", total - 12));
    }
    lines
}

fn tool_row_text(row: &ToolRow, verbose: bool) -> String {
    let mark = if row.ok { "✓" } else { "✗" };
    let mut text = format!("{mark} {}{TIME}{}", row.summary, duration(row.ms));
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
    pending: Option<Pending>,
    pending_since: Option<Instant>,
    /// The theme active when the theme picker opened, while it previews.
    preview_from: Option<&'static crate::theme::Theme>,
    /// Where managed provider profiles are written.
    config_path: std::path::PathBuf,
    /// Where recently chosen models are remembered.
    models_path: std::path::PathBuf,
    /// The provider's models, catalog first and then whatever it reported.
    available_models: Vec<String>,
    /// Messages parked for this session, never persisted.
    stash: Vec<String>,
    bell_pending: bool,
    status: Status,
    verbose: bool,
    root: std::path::PathBuf,
    agent: String,
    /// Agent profiles as name and description.
    agents: Vec<(String, String)>,
    sessions_dir: std::path::PathBuf,
    last_redraw: Instant,
    last_size: (usize, usize),
    dirty: bool,
    quit: bool,
    spinner: usize,
    /// Rendered rows per entry, keyed by a hash of what shapes them.
    drawn: Vec<(u64, Vec<(Role, Line)>)>,
    tool: Option<ToolRow>,
    /// The worker for the session in view; `None` in tests.
    worker: Option<Worker>,
    /// When the session in view last did anything, to pick one to close.
    touched: Instant,
    /// The session in view finished a turn while out of view.
    replied: bool,
    /// When this session started waiting on the user; orders the queue.
    since: Instant,
    /// Every live session in order; the one in view is `None` at `at`.
    panes: Vec<Option<Pane>>,
    at: usize,
    /// Builds a further live session; `None` keeps `/new` in one session.
    fresh: Option<Fresh>,
}

/// Builds a session for a working directory, for another live session.
pub type Fresh = Box<dyn Fn(&std::path::Path) -> Session>;

/// The most live sessions at once; the Pi Zero has 512 MB.
const MAX_PANES: usize = 4;

/// A live session out of view: the fields `App` holds for the one in view.
struct Pane {
    cfg: Config,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    entries: Vec<Entry>,
    input: String,
    cursor: usize,
    hist_pos: Option<usize>,
    follow: bool,
    top: usize,
    max_top: usize,
    unseen: usize,
    pending: Option<Pending>,
    pending_since: Option<Instant>,
    stash: Vec<String>,
    bell_pending: bool,
    status: Status,
    tool: Option<ToolRow>,
    root: std::path::PathBuf,
    agent: String,
    worker: Option<Worker>,
    touched: Instant,
    replied: bool,
    since: Instant,
}

impl Pane {
    fn new(session: &Session, worker: Option<Worker>) -> Self {
        Self {
            cfg: session.cfg.clone(),
            cancel: session.cancel.clone(),
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            hist_pos: None,
            follow: true,
            top: 0,
            max_top: 0,
            unseen: 0,
            pending: None,
            pending_since: None,
            stash: Vec::new(),
            bell_pending: false,
            status: Status::default(),
            tool: None,
            root: session.tools.root.clone(),
            agent: session.agent.name.clone(),
            worker,
            touched: Instant::now(),
            replied: false,
            since: Instant::now(),
        }
    }

    fn idle(&self) -> bool {
        !self.status.streaming && self.pending.is_none()
    }

    /// Waiting on the user: a reply not yet seen, or an open prompt.
    fn waits(&self) -> bool {
        self.replied || matches!(self.pending, Some(Pending::Confirm { .. } | Pending::Question { .. }))
    }
}

enum Pending {
    Pick(Picker),
    Confirm {
        what: String,
        reply: Sender<Answer>,
        /// Whether `a` (always) is offered.
        always: bool,
    },
    Prompt {
        label: String,
        value: String,
        rename_id: String,
    },
    RenameCurrent {
        value: String,
    },
    ProviderForm {
        name: String,
        fields: Vec<(String, String)>,
        at: usize,
    },
    ConfirmDelete {
        id: String,
        title: String,
        /// The session list, drawn behind the question and reopened after.
        picker: Box<Picker>,
    },
    Question {
        q: String,
        opts: Vec<String>,
        reply: Sender<Option<String>>,
        mode: AskMode,
        typed: String,
    },
}

enum AskMode {
    Choose,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PickKind {
    Agent,
    Session,
    Verbosity,
    Dir,
    Image,
    Command,
    Model,
    Theme,
    Provider,
    Mode,
    Keys,
}

#[derive(Clone)]
struct PickItem {
    label: String,
    value: String,
    /// A one-line description drawn in the right column.
    desc: String,
    /// Single-key actions for this row, shown in the footer and run while the
    /// filter is empty.
    actions: Vec<(char, &'static str)>,
}

/// The one picker: a title, a filter, a scrolling list, and a selection.
struct Picker {
    title: String,
    filter: String,
    items: Vec<PickItem>,
    index: usize,
    offset: usize,
    kind: PickKind,
    /// Text before the filter that the composer should show, when typed
    /// characters belong to a command line.
    prefix: Option<String>,
    current: std::path::PathBuf,
    root: std::path::PathBuf,
}

/// Recently chosen models, newest first.
fn load_models(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(8)
        .map(str::to_string)
        .collect()
}

/// Move a model to the front of the recents file.
fn record_model(path: &std::path::Path, name: &str) {
    let mut names: Vec<String> = load_models(path)
        .into_iter()
        .filter(|old| old != name)
        .collect();
    names.insert(0, name.to_string());
    names.truncate(8);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, names.join("\n"));
}

/// Every slash command as a picker row.
fn command_items() -> Vec<PickItem> {
    COMMANDS
        .iter()
        .map(|(name, desc)| PickItem {
            label: name.to_string(),
            value: name.to_string(),
            desc: desc.to_string(),
            actions: Vec::new(),
        })
        .collect()
}

/// One list row: mark, label, and an aligned description when the picker has
/// any.
/// The right column of a picker row, led by `current` for the active entry.
fn mark_current(current: bool, about: &str) -> String {
    match (current, about.is_empty()) {
        (true, true) => "current".into(),
        (true, false) => format!("current · {about}"),
        (false, _) => about.to_string(),
    }
}

fn picker_row(picker: &Picker, at: usize, index: usize, inner: usize) -> String {
    let item = &picker.items[index];
    let marker = if at == picker.index { "▸" } else { " " };
    if picker.items.iter().any(|item| !item.desc.is_empty()) {
        let mut left = picker
            .items
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(0)
            .max(10)
            + 2;
        let mut label = item.label.clone();
        let mut desc = item.desc.clone();
        if picker.kind != PickKind::Command {
            // The right column stays whole, one space off the wall; the
            // label gives way.
            let wide = picker.items.iter().map(|i| i.desc.chars().count()).max().unwrap_or(0);
            left = left.min(inner.saturating_sub(2 + 2 + wide + 1)).max(4);
            label = fit(&label, left);
            if picker.kind == PickKind::Session {
                // Counts line up on their last digit.
                desc = format!("{desc:>wide$}");
            }
        }
        format!("{marker} {label:<left$}  {desc}")
    } else {
        format!("{marker} {}", item.label)
    }
}

/// Commands whose argument is typed rather than picked.
const FREE_TEXT: &[&str] = &["stash", "rename"];

/// Rows the picker shows at once. Paging moves by the same amount.
const PICK_ROWS: usize = 8;

impl Picker {
    fn new(title: impl Into<String>, mut items: Vec<PickItem>, kind: PickKind) -> Self {
        if kind == PickKind::Session {
            // Every session row offers the same list actions.
            for item in items.iter_mut() {
                item.actions = vec![('n', "new"), ('r', "rename"), ('d', "delete")];
            }
        }
        Self {
            title: title.into(),
            filter: String::new(),
            items,
            index: 0,
            offset: 0,
            kind,
            prefix: None,
            current: std::path::PathBuf::new(),
            root: std::path::PathBuf::new(),
        }
    }

    /// A browser start at the session root, never above it.
    fn browse(title: impl Into<String>, kind: PickKind, root: &std::path::Path) -> Self {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut picker = Self::new(title, Vec::new(), kind);
        picker.current = root.clone();
        picker.root = root;
        picker.reload();
        picker
    }

    fn reload(&mut self) {
        self.items = match self.kind {
            PickKind::Dir => browse_items(&self.current, &self.current, false),
            PickKind::Image => browse_items(&self.current, &self.root, true),
            _ => Vec::new(),
        };
        if self.kind == PickKind::Dir {
            // `./` makes the folder on screen choosable; `../` sits under it.
            let row = |label: &str, path: &std::path::Path| PickItem {
                label: label.into(),
                value: path.display().to_string(),
                actions: Vec::new(),
                desc: String::new(),
            };
            let mut fixed = vec![row("./", &self.current)];
            if let Some(up) = self.current.parent() {
                fixed.push(row("../", up));
            }
            self.items.splice(0..0, fixed);
        }
        self.filter.clear();
        self.reset();
    }

    /// Move one directory up. A directory browser may climb above the session
    /// root; the image browser stays inside it.
    fn parent(&mut self) -> bool {
        if self.current == self.root && self.kind != PickKind::Dir {
            return false;
        }
        let Some(parent) = self.current.parent() else {
            return false;
        };
        let parent = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        self.current = if self.kind == PickKind::Dir || parent.starts_with(&self.root) {
            parent
        } else {
            self.root.clone()
        };
        self.reload();
        true
    }

    /// Indices of the items matching the filter, prefix matches first.
    fn matches(&self) -> Vec<usize> {
        let needle = self.filter.to_lowercase();
        let mut out: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| needle.is_empty() || item.label.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        out.sort_by_key(|&i| {
            !self.items[i]
                .label
                .trim_start_matches('/')
                .to_lowercase()
                .starts_with(&needle)
        });
        out
    }

    /// The action a row declares for this key, if any.
    fn action(&self, key: char) -> Option<&'static str> {
        self.selected().and_then(|item| {
            item.actions
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, label)| *label)
        })
    }

    /// One hint line naming the keys that work here.
    fn footer(&self) -> String {
        let mut parts: Vec<String> = match self.kind {
            PickKind::Dir => vec![
                "Enter choose".into(),
                "Tab/→ open".into(),
                "← up".into(),
                "Esc close".into(),
            ],
            PickKind::Image => vec![
                "Enter attach".into(),
                "Tab/→ open".into(),
                "← up".into(),
                "Esc close".into(),
            ],
            PickKind::Keys => return "? or Esc close".into(),
            _ => vec!["Enter choose".into(), "Esc close".into()],
        };
        if let Some(item) = self.items.first() {
            for (key, label) in &item.actions {
                parts.push(format!("{key} {label}"));
            }
        }
        parts.join(" · ")
    }

    fn selected(&self) -> Option<&PickItem> {
        let matches = self.matches();
        matches.get(self.index).map(|&i| &self.items[i])
    }

    fn reset(&mut self) {
        self.index = 0;
        self.offset = 0;
    }

    fn move_by(&mut self, delta: isize) {
        let count = self.matches().len();
        if count == 0 {
            self.reset();
            return;
        }
        let target = (self.index as isize + delta).clamp(0, count as isize - 1);
        self.index = target as usize;
        if self.index < self.offset {
            self.offset = self.index;
        }
        if self.index >= self.offset + PICK_ROWS {
            self.offset = self.index + 1 - PICK_ROWS;
        }
        if self.offset + PICK_ROWS > count {
            self.offset = count.saturating_sub(PICK_ROWS);
        }
    }

    fn filter_push(&mut self, c: char) {
        self.filter.push(c);
        self.reset();
    }

    fn filter_pop(&mut self) {
        self.filter.pop();
        self.reset();
    }
}

/// Directories under `dir`, or directories plus image files when `images`.
///
/// Paths are canonicalised and must stay under the session root, so a symlink
/// cannot lead the browser outside it.
fn browse_items(dir: &std::path::Path, root: &std::path::Path, images: bool) -> Vec<PickItem> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if crate::tools::IGNORE.contains(&name.as_str()) {
            continue;
        }
        let Ok(path) = entry.path().canonicalize() else {
            continue;
        };
        if !path.starts_with(root) {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            out.push(PickItem {
                label: format!("{name}/"),
                value: path.display().to_string(),
                actions: Vec::new(),
                desc: String::new(),
            });
        } else if images && is_image_name(&name) {
            out.push(PickItem {
                label: name,
                value: path.display().to_string(),
                actions: Vec::new(),
                desc: String::new(),
            });
        }
    }
    out.sort_by_cached_key(|item| item.label.to_lowercase());
    out
}

fn is_image_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

#[derive(Default)]
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
        agent: String,
    ) -> Self {
        let palette = Palette::detect(&cfg);
        let provider = cfg.provider;
        let history = load_history();
        let agents = crate::store::load_agents(&cfg, &root)
            .into_iter()
            .map(|a| (a.name, a.description))
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
            pending: None,
            pending_since: None,
            preview_from: None,
            config_path: Config::config_path(),
            models_path: crate::config::dirs_state().join("imp").join("models"),
            available_models: crate::providers::catalog(provider),
            stash: Vec::new(),
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
            agent,
            agents,
            sessions_dir,
            last_redraw: Instant::now() - Duration::from_secs(1),
            last_size: (0, 0),
            dirty: true,
            quit: false,
            spinner: 0,
            drawn: Vec::new(),
            tool: None,
            worker: None,
            touched: Instant::now(),
            replied: false,
            since: Instant::now(),
            panes: vec![None],
            at: 0,
            fresh: None,
        }
    }

    /// Exchange the per-session fields with a pane's.
    fn swap_pane(&mut self, p: &mut Pane) {
        use std::mem::swap;
        swap(&mut self.cfg, &mut p.cfg);
        swap(&mut self.cancel, &mut p.cancel);
        swap(&mut self.entries, &mut p.entries);
        swap(&mut self.input, &mut p.input);
        swap(&mut self.cursor, &mut p.cursor);
        swap(&mut self.hist_pos, &mut p.hist_pos);
        swap(&mut self.follow, &mut p.follow);
        swap(&mut self.top, &mut p.top);
        swap(&mut self.max_top, &mut p.max_top);
        swap(&mut self.unseen, &mut p.unseen);
        swap(&mut self.pending, &mut p.pending);
        swap(&mut self.pending_since, &mut p.pending_since);
        swap(&mut self.stash, &mut p.stash);
        swap(&mut self.bell_pending, &mut p.bell_pending);
        swap(&mut self.status, &mut p.status);
        swap(&mut self.tool, &mut p.tool);
        swap(&mut self.root, &mut p.root);
        swap(&mut self.agent, &mut p.agent);
        swap(&mut self.worker, &mut p.worker);
        swap(&mut self.touched, &mut p.touched);
        swap(&mut self.replied, &mut p.replied);
        swap(&mut self.since, &mut p.since);
    }

    /// Bring the pane at `index` into view and park the current one.
    fn show(&mut self, index: usize) {
        if index == self.at {
            return;
        }
        let Some(mut p) = self.panes[index].take() else { return };
        self.swap_pane(&mut p);
        self.panes[self.at] = Some(p);
        self.at = index;
        self.touched = Instant::now();
        self.replied = false;
        // A prompt's deadline counts from when it is in view.
        if self.pending_since.is_some() {
            self.pending_since = Some(Instant::now());
        }
        self.dirty = true;
    }

    /// Apply a worker event to a pane out of view, through the same code.
    fn apply_to(&mut self, index: usize, ev: Ev) {
        if index == self.at {
            return self.apply(ev);
        }
        let Some(mut p) = self.panes[index].take() else { return };
        self.swap_pane(&mut p);
        if matches!(ev, Ev::Done(_)) {
            self.replied = true;
            self.since = Instant::now();
        }
        self.apply(ev);
        self.touched = Instant::now();
        // The bell rings once, now, rather than again when the pane is shown.
        let ring = std::mem::take(&mut self.bell_pending);
        self.swap_pane(&mut p);
        self.panes[index] = Some(p);
        self.bell_pending |= ring;
    }

    /// Refuse a prompt that has been in view for the whole deadline.
    fn expire(&mut self) {
        let due = self.pending_since.is_some_and(|t| t.elapsed() >= CONFIRM_WAIT);
        if !due {
            return;
        }
        match self.pending.take() {
            Some(Pending::Confirm { reply, .. }) => {
                let _ = reply.send(Answer::No);
            }
            Some(Pending::Question { reply, .. }) => {
                let _ = reply.send(None);
            }
            other => {
                self.pending = other;
                return;
            }
        }
        self.apply(Ev::Stop(format!(
            "no answer within {}s; nothing ran and the turn stopped",
            CONFIRM_WAIT.as_secs()
        )));
    }

    /// Sessions out of view waiting on the user, longest wait first.
    fn queue(&self) -> Vec<usize> {
        let mut q: Vec<(Instant, usize)> = self
            .panes
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.as_ref().filter(|p| p.waits()).map(|p| (p.since, i)))
            .collect();
        q.sort();
        q.into_iter().map(|(_, i)| i).collect()
    }

    fn waiting(&self) -> usize {
        self.queue().len()
    }

    /// Ctrl+O: the session that has waited longest. A reply in view counts as
    /// read; an open prompt in view goes to the back of the queue, unanswered.
    fn next_waiting(&mut self) -> Option<usize> {
        if self.pending.is_some() {
            self.since = Instant::now();
        }
        let next = *self.queue().first()?;
        self.show(next);
        Some(next)
    }

    /// Apply every waiting worker event, in view or not. True when any arrived.
    fn poll(&mut self) -> bool {
        let mut any = false;
        for i in 0..self.panes.len() {
            let worker = match &self.panes[i] {
                None => self.worker.as_ref(),
                Some(p) => p.worker.as_ref(),
            };
            let Some((_, rx, _)) = worker else { continue };
            let mut events = Vec::new();
            let mut gone = false;
            loop {
                match rx.try_recv() {
                    Ok(ev) => events.push(ev),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        gone = true;
                        break;
                    }
                }
            }
            any |= !events.is_empty();
            for ev in events {
                self.apply_to(i, ev);
            }
            if gone && i == self.at {
                self.quit = true;
            }
        }
        any
    }

    /// Whether the pane at `index` is neither working nor waiting.
    fn pane_idle(&self, index: usize) -> bool {
        match &self.panes[index] {
            None => !self.status.streaming && self.pending.is_none(),
            Some(p) => p.idle(),
        }
    }

    fn pane_touched(&self, index: usize) -> Instant {
        self.panes[index].as_ref().map_or(self.touched, |p| p.touched)
    }

    /// Stop a pane's worker and drop it; the session stays in `/sessions`.
    fn close(&mut self, index: usize) {
        if index == self.at {
            return;
        }
        if let Some(Some(p)) = self.panes.get(index) {
            if let Some((tx, _, _)) = &p.worker {
                let _ = tx.send(Cmd::Quit);
            }
        }
        self.panes.remove(index);
        if index < self.at {
            self.at -= 1;
        }
        self.dirty = true;
    }

    /// Open a session in a new live pane, closing the oldest idle one for room.
    fn open(&mut self, session: Session) -> bool {
        let mut victim = None;
        if self.panes.len() >= MAX_PANES {
            victim = (0..self.panes.len())
                .filter(|&i| self.pane_idle(i))
                .min_by_key(|&i| self.pane_touched(i));
            if victim.is_none() {
                self.push(Role::Error, "all four sessions are busy; answer or stop one first");
                return false;
            }
        }
        let mut pane = Pane::new(&session, None);
        pane.worker = Some(spawn_worker(session, self.sessions_dir.clone()));
        self.panes.push(Some(pane));
        self.show(self.panes.len() - 1);
        // After the switch the victim is out of view, even if it was in view.
        if let Some(v) = victim {
            self.close(v);
        }
        true
    }

    /// Start another live session, or reset this one without a factory.
    fn new_session(&mut self, tx: &Sender<Cmd>) {
        let Some(fresh) = &self.fresh else {
            let _ = tx.send(Cmd::Reset);
            self.entries.clear();
            self.push(Role::Info, "new conversation");
            return;
        };
        let mut session = fresh(&self.root);
        session.attach_log(crate::store::Log::create(&self.sessions_dir, &self.root, &session.cfg.model));
        if self.open(session) {
            self.push(Role::Info, "new conversation");
        }
    }

    /// The session picker's `n`: a new logged session, as before without a factory.
    fn new_session_from_picker(&mut self, tx: &Sender<Cmd>) {
        if self.fresh.is_some() {
            return self.new_session(tx);
        }
        let _ = tx.send(Cmd::NewSession);
        self.entries.clear();
        self.push(Role::Info, "new conversation");
    }

    /// Resume a stored session in another live pane, or in this one.
    fn resume(&mut self, id: String, tx: &Sender<Cmd>) {
        let Some(fresh) = &self.fresh else {
            self.entries.clear();
            self.push(Role::Info, format!("resuming {id}"));
            let _ = tx.send(Cmd::Resume(id));
            return;
        };
        let mut session = fresh(&self.root);
        match session.resume_id(&self.sessions_dir, &id) {
            Ok(()) => {
                if self.open(session) {
                    self.push(Role::Info, format!("resumed {id}"));
                }
            }
            Err(e) => self.push(Role::Error, e),
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

    /// Move the view by `rows`; reaching the end resumes following.
    fn scroll(&mut self, rows: isize) {
        if rows < 0 {
            self.follow = false;
            self.top = self.top.saturating_sub(rows.unsigned_abs());
        } else {
            self.top = self.top.saturating_add(rows as usize);
            if self.top >= self.max_top {
                self.follow_tail();
            }
        }
        self.dirty = true;
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
            Ev::ToolStart {
                name,
                summary,
                args,
            } => {
                let _ = name;
                self.tool = Some(ToolRow {
                    index: self.entries.len(),
                    summary: summary.clone(),
                    args,
                    preview: String::new(),
                    output: String::new(),
                    detail: String::new(),
                    detail_start: None,
                    detail_count: 0,
                    ok: true,
                    ms: 0,
                    running: true,
                    expanded: false,
                });
                let frame = spinner_frame(self.spinner);
                self.push(Role::Tool, format!("{frame} {summary}"))
            }
            Ev::ToolEnd {
                ok,
                preview,
                output,
                detail,
                ms,
            } => {
                let role = if ok { Role::Tool } else { Role::Error };
                match self.tool.take() {
                    Some(mut row) => {
                        row.ok = ok;
                        row.ms = ms;
                        row.running = false;
                        row.output = output;
                        row.preview = preview;
                        row.detail = detail;
                        if !row.detail.is_empty() {
                            let start = self.entries.len();
                            let preview = preview_detail(&row.detail);
                            row.detail_count = preview.len();
                            row.detail_start = Some(start);
                            for line in preview {
                                let role = if line.starts_with('+') {
                                    Role::Add
                                } else if line.starts_with('-') {
                                    Role::Delete
                                } else {
                                    Role::Thinking
                                };
                                self.push(role, line);
                            }
                        }
                        let text = tool_row_text(&row, self.verbose);
                        if let Some(entry) = self.entries.get_mut(row.index) {
                            entry.role = role;
                            entry.text = text;
                        }
                        self.tool = Some(row);
                    }
                    None => self.push(
                        role,
                        format!("{} {preview}{TIME}{}", if ok { "✓" } else { "✗" }, duration(ms)),
                    ),
                }
            }
            Ev::Info(t) => self.push(Role::Info, t),
            Ev::Error(t) => self.push(Role::Error, t),
            Ev::Root(path) => {
                self.root = path;
                self.dirty = true;
            }
            Ev::Models(models) => {
                self.available_models = models;
                let model_picker = matches!(
                    &self.pending,
                    Some(Pending::Pick(picker)) if picker.kind == PickKind::Model
                );
                if model_picker {
                    let items = self.model_items();
                    if let Some(Pending::Pick(picker)) = &mut self.pending {
                        picker.items = items;
                        picker.reset();
                    }
                }
                self.dirty = true;
            }
            Ev::Replay(rows) => {
                self.entries.clear();
                self.tool = None;
                for (role, text) in rows {
                    self.push(role, text);
                }
                self.follow_tail();
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
            Ev::Confirm { what, reply, always } => {
                self.pending = Some(Pending::Confirm { what, reply, always });
                self.pending_since = Some(Instant::now());
                self.since = Instant::now();
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
                self.since = Instant::now();
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

    fn handle_key(&mut self, key: Key, tx: &Sender<Cmd>) {
        if matches!(key, Key::Ctrl('o')) && !matches!(self.pending, Some(Pending::Pick(_))) {
            self.go_next();
        } else {
            // A key that answers a prompt or starts a turn hands the session off.
            let asked = matches!(self.pending, Some(Pending::Confirm { .. } | Pending::Question { .. }));
            let (at, was_working) = (self.at, self.status.streaming);
            let interrupt = matches!(key, Key::Ctrl('c'));
            self.handle_key_inner(key, tx);
            let handed = self.at == at
                && !interrupt
                && ((asked && self.pending.is_none()) || (!was_working && self.status.streaming));
            if handed && self.cfg.auto_next {
                if let Some(&i) = self.queue().first() {
                    let prompt = self.panes[i].as_ref().is_some_and(|p| p.pending.is_some());
                    let state = if prompt { "needs you" } else { "replied" };
                    self.show(i);
                    let note = format!("→ {}/{} {} · {state}", i + 1, self.panes.len(), title_of(&self.entries));
                    self.push(Role::Info, note);
                }
            }
        }
        self.sync_preview();
    }

    fn go_next(&mut self) {
        if self.next_waiting().is_none() {
            self.push(Role::Info, "nothing waiting");
        }
        self.dirty = true;
    }

    /// The open theme picker paints the screen in its highlighted theme; any
    /// close but Enter puts the starting theme back.
    fn sync_preview(&mut self) {
        let shown = match &self.pending {
            Some(Pending::Pick(p)) if p.kind == PickKind::Theme => {
                Some(p.selected().and_then(|i| crate::theme::by_name(&i.value)))
            }
            _ => None,
        };
        match shown {
            Some(choice) => {
                let start = *self.preview_from.get_or_insert(self.palette.theme);
                let theme = choice.unwrap_or(start);
                if !std::ptr::eq(theme, self.palette.theme) {
                    self.palette = self.palette.with_theme(theme);
                    self.dirty = true;
                }
            }
            None => {
                if let Some(start) = self.preview_from.take() {
                    self.palette = self.palette.with_theme(start);
                    self.dirty = true;
                }
            }
        }
    }

    fn handle_key_inner(&mut self, key: Key, tx: &Sender<Cmd>) {
        // Clicks and unknown sequences do nothing, and must not answer a prompt.
        if matches!(key, Key::Unknown) {
            return;
        }
        // `?` on an empty composer shows the keys; `?` again closes them.
        if matches!(key, Key::Char('?')) {
            match &self.pending {
                None if self.input.is_empty() => {
                    let rows = crate::help::KEYS
                        .iter()
                        .map(|(k, what)| PickItem {
                            label: k.to_string(),
                            value: k.to_string(),
                            desc: what.to_string(),
                            actions: Vec::new(),
                        })
                        .collect();
                    self.pending = Some(Pending::Pick(Picker::new("keys", rows, PickKind::Keys)));
                    self.dirty = true;
                    return;
                }
                Some(Pending::Pick(p)) if p.kind == PickKind::Keys => {
                    self.pending = None;
                    self.dirty = true;
                    return;
                }
                _ => {}
            }
        }
        if matches!(key, Key::WheelUp | Key::WheelDown) {
            match &mut self.pending {
                Some(Pending::Pick(picker)) => {
                    picker.move_by(if matches!(key, Key::WheelUp) { -1 } else { 1 });
                    self.dirty = true;
                }
                Some(_) => {}
                None => self.scroll(if matches!(key, Key::WheelUp) { -3 } else { 3 }),
            }
            return;
        }
        // Ctrl+C protects a draft first. Prompts do not use the composer, so
        // they keep the exit; a picker is part of composing, so it closes with
        // the text.
        if matches!(key, Key::Ctrl('c')) {
            let prompt = matches!(
                self.pending,
                Some(Pending::Confirm { .. })
                    | Some(Pending::ConfirmDelete { .. })
                    | Some(Pending::Question { .. })
                    | Some(Pending::Prompt { .. })
                    | Some(Pending::ProviderForm { .. })
                    | Some(Pending::RenameCurrent { .. })
            );
            if prompt || self.input.is_empty() {
                self.quit = true;
            } else {
                self.input.clear();
                self.cursor = 0;
                self.hist_pos = None;
                self.pending = None;
                self.pending_since = None;
                self.follow_tail();
                self.dirty = true;
            }
            return;
        }
        if let Some(pending) = self.pending.take() {
            self.answer(pending, key, tx);
            if self.pending.is_none() {
                self.pending_since = None;
            }
            return;
        }

        match key {
            Key::Ctrl('c') => self.quit = true,
            Key::Ctrl('d') => self.quit = true,
            Key::Ctrl('n') | Key::Ctrl('p') if self.panes.len() > 1 => {
                let n = self.panes.len();
                let step = if matches!(key, Key::Ctrl('n')) { 1 } else { n - 1 };
                self.show((self.at + step) % n);
            }
            Key::Ctrl('e') => {
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
            Key::Tab => self.sync_menu(),
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
        if let (Some(start), true) = (row.detail_start, !row.detail.is_empty()) {
            if start + row.detail_count == self.entries.len() {
                self.entries.truncate(start);
                let lines = if row.expanded {
                    row.detail.lines().map(str::to_string).collect()
                } else {
                    preview_detail(&row.detail)
                };
                row.detail_count = lines.len();
                for line in lines {
                    let role = if line.starts_with('+') {
                        Role::Add
                    } else if line.starts_with('-') {
                        Role::Delete
                    } else {
                        Role::Thinking
                    };
                    self.push(role, line);
                }
            }
        }
        self.tool = Some(row);
    }

    fn agent_items(&self) -> Vec<PickItem> {
        self.agents
            .iter()
            .map(|(name, about)| PickItem {
                label: name.clone(),
                value: name.clone(),
                desc: mark_current(name == &self.agent, about),
                actions: Vec::new(),
            })
            .collect()
    }

    fn model_items(&self) -> Vec<PickItem> {
        let current = self.cfg.model.clone();
        let default = self.cfg.provider.profile().model.to_string();
        let mut names = vec![current.clone(), default.clone()];
        names.extend(self.available_models.iter().cloned());
        names.extend(load_models(&self.models_path));
        let mut seen: Vec<String> = Vec::new();
        let mut items = Vec::new();
        for name in names {
            if name.is_empty() || seen.contains(&name) {
                continue;
            }
            seen.push(name.clone());
            let about = if name == default { "default" } else { "" };
            items.push(PickItem {
                label: name.clone(),
                desc: mark_current(name == current, about),
                value: name,
                actions: Vec::new(),
            });
        }
        items
    }

    fn theme_items(&self) -> Vec<PickItem> {
        let current = self.palette.theme_name();
        crate::theme::names()
            .into_iter()
            .map(|name| PickItem {
                label: name.to_string(),
                value: name.to_string(),
                desc: mark_current(name == current, ""),
                actions: Vec::new(),
            })
            .collect()
    }

    fn mode_items(&self) -> Vec<PickItem> {
        let current = self.cfg.live_permissions().as_str();
        [
            ("normal", "unlisted commands ask"),
            ("strict", "every action asks"),
            ("yolo", "never asks; deny list holds"),
        ]
        .into_iter()
        .map(|(name, about)| PickItem {
            label: name.into(),
            value: name.into(),
            desc: mark_current(name == current, about),
            actions: Vec::new(),
        })
        .collect()
    }

    /// Set the session's mode, applied before the worker sees it.
    fn set_mode(&mut self, name: &str, tx: &Sender<Cmd>) {
        match crate::config::Permissions::parse(name) {
            Ok(mode) => {
                self.cfg.set_permissions(mode);
                let _ = tx.send(Cmd::SetPermissions(mode));
                self.push(Role::Info, crate::perm::mode_line(mode));
            }
            Err(_) => self.push(Role::Error, format!("unknown mode {name}; use normal, strict, or yolo")),
        }
    }

    fn verbosity_items(&self) -> Vec<PickItem> {
        vec![
            PickItem {
                label: "low".into(),
                value: "low".into(),
                desc: "one summary line".into(),
                actions: Vec::new(),
            },
            PickItem {
                label: "high".into(),
                value: "high".into(),
                desc: "arguments and output".into(),
                actions: Vec::new(),
            },
        ]
    }

    fn provider_items(&self) -> Vec<PickItem> {
        let mut items = Vec::new();
        for provider in [
            crate::providers::Provider::DeepSeek,
            crate::providers::Provider::OpenAi,
            crate::providers::Provider::Anthropic,
            crate::providers::Provider::Gemini,
            crate::providers::Provider::Ollama,
            crate::providers::Provider::Custom,
        ] {
            let name = format!("{provider:?}").to_lowercase();
            let profile = provider.profile();
            let about = format!("{}  {}", profile.base_url, profile.model);
            items.push(PickItem {
                desc: mark_current(name == self.cfg.active_name, &about),
                label: name.clone(),
                value: name.clone(),
                actions: Vec::new(),
            });
        }
        for profile in &self.cfg.named {
            items.push(PickItem {
                label: profile.name.clone(),
                value: profile.name.clone(),
                desc: mark_current(
                    profile.name == self.cfg.active_name,
                    &format!("{}  {}  {}", profile.base, profile.model, profile.dialect),
                ),
                actions: Vec::new(),
            });
        }
        items
    }

    /// The picker for a command argument, the composer prefix it mirrors, and
    /// the text that starts as its filter.
    /// The newest twenty sessions, with their row actions.
    fn session_picker(&self) -> Picker {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut options: Vec<PickItem> = crate::store::Log::list(&self.sessions_dir)
            .into_iter()
            .map(|m| {
                let (label, desc) = crate::store::session_row(&m, now);
                PickItem {
                    label,
                    value: m.id,
                    actions: Vec::new(),
                    desc,
                }
            })
            .collect();
        options.truncate(20);
        spin_rows(&mut options, &self.working_ids(), spinner_frame(self.spinner));
        Picker::new("sessions", options, PickKind::Session)
    }

    /// Sessions whose text mentions `query`, each row ending in a snippet.
    fn search_picker(&self, query: &str) -> Picker {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let items = crate::store::search(&self.sessions_dir, query, 20)
            .into_iter()
            .map(|(mut m, snippet)| {
                m.title = snippet;
                let (label, desc) = crate::store::session_row(&m, now);
                PickItem { label, value: m.id, actions: Vec::new(), desc }
            })
            .collect::<Vec<_>>();
        let mut items = items;
        spin_rows(&mut items, &self.working_ids(), spinner_frame(self.spinner));
        Picker::new(format!("search \"{}\"", query.trim()), items, PickKind::Session)
    }

    /// The value a picker of this kind should open on.
    fn current_value(&self, kind: PickKind) -> String {
        match kind {
            PickKind::Agent => self.agent.clone(),
            PickKind::Model => self.cfg.model.clone(),
            PickKind::Theme => self.palette.theme_name().to_string(),
            PickKind::Verbosity => if self.verbose { "high" } else { "low" }.into(),
            PickKind::Provider => self.cfg.active_name.clone(),
            PickKind::Mode => self.cfg.live_permissions().as_str().into(),
            _ => String::new(),
        }
    }

    /// A value picker that opens with the current entry highlighted.
    fn value_picker(&self, title: impl Into<String>, items: Vec<PickItem>, kind: PickKind) -> Picker {
        let mut picker = Picker::new(title, items, kind);
        let current = self.current_value(kind);
        picker.index = picker.items.iter().position(|i| i.value == current).unwrap_or(0);
        picker
    }

    fn argument_picker(&self, word: &str, rest: &str) -> Option<(Picker, String, String)> {
        let simple = |title: &str, items: Vec<PickItem>, kind: PickKind| {
            (
                self.value_picker(title, items, kind),
                format!("/{word} "),
                rest.to_string(),
            )
        };
        match word {
            "agent" => Some(simple("agent", self.agent_items(), PickKind::Agent)),
            "model" => Some(simple("model", self.model_items(), PickKind::Model)),
            "theme" => Some(simple("theme", self.theme_items(), PickKind::Theme)),
            "mode" => Some(simple("mode", self.mode_items(), PickKind::Mode)),
            "verbosity" => Some(simple(
                "verbosity",
                self.verbosity_items(),
                PickKind::Verbosity,
            )),
            "provider" => {
                let verb = rest.split_whitespace().next().unwrap_or("");
                match verb {
                    "" => Some((
                        self.value_picker("provider", self.provider_items(), PickKind::Provider),
                        "/provider ".into(),
                        String::new(),
                    )),
                    "use" | "edit" | "delete" => Some((
                        Picker::new(
                            format!("provider {verb}"),
                            self.provider_items(),
                            PickKind::Provider,
                        ),
                        format!("/provider {verb} "),
                        rest.strip_prefix(verb).unwrap_or("").trim_start().to_string(),
                    )),
                    _ => None,
                }
            }
            "sessions" => Some((self.session_picker(), "/sessions ".into(), rest.to_string())),
            "cd" if rest.is_empty() => Some((
                Picker::browse("cd", PickKind::Dir, &self.root),
                String::new(),
                String::new(),
            )),
            "image" if rest.is_empty() => Some((
                Picker::browse("image", PickKind::Image, &self.root),
                String::new(),
                String::new(),
            )),
            _ => None,
        }
    }

    fn sync_menu(&mut self) {
        if !self.input.starts_with('/') {
            return;
        }
        let sans = self.input[1..].to_string();
        if let Some((word, rest)) = sans.split_once(' ') {
            // A value list opens after a command that has one. Free-text
            // arguments stay in the composer.
            if let Some((mut picker, prefix, filter)) = self.argument_picker(word, rest) {
                picker.filter = filter;
                if !prefix.is_empty() {
                    picker.prefix = Some(prefix);
                }
                self.pending = Some(Pending::Pick(picker));
            }
            self.dirty = true;
            return;
        }
        let mut picker = Picker::new("commands", command_items(), PickKind::Command);
        picker.filter = sans;
        picker.prefix = Some("/".into());
        self.pending = Some(Pending::Pick(picker));
        self.dirty = true;
    }

    fn mirror_picker(&mut self, picker: &Picker) {
        if let Some(prefix) = &picker.prefix {
            self.input = format!("{prefix}{}", picker.filter);
            self.cursor = self.input.len();
        }
    }

    fn submit(&mut self, tx: &Sender<Cmd>) {
        if self.input.trim().is_empty() {
            return;
        }
        let text = self.input.trim().to_string();
        self.input.clear();
        self.cursor = 0;
        self.hist_pos = None;
        self.follow_tail();

        if let Some(cmd) = text.strip_prefix('/') {
            self.run_command(cmd, tx);
            return;
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
        // Typed means approved, but the deny list still holds.
        let mode = self.cfg.live_permissions();
        if let crate::perm::Decision::Deny(why) =
            crate::perm::classify_command(command, mode, self.cfg.allow_sudo)
        {
            self.push(Role::Error, format!("$ {command}: {why}"));
            return;
        }
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

    /// Apply a model to this session and remember it for the picker.
    fn set_model(&mut self, name: &str, tx: &Sender<Cmd>) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        self.cfg.model = name.to_string();
        self.status.served.clear();
        let _ = tx.send(Cmd::SetModel(name.to_string()));
        record_model(&self.models_path, name);
        self.push(Role::Info, format!("model: {name}"));
    }

    /// Add, list, pop, or delete parked messages.
    fn stash_command(&mut self, rest: &str) {
        if rest.is_empty() {
            if self.stash.is_empty() {
                self.push(Role::Info, "stash is empty");
            } else {
                let lines: Vec<String> = self
                    .stash
                    .iter()
                    .enumerate()
                    .map(|(i, text)| format!("{}. {text}", i + 1))
                    .collect();
                self.push(Role::Info, lines.join("\n"));
            }
            return;
        }
        if rest == "pop all" {
            if self.stash.is_empty() {
                self.push(Role::Info, "stash is empty");
                return;
            }
            let all = std::mem::take(&mut self.stash).join("\n\n");
            self.load_stash(&all);
            self.push(Role::Info, "stash: loaded all");
            return;
        }
        if let Some(n) = rest.strip_prefix("pop").map(str::trim) {
            let index = if n.is_empty() {
                1
            } else {
                match n.parse::<usize>() {
                    Ok(n) if n >= 1 => n,
                    _ => {
                        self.push(Role::Error, format!("stash: no entry {n:?}"));
                        return;
                    }
                }
            };
            if index > self.stash.len() {
                self.push(Role::Error, self.stash_range());
                return;
            }
            let text = self.stash.remove(index - 1);
            self.load_stash(&text);
            self.push(Role::Info, format!("stash: popped {index}"));
            return;
        }
        if let Some(n) = rest.strip_prefix("delete").map(str::trim) {
            let index = match n.parse::<usize>() {
                Ok(n) if n >= 1 => n,
                _ => {
                    self.push(Role::Error, format!("stash: no entry {n:?}"));
                    return;
                }
            };
            if index > self.stash.len() {
                self.push(Role::Error, self.stash_range());
                return;
            }
            self.stash.remove(index - 1);
            self.push(Role::Info, format!("stash: deleted {index}"));
            return;
        }
        self.stash.push(rest.to_string());
        self.push(Role::Info, format!("stash: added ({})", self.stash.len()));
    }

    fn stash_range(&self) -> String {
        if self.stash.is_empty() {
            "stash is empty".into()
        } else {
            format!("stash: pick 1 to {}", self.stash.len())
        }
    }

    /// Put stashed text in the composer, after any draft already there.
    fn load_stash(&mut self, text: &str) {
        if self.input.trim().is_empty() {
            self.input = text.to_string();
        } else {
            self.input.push_str("\n\n");
            self.input.push_str(text);
        }
        self.cursor = self.input.len();
        self.follow_tail();
        self.dirty = true;
    }

    /// List, add, edit, delete, or switch provider profiles.
    fn provider_command(&mut self, rest: &str, tx: &Sender<Cmd>) {
        let mut parts = rest.split_whitespace();
        let verb = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        match verb {
            "" => {
                let mut lines = Vec::new();
                for provider in [
                    crate::providers::Provider::DeepSeek,
                    crate::providers::Provider::OpenAi,
                    crate::providers::Provider::Anthropic,
                    crate::providers::Provider::Gemini,
                    crate::providers::Provider::Ollama,
                    crate::providers::Provider::Custom,
                ] {
                    let profile = provider.profile();
                    let mark = if self.cfg.active_name == format!("{provider:?}").to_lowercase() {
                        "*"
                    } else {
                        " "
                    };
                    lines.push(format!(
                        "{mark}{}  {}  {}",
                        format!("{provider:?}").to_lowercase(),
                        profile.base_url,
                        profile.model
                    ));
                }
                for profile in &self.cfg.named {
                    let mark = if self.cfg.active_name == profile.name {
                        "*"
                    } else {
                        " "
                    };
                    lines.push(format!(
                        "{mark}{}  {}  {}  ({})",
                        profile.name, profile.base, profile.model, profile.dialect
                    ));
                }
                lines.push("add <name> · edit <name> · delete <name> · use <name>".into());
                self.push(Role::Info, lines.join("\n"));
            }
            "add" => {
                if !crate::config::valid_profile_name(name) {
                    self.push(Role::Error, "provider: name must be letters, digits, - or _");
                    return;
                }
                if self.cfg.named.iter().any(|n| n.name == name) {
                    self.push(Role::Error, format!("provider {name} already exists"));
                    return;
                }
                let fields = ["base".to_string(), "model".to_string(), "dialect".to_string(), "key_env".to_string()]
                    .into_iter()
                    .map(|label| (label, String::new()))
                    .collect();
                self.pending = Some(Pending::ProviderForm {
                    name: name.to_string(),
                    fields,
                    at: 0,
                });
                self.dirty = true;
            }
            "edit" => {
                let Some(profile) = self.cfg.named.iter().find(|n| n.name == name).cloned() else {
                    self.push(Role::Error, format!("no provider {name:?}"));
                    return;
                };
                let fields = vec![
                    ("base".to_string(), profile.base),
                    ("model".to_string(), profile.model),
                    ("dialect".to_string(), profile.dialect),
                    ("key_env".to_string(), profile.key_env),
                ];
                self.pending = Some(Pending::ProviderForm {
                    name: name.to_string(),
                    fields,
                    at: 0,
                });
                self.dirty = true;
            }
            "delete" => {
                if name == self.cfg.active_name {
                    self.push(
                        Role::Error,
                        format!("provider {name} is active; use another provider first"),
                    );
                    return;
                }
                let before = self.cfg.named.len();
                self.cfg.named.retain(|n| n.name != name);
                if self.cfg.named.len() == before {
                    self.push(Role::Error, format!("no provider {name:?}"));
                    return;
                }
                match crate::config::save_named(&self.config_path, &self.cfg.named) {
                    Ok(()) => self.push(Role::Info, format!("provider {name} deleted")),
                    Err(e) => self.push(Role::Error, e),
                }
            }
            "use" => match self.set_provider(name, tx) {
                Ok(()) => self.push(Role::Info, format!("provider: {name}")),
                Err(e) => self.push(Role::Error, e),
            },
            other => self.push(
                Role::Error,
                format!("provider: unknown {other:?}; use add, edit, delete, or use"),
            ),
        }
    }

    /// Point the session at a built-in or custom profile.
    fn set_provider(&mut self, name: &str, tx: &Sender<Cmd>) -> Result<(), String> {
        let (dialect, base, model, key_env) = if let Ok(provider) =
            crate::providers::Provider::parse(name)
        {
            let profile = provider.profile();
            (
                name.to_ascii_lowercase(),
                profile.base_url.to_string(),
                profile.model.to_string(),
                String::new(),
            )
        } else {
            let profile = self
                .cfg
                .named
                .iter()
                .find(|n| n.name == name)
                .cloned()
                .ok_or_else(|| format!("no provider {name:?}"))?;
            (
                profile.dialect.clone(),
                profile.base.clone(),
                profile.model.clone(),
                profile.key_env.clone(),
            )
        };
        self.cfg.set_active(name)?;
        self.available_models = crate::providers::Provider::parse(&dialect)
            .map(crate::providers::catalog)
            .unwrap_or_default();
        let _ = tx.send(Cmd::UseProvider {
            name: name.to_string(),
            dialect,
            base,
            model,
            key_env,
        });
        Ok(())
    }

    /// Save a completed provider form, replacing any profile of that name.
    fn finish_provider_form(&mut self, name: String, fields: Vec<(String, String)>) {
        let value = |at: usize| fields.get(at).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
        let (base, model, dialect, key_env) = (value(0), value(1), value(2), value(3));
        if base.is_empty() || model.is_empty() || dialect.is_empty() {
            self.push(Role::Error, "provider: base, model, and dialect are required");
            self.pending = Some(Pending::ProviderForm {
                name,
                fields,
                at: 0,
            });
            self.dirty = true;
            return;
        }
        if crate::providers::Provider::parse(&dialect).is_err() {
            self.push(
                Role::Error,
                "provider: dialect must be deepseek, openai, anthropic, gemini, ollama, or custom",
            );
            self.pending = Some(Pending::ProviderForm {
                name,
                fields,
                at: 2,
            });
            self.dirty = true;
            return;
        }
        let profile = crate::config::NamedProvider {
            name: name.clone(),
            base,
            model,
            dialect,
            key_env,
        };
        match self.cfg.named.iter_mut().find(|n| n.name == name) {
            Some(existing) => *existing = profile,
            None => self.cfg.named.push(profile),
        }
        match crate::config::save_named(&self.config_path, &self.cfg.named) {
            Ok(()) => self.push(Role::Info, format!("provider {name} saved")),
            Err(e) => self.push(Role::Error, e),
        }
        self.dirty = true;
    }

    fn run_command(&mut self, cmd: &str, tx: &Sender<Cmd>) {
        match cmd {
            "help" => {
                let width = self.cols.load(Ordering::Relaxed) as usize;
                self.push(Role::Info, crate::help::commands(width.saturating_sub(2).max(40), true))
            }
            "quit" | "exit" => self.quit = true,
            other if other == "agent" || other.starts_with("agent ") => {
                let rest = other.strip_prefix("agent").unwrap_or("").trim();
                match rest {
                    "" => {
                        let picker = self.value_picker("agent", self.agent_items(), PickKind::Agent);
                        self.pending = Some(Pending::Pick(picker));
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
            "mode" => {
                let picker = self.value_picker("mode", self.mode_items(), PickKind::Mode);
                self.pending = Some(Pending::Pick(picker));
                self.dirty = true;
            }
            other if other.starts_with("mode ") => {
                let name = other["mode ".len()..].trim().to_string();
                self.set_mode(&name, tx);
            }
            "reload" => {
                let _ = tx.send(Cmd::Reload);
                self.push(Role::Info, "reloading instructions and skills…");
            }
            "sessions" => {
                self.pending = Some(Pending::Pick(self.session_picker()));
                self.dirty = true;
            }
            other if other.starts_with("sessions ") => {
                let id = other["sessions".len()..].trim().to_string();
                self.resume(id, tx);
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
                    format!("thinking: {}", if self.cfg.thinking { "on" } else { "off" }),
                );
            }
            "new" => self.new_session(tx),
            "next" => self.go_next(),
            "cwd" => self.push(Role::Info, self.root.display().to_string()),
            "fork" => {
                let _ = tx.send(Cmd::Fork);
            }
            other if other == "theme" || other.starts_with("theme ") => {
                let rest = other.strip_prefix("theme").unwrap_or("").trim();
                if rest.is_empty() {
                    let picker = self.value_picker("theme", self.theme_items(), PickKind::Theme);
                    self.pending = Some(Pending::Pick(picker));
                    self.dirty = true;
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
                    self.pending = Some(Pending::Pick(Picker::browse(
                        "image",
                        PickKind::Image,
                        &self.root,
                    )));
                    self.dirty = true;
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
                    self.pending = Some(Pending::Pick(Picker::browse(
                        "cd",
                        PickKind::Dir,
                        &self.root,
                    )));
                    self.dirty = true;
                } else {
                    match crate::config::resolve_root(Some(raw), &self.root) {
                        // The worker reports the change once it has happened.
                        Ok(path) => {
                            let _ = tx.send(Cmd::SetRoot(path));
                        }
                        Err(e) => self.push(Role::Error, e),
                    }
                }
            }
            other if other == "verbosity" || other.starts_with("verbosity ") => {
                let level = other.strip_prefix("verbosity").unwrap_or("").trim();
                match level {
                    "" => {
                        let mut picker = Picker::new(
                            "verbosity",
                            self.verbosity_items(),
                            PickKind::Verbosity,
                        );
                        picker.index = if self.verbose { 1 } else { 0 };
                        self.pending = Some(Pending::Pick(picker));
                        self.dirty = true;
                    }
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
            "shell" => {
                let _ = tx.send(Cmd::Shell);
            }
            other if other == "model" || other.starts_with("model ") => {
                let rest = other.strip_prefix("model").unwrap_or("").trim();
                if rest.is_empty() {
                    let items = self.model_items();
                    self.pending = Some(Pending::Pick(Picker::new(
                        "model",
                        items,
                        PickKind::Model,
                    )));
                    let _ = tx.send(Cmd::ListModels);
                    self.dirty = true;
                } else {
                    self.set_model(rest, tx);
                }
            }
            other if other == "rename" || other.starts_with("rename ") => {
                let rest = other.strip_prefix("rename").unwrap_or("").trim();
                if rest.is_empty() {
                    self.pending = Some(Pending::RenameCurrent {
                        value: String::new(),
                    });
                    self.dirty = true;
                } else {
                    let _ = tx.send(Cmd::RenameCurrent(rest.to_string()));
                    self.push(Role::Info, format!("renamed to {rest:?}"));
                }
            }
            other if other == "stash" || other.starts_with("stash ") => {
                let rest = other.strip_prefix("stash").unwrap_or("").trim();
                self.stash_command(rest);
            }
            other if other == "provider" || other.starts_with("provider ") => {
                let rest = other.strip_prefix("provider").unwrap_or("").trim();
                if rest.is_empty() {
                    let picker =
                        self.value_picker("provider", self.provider_items(), PickKind::Provider);
                    self.pending = Some(Pending::Pick(picker));
                    self.dirty = true;
                } else {
                    self.provider_command(rest, tx);
                }
            }
            other => self.push(Role::Error, crate::help::unknown(other)),
        }
    }

    fn answer(&mut self, pending: Pending, key: Key, tx: &Sender<Cmd>) {
        match pending {
            Pending::RenameCurrent { mut value } => {
                match key {
                    Key::Esc => {
                        self.push(Role::Info, "rename cancelled");
                        self.dirty = true;
                    }
                    Key::Enter => {
                        let title = value.trim().to_string();
                        if title.is_empty() {
                            self.push(Role::Error, "the title is empty");
                            self.pending = Some(Pending::RenameCurrent { value });
                        } else {
                            let _ = tx.send(Cmd::RenameCurrent(title.clone()));
                            self.push(Role::Info, format!("renamed to {title:?}"));
                        }
                        self.dirty = true;
                    }
                    Key::Char(c) => {
                        value.push(c);
                        self.pending = Some(Pending::RenameCurrent { value });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        value.pop();
                        self.pending = Some(Pending::RenameCurrent { value });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::RenameCurrent { value });
                    }
                }
                return;
            }
            Pending::ProviderForm {
                name,
                mut fields,
                mut at,
            } => {
                match key {
                    Key::Esc => {
                        self.push(Role::Info, "provider cancelled");
                        self.dirty = true;
                    }
                    Key::Enter => {
                        if at + 1 < fields.len() {
                            at += 1;
                            self.pending = Some(Pending::ProviderForm { name, fields, at });
                        } else {
                            self.finish_provider_form(name, fields);
                        }
                        self.dirty = true;
                    }
                    Key::Char(c) => {
                        if let Some(field) = fields.get_mut(at) {
                            field.1.push(c);
                        }
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        if let Some(field) = fields.get_mut(at) {
                            field.1.pop();
                        }
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                    }
                }
                return;
            }
            Pending::Prompt {
                label,
                mut value,
                rename_id,
            } => {
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
                        self.pending = Some(Pending::Prompt {
                            label,
                            value,
                            rename_id,
                        });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        value.pop();
                        self.pending = Some(Pending::Prompt {
                            label,
                            value,
                            rename_id,
                        });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::Prompt {
                            label,
                            value,
                            rename_id,
                        });
                    }
                }
                return;
            }
            Pending::ConfirmDelete { id, picker, .. } => {
                let mut picker = *picker;
                if matches!(key, Key::Char('y') | Key::Char('Y') | Key::Enter) {
                    let _ = tx.send(Cmd::Delete(id.clone()));
                    self.push(Role::Info, format!("deleted {id}"));
                    picker.items.retain(|item| item.value != id);
                    picker.reset();
                }
                // Back to the list either way, so the next pick is one key away.
                self.pending = Some(Pending::Pick(picker));
                self.dirty = true;
                return;
            }
            Pending::Pick(mut picker) => {
                match key {
                    Key::Up => picker.move_by(-1),
                    Key::Down => picker.move_by(1),
                    Key::PageUp => picker.move_by(-(PICK_ROWS as isize)),
                    Key::PageDown => picker.move_by(PICK_ROWS as isize),
                    Key::Backspace => {
                        if picker.prefix.is_some() && picker.filter.is_empty() {
                            if picker.kind == PickKind::Command {
                                // Backspacing the slash leaves command editing.
                                self.input.clear();
                                self.cursor = 0;
                            } else {
                                // Backspacing the space returns to the command.
                                let prefix = picker.prefix.clone().unwrap_or_default();
                                self.input = prefix.trim_end().to_string();
                                self.cursor = self.input.len();
                                self.pending = None;
                                self.sync_menu();
                            }
                            self.dirty = true;
                            return;
                        }
                        picker.filter_pop();
                        self.mirror_picker(&picker);
                    }
                    Key::Char(c)
                        if picker.prefix.is_none()
                            && matches!(picker.kind, PickKind::Dir | PickKind::Image)
                            && matches!(c, '/' | '~' | '.') =>
                    {
                        // A typed path takes over from the browser.
                        self.input.push(c);
                        self.cursor = self.input.len();
                        self.dirty = true;
                        return;
                    }
                    Key::Char(c) if picker.prefix.is_some() => {
                        picker.filter_push(c);
                        self.mirror_picker(&picker);
                        if picker.filter.contains(' ') {
                            // The word is done: open the next value list, or
                            // hand free text such as `add work` to the composer.
                            self.pending = None;
                            self.sync_menu();
                            return;
                        }
                    }
                    Key::Char(c) if picker.filter.is_empty() && picker.action(c).is_some() => {
                        match picker.action(c) {
                            Some("new") => self.new_session_from_picker(tx),
                            Some("rename") => {
                                if let Some(choice) = picker.selected() {
                                    let id = choice.value.clone();
                                    self.pending = Some(Pending::Prompt {
                                        label: format!("new title for {id}"),
                                        value: String::new(),
                                        rename_id: id,
                                    });
                                }
                            }
                            Some("delete") => {
                                if let Some(choice) = picker.selected() {
                                    let id = choice.value.clone();
                                    let title = crate::store::Log::list(&self.sessions_dir)
                                        .into_iter()
                                        .find(|m| m.id == id)
                                        .map(|m| m.title)
                                        .unwrap_or_default();
                                    self.pending = Some(Pending::ConfirmDelete {
                                        id,
                                        title,
                                        picker: Box::new(picker),
                                    });
                                }
                            }
                            _ => {}
                        }
                        self.dirty = true;
                        return;
                    }
                    Key::Char(c) => picker.filter_push(c),
                    Key::Tab if picker.kind == PickKind::Command => {
                        if let Some(choice) = picker.selected() {
                            let word = choice.value.trim_start_matches('/').to_string();
                            let values = self.argument_picker(&word, "").is_some();
                            // Values open their picker, free text gets its
                            // space, and a bare command waits for Enter.
                            self.input = if values || FREE_TEXT.contains(&word.as_str()) {
                                format!("/{word} ")
                            } else {
                                format!("/{word}")
                            };
                            self.cursor = self.input.len();
                            if values {
                                self.sync_menu();
                            }
                        }
                        self.dirty = true;
                        return;
                    }
                    Key::Tab | Key::Right if matches!(picker.kind, PickKind::Dir | PickKind::Image) => {
                        // Tab and Right open; `./` is already open.
                        if let Some(choice) = picker.selected().cloned() {
                            if choice.label.ends_with('/') && choice.label != "./" {
                                picker.current = std::path::PathBuf::from(&choice.value);
                                picker.reload();
                            }
                        }
                    }
                    Key::Left if matches!(picker.kind, PickKind::Dir | PickKind::Image) => {
                        picker.parent();
                    }
                    Key::Enter if picker.kind == PickKind::Command => {
                        match picker.selected() {
                            Some(choice) => {
                                self.input = choice.value.clone();
                                self.cursor = self.input.len();
                                self.submit(tx);
                            }
                            None => {
                                self.pending = Some(Pending::Pick(picker));
                                self.dirty = true;
                            }
                        }
                        return;
                    }
                    Key::Enter => {
                        if picker.kind == PickKind::Session
                            && picker.selected().is_none()
                            && !picker.filter.trim().is_empty()
                        {
                            self.pending = Some(Pending::Pick(self.search_picker(&picker.filter)));
                            self.dirty = true;
                            return;
                        }
                        let Some(choice) = picker.selected().cloned() else {
                            self.pending = Some(Pending::Pick(picker));
                            self.dirty = true;
                            return;
                        };
                        match picker.kind {
                            PickKind::Agent => {
                                self.agent = choice.value.clone();
                                let _ = tx.send(Cmd::SetAgent(choice.value.clone()));
                                self.push(Role::Info, format!("agent: {}", choice.value));
                            }
                            PickKind::Session => self.resume(choice.value, tx),
                            PickKind::Model => {
                                self.set_model(&choice.value, tx);
                            }
                            PickKind::Mode => self.set_mode(&choice.value, tx),
                            PickKind::Keys => {}
                            PickKind::Verbosity => {
                                self.verbose = choice.value == "high";
                                self.push(Role::Info, format!("verbosity: {}", choice.value));
                            }
                            PickKind::Theme => {
                                let _ = crate::theme::save(&choice.value);
                                if let Some(theme) = crate::theme::by_name(&choice.value) {
                                    self.palette = self.palette.with_theme(theme);
                                }
                                // Chosen, so closing must not restore.
                                self.preview_from = None;
                                self.push(Role::Info, format!("theme: {}", choice.value));
                            }
                            PickKind::Provider => match self.set_provider(&choice.value, tx) {
                                Ok(()) => {
                                    self.push(Role::Info, format!("provider: {}", choice.value))
                                }
                                Err(e) => self.push(Role::Error, e),
                            },
                            PickKind::Dir => {
                                // Enter chooses. The typed command keeps the jail
                                // and root-change logic in one place.
                                self.run_command(&format!("cd {}", choice.value), tx);
                                self.dirty = true;
                                return;
                            }
                            PickKind::Command => {}
                            PickKind::Image => {
                                if choice.label.ends_with('/') {
                                    picker.current = std::path::PathBuf::from(&choice.value);
                                    picker.reload();
                                    self.pending = Some(Pending::Pick(picker));
                                    self.dirty = true;
                                    return;
                                }
                                match crate::images::attach(&choice.value, &self.root) {
                                    Ok(image) => {
                                        self.push(Role::Thinking, crate::images::marker(&image));
                                        let _ = tx.send(Cmd::Attach(vec![image]));
                                    }
                                    Err(e) => self.push(Role::Error, e),
                                }
                            }
                        }
                        if picker.prefix.is_some() {
                            self.input.clear();
                            self.cursor = 0;
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
                self.pending = Some(Pending::Pick(picker));
                self.dirty = true;
            }
            Pending::Confirm { what, reply, always } => {
                let answer = match key {
                    Key::Char('y' | 'Y') | Key::Enter => Answer::Yes,
                    Key::Char('s' | 'S') if always => Answer::Session,
                    Key::Char('a' | 'A') if always => Answer::Always,
                    Key::Esc | Key::Char('n' | 'N') => Answer::No,
                    _ => {
                        self.pending = Some(Pending::Confirm { what, reply, always });
                        return;
                    }
                };
                // The tool row carries the result, so approval needs no row of its own.
                if answer == Answer::No {
                    self.push(Role::Info, format!("declined: {what}"));
                }
                let _ = reply.send(answer);
                self.dirty = true;
            }
            Pending::Question {
                q,
                opts,
                reply,
                mode,
                mut typed,
            } => match mode {
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
            },
        }
    }

    /// A frame is due only when a key or event arrived or something visible
    /// changed. Nothing redraws on a timer.
    fn wants_render(&self, redraw: bool) -> bool {
        redraw || self.dirty
    }

    /// The spinner advances at most ten times a second while a tool runs.
    fn spinner_due(&self) -> bool {
        self.tool.as_ref().is_some_and(|row| row.running)
            && self.last_redraw.elapsed() >= Duration::from_millis(100)
    }

    /// Ids of live sessions whose agent is working.
    fn working_ids(&self) -> Vec<String> {
        (0..self.panes.len())
            .filter_map(|i| {
                let (worker, busy) = match &self.panes[i] {
                    None => (self.worker.as_ref(), self.status.streaming),
                    Some(p) => (p.worker.as_ref(), p.status.streaming),
                };
                let id = worker?.2.lock().ok()?.clone();
                (busy && !id.is_empty()).then_some(id)
            })
            .collect()
    }

    /// Repaint the spinners in an open session picker, ten times a second.
    /// True when a row changed.
    fn spin_sessions(&mut self) -> bool {
        if self.last_redraw.elapsed() < Duration::from_millis(100)
            || !matches!(&self.pending, Some(Pending::Pick(p)) if p.kind == PickKind::Session)
        {
            return false;
        }
        let working = self.working_ids();
        if !self.tool.as_ref().is_some_and(|row| row.running) {
            self.spinner += 1;
        }
        let frame = spinner_frame(self.spinner);
        let Some(Pending::Pick(p)) = &mut self.pending else { return false };
        let before: Vec<String> = p.items.iter().map(|i| i.desc.clone()).collect();
        spin_rows(&mut p.items, &working, frame);
        p.items.iter().zip(&before).any(|(i, b)| &i.desc != b)
    }

    /// How long the loop may wait for a key: short while anything moves,
    /// longer while a prompt counts down, long when idle. `quiet` is the time
    /// since the last key or event.
    fn loop_wait(&self, quiet: Duration) -> Duration {
        let streaming = self.status.streaming || self.panes.iter().flatten().any(|p| p.status.streaming);
        if streaming || self.tool.as_ref().is_some_and(|row| row.running) || quiet < Duration::from_secs(2) {
            Duration::from_millis(50)
        } else if self.pending_since.is_some() {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(5)
        }
    }

    /// Advance the running row's glyph in place.
    fn tick_spinner(&mut self) {
        self.spinner += 1;
        let frame = spinner_frame(self.spinner);
        if let Some(row) = &self.tool {
            if row.running {
                if let Some(entry) = self.entries.get_mut(row.index) {
                    entry.text = format!("{frame} {}", row.summary);
                }
            }
        }
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
        let composer = self.composer(cols, rows.saturating_sub(1).max(1));
        // A rule sets the composer apart, unless the screen is too short.
        let rule = usize::from(rows >= 8);
        let body_rows = rows.saturating_sub(1 + rule + composer.len()).max(1);

        let markdown = self.cfg.links.supported();
        // Only entries whose text, role, width, or turn gap changed render again.
        self.drawn.truncate(self.entries.len());
        let mut spoke = false;
        for (i, entry) in self.entries.iter().enumerate() {
            // One blank row opens every turn after the first.
            let gap = entry.role == Role::User && spoke;
            spoke |= entry.role == Role::User;
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (cols, markdown, gap, entry.role as u8, &entry.text).hash(&mut h);
            let key = h.finish();
            if self.drawn.get(i).map(|slot| slot.0) != Some(key) {
                let slot = (key, entry_lines(entry, cols, markdown, gap));
                if i < self.drawn.len() {
                    self.drawn[i] = slot;
                } else {
                    self.drawn.push(slot);
                }
            }
        }
        // A quiet hint until the first message. The status line says where.
        let hint = (Role::Thinking, Line::Text("imp · / for commands".into()));
        let hint = self.entries.is_empty().then_some(&hint);

        let total = self.drawn.iter().map(|slot| slot.1.len()).sum::<usize>() + usize::from(hint.is_some());
        let max_top = total.saturating_sub(body_rows);
        self.body_rows = body_rows;
        self.max_top = max_top;
        if self.follow {
            self.top = max_top;
        }
        self.top = self.top.min(max_top);
        self.unseen = total.saturating_sub(self.top + body_rows);
        let visible: Vec<&(Role, Line)> = self
            .drawn
            .iter()
            .flat_map(|slot| slot.1.iter())
            .chain(hint)
            .skip(self.top)
            .take(body_rows)
            .collect();

        let mut screen = String::from("\x1b[?25l\x1b[?7l\x1b[H");
        for _ in 0..body_rows.saturating_sub(visible.len()) {
            screen.push_str("\x1b[2K\r\n");
        }
        for (role, line) in visible {
            screen.push_str("\x1b[2K");
            match line {
                Line::Text(text) => screen.push_str(&self.marked(*role, &fit(text, cols))),
                Line::Spans(spans) => screen.push_str(&self.paint_spans(*role, spans)),
                Line::Timed(text, time) => {
                    screen.push_str(&self.marked(*role, text));
                    let gap = cols.saturating_sub(text.chars().count() + time.chars().count());
                    screen.push_str(&" ".repeat(gap));
                    screen.push_str(&self.palette.paint(Role::Thinking, time));
                }
            }
            screen.push_str("\r\n");
        }

        if rule == 1 {
            screen.push_str("\x1b[2K");
            // The rule takes the accent while a question waits below it.
            let asking = self.pending.is_some() && !matches!(self.pending, Some(Pending::Pick(_)));
            let tone = if asking {
                Role::Info
            } else if self.yolo() {
                Role::Error
            } else {
                Role::Border
            };
            screen.push_str(&self.palette.paint(tone, &"─".repeat(cols)));
            screen.push_str("\r\n");
        }
        for row in &composer {
            screen.push_str("\x1b[2K");
            screen.push_str(row);
            screen.push_str("\r\n");
        }

        screen.push_str("\x1b[2K\x1b[J");
        screen.push_str(&self.palette.paint(Role::Thinking, &self.status_line(cols)));
        screen.push_str("\x1b[?7h");

        if let Some((overlay, cursor_x, cursor_y)) = self.overlay(cols, rows) {
            screen.push_str("\x1b[?7l");
            screen.push_str(&overlay);
            if matches!(self.pending, Some(Pending::Pick(_))) {
                // Only an open filter takes the cursor.
                screen.push_str(&format!("\x1b[{cursor_y};{cursor_x}H\x1b[?25h"));
            }
            screen.push_str("\x1b[?7h");
        }

        screen
    }

    /// A tool row: the leading `✓` bold in the success colour, `✗` in the
    /// error colour, and the rest in the row's own role.
    fn marked(&self, role: Role, text: &str) -> String {
        let found = text.find(['✓', '✗']);
        let Some(at) = found.filter(|_| self.palette.on && matches!(role, Role::Tool | Role::Error)) else {
            return self.palette.paint(role, text);
        };
        let mark = text[at..].chars().next().unwrap_or('✓');
        let tone = if mark == '✓' { Role::Success } else { Role::Error };
        let rest = &text[at + mark.len_utf8()..];
        let head = if at == 0 { String::new() } else { self.palette.paint(role, &text[..at]) };
        // `paint` ends in a full reset, which also ends the bold.
        format!(
            "{head}\x1b[1m{}{}",
            self.palette.paint(tone, &mark.to_string()),
            self.palette.paint(role, rest)
        )
    }

    /// Paint one row of styled spans, with a hyperlink around each linked run.
    fn paint_spans(&self, role: Role, spans: &[crate::render::Span]) -> String {
        let mut out = String::new();
        for span in spans {
            let colour = if span.style.border {
                Role::Border
            } else if span.style.code {
                Role::Code
            } else if span.link.is_some() {
                Role::Link
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

    /// The confirmation row: question, key chips, and the countdown at the
    /// right edge. Keys are bold accent, labels and countdown muted.
    fn confirm_row(&self, what: &str, always: bool, cols: usize) -> String {
        let chips: &[(&str, &str)] = if always { &ALL_CHIPS } else { &ALL_CHIPS[..2] };
        let keys = chip_text(chips);
        let secs = format!("{}s", self.seconds_left());
        // "▌ " + question + "   " + chips + at least one space + countdown.
        let fixed = 2 + 3 + keys.chars().count() + 1 + secs.len();
        // A labelled row hides the command, so the question names it.
        let shown = self.tool.as_ref().is_none_or(|row| {
            let command = what.split('`').nth(1).unwrap_or(what);
            row.summary.contains(command)
        });
        let ask = if shown {
            verb(what).to_string()
        } else {
            format!("{}?", fit(what, cols.saturating_sub(fixed + 1).max(8)))
        };
        let bell = if self.bell_pending { "\u{7}" } else { "" };
        let used = fixed - 1 + ask.chars().count();
        let p = &self.palette;
        if used + 1 > cols {
            // Too narrow for chips: one plain accent line, cut at the edge.
            let plain = format!("▌ {ask}   {keys} {secs}");
            return format!("{bell}{}", p.paint(Role::Info, &fit(&plain, cols)));
        }
        let mut row = format!("{bell}{}   ", p.paint(Role::Info, &format!("▌ {ask}")));
        row.push_str(&self.paint_chips(chips));
        row.push_str(&" ".repeat(cols - used));
        row.push_str(&p.paint(Role::Thinking, &secs));
        row
    }

    /// Answer chips: key letters bold accent, brackets and labels muted.
    fn paint_chips(&self, chips: &[(&str, &str)]) -> String {
        let p = &self.palette;
        let mut row = String::new();
        for (i, (k, l)) in chips.iter().enumerate() {
            if i > 0 {
                row.push_str("  ");
            }
            let key = p.paint(Role::Info, k);
            let key = if p.on { format!("\x1b[1m{key}") } else { key };
            row.push_str(&format!("{}{key}{}", p.paint(Role::Thinking, "["), p.paint(Role::Thinking, &format!("] {l}"))));
        }
        row
    }

    /// A single row for an open question or confirmation, if any.
    ///
    /// The picker draws as an overlay instead, so it has no composer row.
    fn pending_row(&self, cols: usize) -> Option<String> {
        if let Some(pending) = &self.pending {
            let text = match pending {
                Pending::Pick(_) => return None,
                Pending::Confirm { what, always, .. } => return Some(self.confirm_row(what, *always, cols)),
                Pending::ConfirmDelete { id, title, .. } if title.is_empty() => {
                    format!("delete {id}? y/N")
                }
                Pending::ConfirmDelete { id, title, .. } => {
                    format!("delete \"{}\" ({id})? y/N", title.replace('\n', " "))
                }
                Pending::Prompt { label, value, .. } => format!("{label}: {value}{CARET}"),
                Pending::RenameCurrent { value } => format!("new title: {value}{CARET}"),
                Pending::ProviderForm { name, fields, at } => {
                    let (label, value) = fields
                        .get(*at)
                        .cloned()
                        .unwrap_or_else(|| ("field".into(), String::new()));
                    format!("provider {name} · {label}: {value}{CARET}  (Enter next · Esc cancel)")
                }
                Pending::Question {
                    q,
                    opts,
                    mode,
                    typed,
                    ..
                } => {
                    let mut text = format!("{q}   ");
                    for (i, o) in opts.iter().enumerate() {
                        text.push_str(&format!("[{}] {o}  ", i + 1));
                    }
                    match mode {
                        AskMode::Choose => text.push_str("[e] answer   Esc skip"),
                        AskMode::Custom => text.push_str(&format!("> {typed}{CARET}")),
                    }
                    text
                }
            };
            // A bar marks the row that is waiting on the user.
            let text = if self.bell_pending {
                format!("\u{7}▌ {text}")
            } else {
                format!("▌ {text}")
            };
            // The caret is the composer's block, and keeps the row's colour.
            let painted = self.palette.paint(Role::Info, &fit(&text, cols));
            return Some(painted.replace(CARET, "\x1b[7m \x1b[27m"));
        }
        None
    }

    /// The picker as a centred bordered block, drawn after the normal frame.
    ///
    /// Absolute cursor addressing paints the rectangle without disturbing the
    /// transcript, and the next full frame replaces it, leaving no residue.
    fn overlay(&self, cols: usize, rows: usize) -> Option<(String, usize, usize)> {
        let picker = match &self.pending {
            Some(Pending::Pick(picker)) => picker,
            Some(Pending::ConfirmDelete { picker, .. }) => &**picker,
            _ => return None,
        };
        let width = (cols * 80 / 100).clamp(40, 72).min(cols);
        if width < 4 || rows < 5 {
            return None;
        }
        let inner = width.saturating_sub(2);
        let matches = picker.matches();
        let visible = matches.len().min(PICK_ROWS).max(1);
        let height = visible + 4;
        let body = self.body_rows.clamp(1, rows.saturating_sub(1).max(1));
        let x = (cols.saturating_sub(width)) / 2 + 1;
        let y = (body.saturating_sub(height)) / 2 + 1;

        // An edge with text set into it: `╭─ title ───── 3/40 ─╮`.
        let edge = |l: char, r: char, left: &str, right: &str, tone: Role| {
            let label = fit(left, inner.saturating_sub(4));
            let right = if right.is_empty() { String::new() } else { format!(" {right} ─") };
            let used = label.chars().count() + 3 + right.chars().count();
            let right = if used > inner { String::new() } else { right };
            let fill = inner.saturating_sub(label.chars().count() + 3 + right.chars().count());
            let line = |t: &str| self.palette.paint(Role::Border, t);
            format!(
                "{}{} {}{}",
                line(&format!("{l}─ ")),
                self.palette.paint(tone, &label),
                line(&"─".repeat(fill)),
                line(&format!("{right}{r}"))
            )
        };
        let position = if matches.len() > visible {
            format!("{}/{}", picker.index + 1, matches.len())
        } else {
            String::new()
        };

        let wall = self.palette.paint(Role::Border, "│");
        let title = if matches!(picker.kind, PickKind::Dir | PickKind::Image) {
            // Browsers name where they are, keeping the deepest part.
            let room = inner.saturating_sub(picker.title.chars().count() + position.len() + 9);
            format!("{} {}", picker.title, tail(&tilde(&picker.current), room))
        } else {
            picker.title.clone()
        };
        let mut block: Vec<String> = vec![edge('╭', '╮', &title, &position, Role::Assistant)];
        let filter = format!(" › {}", picker.filter);
        block.push(format!("{wall}{}{wall}", self.boxed(&filter, inner, false)));
        block.push(
            self.palette
                .paint(Role::Border, &format!("├{}┤", "─".repeat(inner))),
        );

        for row in 0..visible {
            let at = picker.offset + row;
            let text = if matches.is_empty() {
                "  no matches".to_string()
            } else {
                match matches.get(at) {
                    Some(&item) => picker_row(picker, at, item, inner),
                    None => String::new(),
                }
            };
            let selected = !matches.is_empty() && at == picker.index;
            block.push(format!(
                "{wall}{}{wall}",
                self.boxed(&text, inner, selected)
            ));
        }
        block.push(edge('╰', '╯', &picker.footer(), "", Role::Thinking));

        let mut out = String::new();
        for (i, line) in block.iter().enumerate() {
            out.push_str(&format!("\x1b[{};{}H{line}", y + i, x));
        }
        let cursor = (x + 1 + filter.chars().count()).min(x + width - 2);
        Some((out, cursor, y + 1))
    }

    /// Fit one row of text to the interior width and pad it with spaces.
    fn boxed(&self, text: &str, inner: usize, selected: bool) -> String {
        let text = fit(text, inner);
        let pad = inner.saturating_sub(text.chars().count());
        let row = format!("{text}{}", " ".repeat(pad));
        if selected {
            self.palette.select(&row)
        } else {
            row
        }
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
        let tone = if self.yolo() { Role::Error } else { Role::User };
        self.palette.paint(tone, prefix) + &fitted.replace(CARET, "\x1b[7m \x1b[0m")
    }

    /// Yolo is shown by colour alone: the prompt glyph and the rule.
    fn yolo(&self) -> bool {
        self.cfg.live_permissions() == crate::config::Permissions::Yolo
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
        let thinking = if self.cfg.thinking { " · think" } else { "" };
        let unseen = if self.unseen > 0 {
            format!(" · ↑ {}", self.unseen)
        } else {
            String::new()
        };
        let money = if self.cfg.provider == crate::providers::Provider::DeepSeek {
            crate::pricing::cost(
                &self.cfg.model,
                crate::pricing::now(),
                self.status.hit,
                self.status.miss,
                self.status.output,
            )
            .map(|c| format!(" · ${c:.2}"))
            .unwrap_or_default()
        } else {
            String::new()
        };
        let model = if self.status.served.is_empty() || self.status.served == self.cfg.model {
            self.cfg.model.clone()
        } else {
            format!("{} ({})", self.cfg.model, self.status.served)
        };
        // With several live sessions, which one this is comes first.
        let place = if self.panes.len() > 1 {
            format!("{}/{} {} · ", self.at + 1, self.panes.len(), fit(&title_of(&self.entries), 20))
        } else {
            String::new()
        };
        let waiting = match self.waiting() {
            0 => String::new(),
            n => format!(" · {n} waiting"),
        };
        let left = format!(
            "{place}{model} · {} tok{money}{unseen}{thinking}{waiting}",
            compact_tokens(self.status.tokens),
        );
        // The directory sits at the right edge, and goes first when narrow.
        let right = tilde(&self.root);
        let gap = cols.saturating_sub(left.chars().count() + right.chars().count());
        if gap >= 2 {
            format!("{left}{}{right}", " ".repeat(gap))
        } else {
            fit(&left, cols)
        }
    }
}

/// A live session's name: its first message, or `new` before one.
fn title_of(entries: &[Entry]) -> String {
    entries
        .iter()
        .find(|e| e.role == Role::User)
        .map(|e| e.text.lines().next().unwrap_or("").to_string())
        .unwrap_or_else(|| "new".into())
}

/// Tool time: `12ms` under a second, `1.4s` from there.
pub fn duration(ms: u128) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
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

const ALL_CHIPS: [(&str, &str); 4] = [("y", "yes"), ("n", "no"), ("s", "session"), ("a", "always")];

fn chip_text(chips: &[(&str, &str)]) -> String {
    chips.iter().map(|(k, l)| format!("[{k}] {l}")).collect::<Vec<_>>().join("  ")
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
/// The one line that reports a new session root.
pub(crate) fn root_report(path: &std::path::Path) -> String {
    format!("cwd: {} (prompt cache reset)", tilde(path))
}

/// A path with the home directory written as `~`.
fn tilde(path: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    match home.as_deref().and_then(|h| path.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

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
        // Leading spaces are layout, as in `/help`; keep them.
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        let mut line = raw[..indent].to_string();
        for word in raw[indent..].split(' ') {
            if line.len() == indent {
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

/// The rows a restored history draws: user and assistant text, and one
/// row per tool call. Reasoning, system, and tool results stay hidden.
fn replay(history: &[crate::api::Message]) -> Vec<(Role, String)> {
    use crate::api::Role as Api;
    let mut rows = Vec::new();
    for m in history {
        match m.role {
            Api::User => rows.push((Role::User, m.content.clone())),
            Api::Assistant => {
                if !m.content.trim().is_empty() {
                    rows.push((Role::Assistant, m.content.clone()));
                }
                for call in &m.tool_calls {
                    let args = serde_json::from_str(&call.arguments).unwrap_or_default();
                    // ponytail: no mark; the log does not say if the call failed.
                    let summary = crate::agent::describe(&call.name, &args);
                    rows.push((Role::Tool, format!("▸ {summary}")));
                }
            }
            _ => {}
        }
    }
    rows
}

/// Draw a restored session and its context size.
fn send_replay(tx: &Sender<Ev>, history: &[crate::api::Message]) {
    let _ = tx.send(Ev::Replay(replay(history)));
    let _ = tx.send(Ev::Usage {
        tokens: crate::api::estimate_tokens(history),
        hit: 0,
        miss: 0,
        output: 0,
        model: None,
    });
}

/// Run the interface until the user quits. Returns the final session id.
/// A worker thread for one session: its command sender, its event receiver,
/// and the id of the session it is logging to.
type Worker = (Sender<Cmd>, Receiver<Ev>, std::sync::Arc<std::sync::Mutex<String>>);

fn spawn_worker(session: Session, sessions_dir: std::path::PathBuf) -> Worker {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (ev_tx, ev_rx) = channel::<Ev>();
    let mut worker_session = session;
    if worker_session
        .history
        .iter()
        .any(|m| m.role != crate::api::Role::System)
    {
        send_replay(&ev_tx, &worker_session.history);
    }
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
                            waited: Duration::ZERO,
                        };
                        let applied = worker_session.switch_agent(agent);
                        let _ = ev_tx.send(Ev::Info(format!("agent: {applied}")));
                        let _ = &mut hooks;
                    } else {
                        let names: Vec<String> = worker_session
                            .roster
                            .iter()
                            .map(|a| a.name.clone())
                            .collect();
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
                        let _ = ev_tx.send(Ev::Info(root_report(&path)));
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
                            send_replay(&ev_tx, &worker_session.history);
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
                Ok(Cmd::SetPermissions(mode)) => {
                    worker_session.set_permissions(mode);
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
                Ok(Cmd::ListModels) => {
                    let provider = worker_session.cfg.provider;
                    let base = worker_session.cfg.base_url.clone();
                    let key = worker_session.cfg.api_key.clone();
                    let models =
                        crate::providers::list_models(provider, &base, &key)
                            .unwrap_or_else(|_| crate::providers::catalog(provider));
                    let _ = ev_tx.send(Ev::Models(models));
                }
                Ok(Cmd::SetModel(name)) => worker_session.cfg.model = name,
                Ok(Cmd::Shell) => {
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
                    let mut hooks = WorkerHooks {
                        tx: ev_tx.clone(),
                        waited: Duration::ZERO,
                    };
                    let code = hooks.run_attached(&shell);
                    let message = match code {
                        Some(0) | None => "shell closed".to_string(),
                        Some(code) => format!("shell exited {code}"),
                    };
                    let _ = ev_tx.send(Ev::Info(message));
                }
                Ok(Cmd::RenameCurrent(title)) => {
                    let result = match &worker_session.log {
                        Some(log) => log.retitle(&title),
                        None => Err("nothing to rename yet".into()),
                    };
                    match result {
                        Ok(()) => {
                            let _ = ev_tx.send(Ev::Info(format!("renamed to {title:?}")));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                }
                Ok(Cmd::UseProvider {
                    name,
                    dialect,
                    base,
                    model,
                    key_env,
                }) => match crate::providers::Provider::parse(&dialect) {
                    Ok(provider) => {
                        worker_session.cfg.provider = provider;
                        worker_session.cfg.base_url = base;
                        worker_session.cfg.model = model;
                        worker_session.cfg.active_name = name.clone();
                        worker_session.cfg.key_env = if key_env.is_empty() {
                            None
                        } else {
                            Some(key_env)
                        };
                        worker_session.cfg.api_key.clear();
                        if let Err(e) = worker_session
                            .cfg
                            .resolve_key_env(|k| std::env::var(k).ok())
                        {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                        let _ = ev_tx.send(Ev::Info(format!("provider: {name}")));
                    }
                    Err(e) => {
                        let _ = ev_tx.send(Ev::Error(e));
                    }
                },
                Ok(Cmd::Quit) | Err(_) => break,
            }
        }
    });
    (cmd_tx, ev_rx, session_id)
}

pub fn run(session: Session, cfg: Config, fresh: Fresh) -> Result<Option<String>, String> {
    install_guards();
    MOUSE.store(cfg.mouse, Ordering::Relaxed);
    let _raw = RawMode::enter();
    if std::env::var_os("IMP_FORCE_PANIC").is_some() {
        panic!("forced panic for the terminal-guard test");
    }
    // Only a first run needs the guess; a saved theme or COLORFGBG answers it.
    let hinted = std::env::var("COLORFGBG")
        .ok()
        .and_then(|v| crate::theme::light_from_colorfgbg(&v))
        .is_some();
    if crate::theme::saved().is_none() && !hinted {
        crate::theme::set_light(query_light().unwrap_or(false));
    }
    let mut app = App::new(
        cfg,
        session.cancel.clone(),
        session.cols.clone(),
        session.tools.root.clone(),
        session.agent.name.clone(),
    );

    let sessions_dir = crate::config::dirs_state().join("imp").join("sessions");
    app.worker = Some(spawn_worker(session, sessions_dir));
    app.fresh = Some(fresh);

    // Key reader.
    let (key_tx, key_rx) = channel::<Vec<u8>>();
    let key_tx_resize = key_tx.clone();
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

    wake_on_resize(key_tx_resize);

    let mut pending_bytes: Vec<u8> = Vec::new();
    let mut busy = Instant::now();

    app.render();
    while !app.quit {
        let mut redraw = false;
        match key_rx.recv_timeout(app.loop_wait(busy.elapsed())) {
            Ok(bytes) => {
                pending_bytes.extend(bytes);
                for key in decode(&mut pending_bytes) {
                    // The session in view can change with any key.
                    let Some(tx) = app.worker.as_ref().map(|w| w.0.clone()) else { break };
                    app.handle_key(key, &tx);
                }
                redraw = true;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Ten frames a second is enough for a spinner. Repainting faster
                // flickers under software terminal renderers.
                if app.spin_sessions() {
                    redraw = true;
                }
                if app.spinner_due() {
                    app.tick_spinner();
                    redraw = true;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if app.poll() {
            redraw = true;
        }
        if redraw {
            busy = Instant::now();
        }
        app.expire();

        // A resize has to be noticed without repainting on a timer.
        let (cols, rows) = size();
        if app.resized(cols, rows) {
            app.dirty = true;
        }

        if app.wants_render(redraw) {
            app.render();
        }
    }

    let id = app
        .worker
        .as_ref()
        .and_then(|w| w.2.lock().ok().map(|slot| slot.clone()))
        .filter(|id| !id.is_empty());
    // Stop every worker and wait up to a second for them to end, so each
    // session drops and its background jobs stop before imp exits.
    let mut workers = vec![(&app.cancel, &app.worker)];
    workers.extend(app.panes.iter().flatten().map(|p| (&p.cancel, &p.worker)));
    for (cancel, worker) in &workers {
        cancel.store(true, Ordering::Relaxed);
        if let Some((tx, _, _)) = worker {
            let _ = tx.send(Cmd::Quit);
        }
    }
    let deadline = Instant::now() + Duration::from_secs(1);
    for (_, worker) in workers {
        if let Some((_, rx, _)) = worker {
            // Disconnected means the worker ended and dropped its session.
            while rx.recv_timeout(deadline.saturating_duration_since(Instant::now())).is_ok() {}
        }
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_reporting_follows_the_setting() {
        assert!(enter_sequence(true).ends_with(MOUSE_ON));
        assert!(!enter_sequence(false).contains("?1000h"));
        assert!(ALT_OFF.starts_with(MOUSE_OFF) && RESTORE.starts_with(MOUSE_OFF));
        let mut cfg = Config::default();
        assert!(cfg.mouse, "mouse should default on");
        cfg.set("mouse", "off").unwrap();
        assert!(!cfg.mouse);
    }

    #[test]
    fn session_keys_decode_raw_and_disambiguated() {
        let mut buf = b"\x05\x0f\x0e\x10\x1b[101;5u\x1b[111;5u\x1b[110;5u\x1b[112;5u".to_vec();
        let keys = decode(&mut buf);
        let letters: Vec<char> = keys
            .iter()
            .map(|k| match k {
                Key::Ctrl(c) => *c,
                other => panic!("not a control key: {other:?}"),
            })
            .collect();
        assert_eq!(letters, ['e', 'o', 'n', 'p', 'e', 'o', 'n', 'p']);
    }

    #[test]
    fn decodes_the_mouse_wheel_and_ignores_clicks() {
        let mut buf = b"\x1b[<64;10;5M\x1b[<65;10;5M\x1b[<0;3;4M\x1b[<0;3;4m\x1b[<68;1;1M".to_vec();
        let keys = decode(&mut buf);
        assert!(matches!(keys[0], Key::WheelUp), "{keys:?}");
        assert!(matches!(keys[1], Key::WheelDown), "{keys:?}");
        assert!(matches!(keys[2], Key::Unknown) && matches!(keys[3], Key::Unknown), "{keys:?}");
        assert!(matches!(keys[4], Key::WheelUp), "shift+wheel: {keys:?}");
        assert!(buf.is_empty());
    }

    #[test]
    fn the_wheel_scrolls_the_transcript_and_moves_a_picker() {
        let mut a = long_app();
        let (tx, _rx) = channel();
        let tail = a.frame(60, 8);
        a.handle_key(Key::WheelUp, &tx);
        assert!(!a.follow, "wheel up did not detach");
        assert_ne!(a.frame(60, 8), tail);
        for _ in 0..20 {
            a.handle_key(Key::WheelDown, &tx);
            a.frame(60, 8);
        }
        assert!(a.follow, "scrolling to the end did not resume following");

        let mut b = app();
        open_many(&mut b, 5);
        b.handle_key(Key::WheelDown, &tx);
        assert_eq!(open_picker(&b).index, 1);
        b.handle_key(Key::WheelUp, &tx);
        assert_eq!(open_picker(&b).index, 0);
    }

    #[test]
    fn a_click_does_not_answer_a_prompt() {
        let mut a = picker();
        let (tx, rx) = channel();
        a.handle_key(Key::Char('d'), &tx);
        a.handle_key(Key::Unknown, &tx);
        a.handle_key(Key::WheelDown, &tx);
        assert!(matches!(a.pending, Some(Pending::ConfirmDelete { .. })), "the question closed");
        assert!(rx.try_recv().is_err());
    }

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
        app_at(std::env::temp_dir())
    }

    /// An app that opens real live sessions, logging under a scratch directory.
    fn multi_app(name: &str) -> App {
        let dir = std::env::temp_dir().join(format!("imp-panes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut a = app_at(dir.clone());
        a.sessions_dir = dir.join("sessions");
        let out = dir.join("out");
        a.fresh = Some(Box::new(move |root: &std::path::Path| {
            let mut cfg = Config::default();
            cfg.color = ColorMode::Never;
            Session::new(
                cfg,
                root.to_path_buf(),
                out.clone(),
                String::new(),
                crate::store::Agent::builtin(),
                Vec::new(),
                std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
            )
        }));
        a
    }

    #[test]
    fn new_opens_a_second_live_session() {
        let mut a = multi_app("second");
        let (tx, _rx) = channel();
        a.push(Role::User, "first question");
        a.status.streaming = true;
        a.run_command("new", &tx);
        assert_eq!((a.panes.len(), a.at), (2, 1));
        assert!(!a.status.streaming, "the new session inherited the turn");
        assert!(!a.entries.iter().any(|e| e.text == "first question"));
        let first = a.panes[0].as_ref().unwrap();
        assert!(first.status.streaming, "the first session stopped");
        assert!(first.entries.iter().any(|e| e.text == "first question"));
        // An event for the first lands there, not in view.
        a.apply_to(0, Ev::Info("from the first".into()));
        assert!(!a.entries.iter().any(|e| e.text == "from the first"));
        assert!(a.panes[0].as_ref().unwrap().entries.iter().any(|e| e.text == "from the first"));
    }

    #[test]
    fn resuming_opens_another_live_session() {
        let mut a = multi_app("resume");
        let (tx, _rx) = channel();
        let log = crate::store::Log::create(&a.sessions_dir, &a.root, "m");
        log.append(&crate::api::Message::user("stored question"));
        a.push(Role::User, "live question");
        a.run_command(&format!("sessions {}", log.id), &tx);
        assert_eq!((a.panes.len(), a.at), (2, 1));
        assert!(a.panes[0].as_ref().unwrap().entries.iter().any(|e| e.text == "live question"));
        // The worker replays the stored turn into the new pane.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !a.entries.iter().any(|e| e.text.contains("stored question")) && Instant::now() < deadline {
            a.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(a.entries.iter().any(|e| e.text.contains("stored question")), "no replay");
    }

    #[test]
    fn enter_with_no_title_match_searches_session_text() {
        let mut a = multi_app("search");
        let (tx, _rx) = channel();
        let log = crate::store::Log::create(&a.sessions_dir, &a.root, "m");
        log.append(&crate::api::Message::user("weather please"));
        log.append(&crate::api::Message::assistant("wttr.in says rain"));
        a.run_command("sessions", &tx);
        for c in "WTTR".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
        let Some(Pending::Pick(p)) = &a.pending else { panic!("picker closed") };
        assert_eq!(p.title, "search \"WTTR\"");
        assert!(p.filter.is_empty());
        assert_eq!(p.items.len(), 1);
        assert!(p.items[0].label.contains("wttr.in"), "{}", p.items[0].label);
        a.handle_key(Key::Enter, &tx);
        assert_eq!((a.panes.len(), a.at), (2, 1), "the hit did not open");

        a.run_command("sessions", &tx);
        for c in "zzqx".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
        let Some(Pending::Pick(p)) = &a.pending else { panic!("picker closed") };
        assert!(p.items.is_empty());
        a.handle_key(Key::Enter, &tx);
        assert!(matches!(a.pending, Some(Pending::Pick(_))), "Enter on no hits closed it");
        assert_eq!(a.panes.len(), 2);
    }

    #[test]
    fn working_sessions_spin_in_the_session_picker() {
        let mut a = multi_app("spin");
        let (tx, _rx) = channel();
        a.run_command("new", &tx);
        let id = a.worker.as_ref().unwrap().2.lock().unwrap().clone();
        // A session is listed once its log has a line.
        std::fs::write(a.sessions_dir.join(format!("{id}.jsonl")), "").unwrap();
        a.status.streaming = true;
        a.run_command("sessions", &tx);
        let desc = |a: &App| {
            let Some(Pending::Pick(p)) = &a.pending else { panic!("no picker") };
            p.items.iter().find(|i| i.value == id).expect("row").desc.clone()
        };
        let first = desc(&a);
        assert!(SPINNER.contains(&first.chars().next().unwrap()), "{first}");
        a.last_redraw = Instant::now() - Duration::from_millis(200);
        assert!(a.spin_sessions());
        assert_ne!(desc(&a), first, "the frame did not advance");
        a.status.streaming = false;
        a.last_redraw = Instant::now() - Duration::from_millis(200);
        assert!(a.spin_sessions());
        assert!(desc(&a).ends_with("msgs") && desc(&a).starts_with("  "), "{}", desc(&a));
        assert_eq!(desc(&a).chars().count(), first.chars().count(), "the cell changed width");
        a.last_redraw = Instant::now() - Duration::from_millis(200);
        assert!(!a.spin_sessions(), "an idle picker repainted");
    }

    #[test]
    fn ctrl_n_and_p_switch_sessions_and_keep_drafts() {
        let mut a = multi_app("switch");
        let (tx, _rx) = channel();
        a.handle_key(Key::Ctrl('n'), &tx);
        assert_eq!(a.panes.len(), 1, "one session must not switch");
        assert!(a.status_line(80).starts_with(&a.cfg.model), "{}", a.status_line(80));
        a.push(Role::User, "fix the parser");
        a.input = "draft one".into();
        a.cursor = a.input.len();
        a.run_command("new", &tx);
        a.input = "draft two".into();
        assert!(a.status_line(80).starts_with("2/2 new · "), "{}", a.status_line(80));
        a.handle_key(Key::Ctrl('n'), &tx);
        assert_eq!((a.at, a.input.as_str()), (0, "draft one"));
        assert!(a.status_line(80).starts_with("1/2 fix the parser · "), "{}", a.status_line(80));
        a.handle_key(Key::Ctrl('p'), &tx);
        assert_eq!((a.at, a.input.as_str()), (1, "draft two"));
    }

    /// Three live sessions: 0 waits for approval, 1 has replied, 2 is in view.
    fn inbox_app(name: &str) -> (App, Receiver<Answer>) {
        let mut a = multi_app(name);
        let (tx, _rx) = channel();
        a.push(Role::User, "run the tests");
        a.status.streaming = true;
        a.run_command("new", &tx);
        a.push(Role::User, "summarise the log");
        a.run_command("new", &tx);
        a.push(Role::User, "in view");
        let (reply, answers) = channel();
        a.apply_to(0, Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        a.apply_to(1, Ev::Token("All done.".into()));
        a.apply_to(1, Ev::Done(None));
        (a, answers)
    }

    #[test]
    fn ctrl_o_walks_the_queue_oldest_first() {
        let (mut a, answers) = inbox_app("queue");
        let (tx, _rx) = channel();
        assert_eq!(a.queue(), vec![0, 1], "the approval came first");
        a.handle_key(Key::Ctrl('o'), &tx);
        assert_eq!(a.at, 0);
        assert!(matches!(a.pending, Some(Pending::Confirm { .. })));
        // Moving on leaves the approval open and sends it to the back.
        a.handle_key(Key::Ctrl('o'), &tx);
        assert_eq!(a.at, 1, "the reply was skipped");
        assert!(answers.try_recv().is_err(), "moving on answered the approval");
        assert_eq!(a.queue(), vec![0]);
        // The reply in view counts as read once the user moves on.
        a.handle_key(Key::Ctrl('o'), &tx);
        assert_eq!(a.at, 0);
        assert!(!a.panes[1].as_ref().unwrap().waits(), "the read reply still waits");
        a.handle_key(Key::Char('y'), &tx);
        assert_eq!(answers.try_recv().ok(), Some(Answer::Yes));
        a.handle_key(Key::Ctrl('o'), &tx);
        assert_eq!(a.at, 0, "moved with nothing waiting");
        assert_eq!(a.entries.last().unwrap().text, "nothing waiting");
        a.run_command("next", &tx);
        assert_eq!(a.entries.last().unwrap().text, "nothing waiting", "/next differs from Ctrl+O");
    }

    #[test]
    fn handing_off_moves_to_the_next_waiting_session() {
        let (mut a, _answers) = inbox_app("handoff");
        let (tx, _rx) = channel();
        // Typing never switches, even with sessions waiting.
        a.handle_key(Key::Char('h'), &tx);
        a.handle_key(Key::Char('i'), &tx);
        assert_eq!(a.at, 2, "typing switched sessions");
        a.handle_key(Key::Enter, &tx);
        assert_eq!(a.at, 0, "sending did not move on");
        assert!(a.entries.last().unwrap().text.starts_with("→ 1/3 run the tests · needs you"), "{}", a.entries.last().unwrap().text);
        // Answering hands off too: the reply is next.
        a.handle_key(Key::Char('y'), &tx);
        assert_eq!(a.at, 1);
        assert_eq!(a.entries.last().unwrap().text, "→ 2/3 summarise the log · replied");

        let (mut off, _answers) = inbox_app("handoff-off");
        off.cfg.auto_next = false;
        off.handle_key(Key::Char('x'), &tx);
        off.handle_key(Key::Enter, &tx);
        assert_eq!(off.at, 2, "auto_next=off still moved");
    }

    #[test]
    fn a_prompt_in_view_survives_ctrl_o() {
        let mut a = multi_app("survive");
        let (tx, _rx) = channel();
        let (reply, answers) = channel();
        a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
        a.handle_key(Key::Ctrl('o'), &tx);
        assert!(matches!(a.pending, Some(Pending::Confirm { .. })), "the prompt was lost");
        a.handle_key(Key::Char('y'), &tx);
        assert_eq!(answers.try_recv().ok(), Some(Answer::Yes));
    }

    #[test]
    fn sessions_out_of_view_ask_for_attention() {
        let (mut a, answers) = inbox_app("attention");
        assert!(a.status_line(120).contains(" · 2 waiting"), "{}", a.status_line(120));
        assert!(a.bell_pending, "no bell for a session out of view");
        assert!(!a.panes[0].as_ref().unwrap().bell_pending, "the bell would ring again on switching");
        // Out of view, the prompt never expires, however old.
        a.panes[0].as_mut().unwrap().pending_since = Some(Instant::now() - CONFIRM_WAIT * 10);
        a.expire();
        assert!(answers.try_recv().is_err(), "a prompt out of view expired");
        // In view, its deadline starts from now.
        a.show(0);
        assert!(a.seconds_left() >= 59, "{}", a.seconds_left());
        a.expire();
        assert!(answers.try_recv().is_err(), "expired as soon as it was shown");
        assert!(a.status_line(120).contains(" · 1 waiting"), "{}", a.status_line(120));
    }

    #[test]
    fn question_mark_opens_the_keys_card_only_from_an_empty_composer() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.handle_key(Key::Char('?'), &tx);
        let Some(Pending::Pick(p)) = &a.pending else { panic!("no keys card") };
        assert_eq!(p.kind, PickKind::Keys);
        assert!(p.items.iter().any(|i| i.label == "Ctrl+O"));
        assert_eq!(p.footer(), "? or Esc close");
        assert!(a.input.is_empty(), "the ? reached the composer");
        a.handle_key(Key::Char('?'), &tx);
        assert!(a.pending.is_none(), "? did not close the card");
        for c in "why".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Char('?'), &tx);
        assert_eq!(a.input, "why?");
        assert!(a.pending.is_none());
    }

    #[test]
    fn modes_stay_per_session() {
        let mut a = multi_app("modes");
        let (tx, _rx) = channel();
        a.run_command("new", &tx);
        a.run_command("mode yolo", &tx);
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);
        assert_eq!(
            a.panes[0].as_ref().unwrap().cfg.live_permissions(),
            crate::config::Permissions::Normal,
            "/mode leaked into another session"
        );
    }

    #[test]
    fn a_fifth_session_closes_the_oldest_idle_one() {
        let mut a = multi_app("room");
        let (tx, _rx) = channel();
        for _ in 0..3 {
            a.status.streaming = true;
            a.run_command("new", &tx);
        }
        assert_eq!(a.panes.len(), 4);
        // Session 0 is busy, 1 and 2 busy, the one in view is idle.
        a.run_command("new", &tx);
        assert_eq!(a.panes.len(), 4, "no room was made");
        assert!(a.panes.iter().flatten().all(|p| p.status.streaming), "an idle session survived");

        a.status.streaming = true;
        a.run_command("new", &tx);
        assert_eq!(a.panes.len(), 4);
        let last = a.entries.last().unwrap();
        assert_eq!(last.role, Role::Error);
        assert!(last.text.starts_with("all four sessions are busy"), "{}", last.text);
    }

    fn app_at(root: std::path::PathBuf) -> App {
        let mut cfg = Config::default();
        cfg.color = ColorMode::Never;
        App::new(
            cfg,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
            root,
            "default".into(),
        )
    }

    /// A unique empty directory for a browser fixture.
    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("imp-picker-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.canonicalize().unwrap()
    }

    fn colour_app() -> App {
        let mut cfg = Config::default();
        cfg.color = ColorMode::Always;
        let mut a = App::new(
            cfg,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
            std::env::temp_dir(),
            "default".into(),
        );
        // Pin the palette: the saved theme on this machine must not matter.
        a.palette = Palette {
            on: true,
            allowed: true,
            truecolor: true,
            theme: crate::theme::by_name("default").unwrap(),
        };
        a
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
        assert!(
            a.hist_pos.is_none(),
            "Up recalled history from the last row"
        );
    }

    #[test]
    fn a_long_prompt_keeps_its_answer_hint() {
        let mut a = app();
        let long = format!("run `{}`", "cat openspec/changes/thing.md; ".repeat(8));
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm { what: long, reply, always: false });
        let out = strip(&a.frame(80, 12));
        let row = out
            .lines()
            .find(|l| l.contains("[y] yes  [n] no"))
            .unwrap_or_else(|| panic!("no answer hint in {out:?}"));
        assert!(row.contains('s'), "no seconds left: {row:?}");
        assert!(
            strip(row).chars().filter(|c| *c != '\u{7}').count() <= 80,
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
            always: false,
        });
        a.status.streaming = true;
        let (tx, _cmds) = channel();
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none(), "the prompt stayed open");
        assert_eq!(
            rx.try_recv().ok(),
            Some(Answer::No),
            "the prompt was not refused"
        );
        assert!(!a.quit, "Esc quit the whole interface");
    }

    #[test]
    fn a_new_prompt_rings_once() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `ls`".into(),
            reply,
            always: false,
        });
        assert!(a.frame(80, 12).contains('\u{7}'), "no bell on a new prompt");
        a.bell_pending = false;
        assert!(
            !a.frame(80, 12).contains('\u{7}'),
            "the same prompt rings twice"
        );
    }

    #[test]
    fn a_labelled_call_names_its_command_when_confirming() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "count files".into(),
            args: "{}".into(),
        });
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm { what: "run `find . | wc -l`".into(), reply, always: false });
        let frame = strip(&a.frame(80, 20));
        assert!(frame.contains("run `find . | wc -l`?   [y] yes  [n] no"), "{frame:?}");
    }

    #[test]
    fn the_confirm_row_counts_down() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `rm -rf build`".into(),
            reply,
            always: false,
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
        let frame = strip(frame);
        let row = frame.lines().find(|l| l.contains("[y] yes")).expect("the prompt row");
        let last = row.trim_end().rsplit(' ').next().expect("countdown");
        last.strip_suffix('s').expect("seconds marker").parse().expect("seconds")
    }

    #[test]
    fn the_countdown_sits_at_the_right_edge() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        let frame = strip(&a.frame(80, 12));
        let row = frame.lines().find(|l| l.contains("[a] always")).expect("chips");
        assert!(row.contains("[y] yes  [n] no  [s] session  [a] always"), "{row}");
        assert_eq!(row.chars().filter(|c| *c != '\u{7}').count(), 80, "countdown not at the edge: {row:?}");
        assert!(row.ends_with("0s") || row.ends_with("9s"), "{row:?}");
    }

    #[test]
    fn always_is_answered_only_where_offered() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        assert!(strip(&a.frame(80, 12)).contains("[y] yes  [n] no  [s] session  [a] always"));
        a.handle_key(Key::Char('a'), &tx);
        assert_eq!(rx.try_recv().ok(), Some(Answer::Always));

        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        a.handle_key(Key::Char('s'), &tx);
        assert_eq!(rx.try_recv().ok(), Some(Answer::Session));

        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `a && b`".into(), reply, always: false });
        let frame = strip(&a.frame(80, 12));
        assert!(frame.contains("[y] yes  [n] no") && !frame.contains("[s]"), "{frame}");
        a.handle_key(Key::Char('s'), &tx);
        a.handle_key(Key::Char('a'), &tx);
        assert!(rx.try_recv().is_err(), "a answered a prompt that did not offer it");
        assert!(matches!(a.pending, Some(Pending::Confirm { .. })));
    }

    #[test]
    fn an_unanswered_prompt_in_view_stops_the_turn() {
        let mut a = app();
        let (reply, answers) = channel();
        a.status.streaming = true;
        a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
        a.expire();
        assert!(a.pending.is_some(), "expired at once");
        a.pending_since = Some(Instant::now() - CONFIRM_WAIT);
        a.expire();
        assert_eq!(answers.try_recv().ok(), Some(Answer::No));
        assert!(a.pending.is_none() && !a.status.streaming);
        assert!(a.cancel.load(Ordering::Relaxed), "the turn was not cancelled");
        assert!(a.entries.last().unwrap().text.starts_with("no answer within 60s"));
    }

    #[test]
    fn a_stop_event_ends_the_turn() {
        let mut a = app();
        let (reply, _rx) = channel();
        a.apply(Ev::Confirm {
            what: "run `ls`".into(),
            reply,
            always: false,
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
    fn a_rule_sets_the_composer_apart_and_turns_are_spaced() {
        let mut a = app();
        a.push(Role::User, "first");
        a.push(Role::Assistant, "reply");
        a.push(Role::User, "second");
        let frame = strip(&a.frame(80, 24));
        let rows: Vec<&str> = frame.split("\r\n").collect();
        let rule = rows.iter().position(|r| r.starts_with("────")).expect("no rule");
        assert!(rows[rule + 1].starts_with("> "), "the composer does not follow: {rows:?}");
        assert_eq!(rows[rule].chars().count(), 80);
        let first = rows.iter().position(|r| *r == "> first").unwrap();
        let second = rows.iter().position(|r| *r == "> second").unwrap();
        assert_eq!(rows[second - 1], "", "no gap before the second turn: {rows:?}");
        assert_ne!(rows[first - 1], "> first");
        assert!(rows[..first].iter().all(|r| r.is_empty()), "{rows:?}");

        let short = strip(&a.frame(80, 7));
        assert!(!short.contains("────"), "a rule on a 7-row screen: {short:?}");
    }

    #[test]
    fn an_empty_transcript_shows_a_welcome_row() {
        let mut a = app();
        let frame = strip(&a.frame(120, 24));
        assert!(frame.contains("imp · / for commands"), "{frame:?}");
        assert_eq!(frame.matches(tilde(&a.root).as_str()).count(), 1, "{frame:?}");
        a.push(Role::User, "hello");
        assert!(!strip(&a.frame(80, 24)).contains("/ for commands"));
        let narrow = strip(&app().frame(30, 24));
        assert!(narrow.contains("/ for commands"), "{narrow:?}");
    }

    #[test]
    fn the_status_line_ends_with_the_directory_when_it_fits() {
        let a = app();
        let root = tilde(&a.root);
        let wide = a.status_line(100);
        assert!(wide.ends_with(&root), "{wide}");
        assert_eq!(wide.chars().count(), 100, "{wide}");
        assert!(wide.starts_with("deepseek-flash · "), "{wide}");
        let narrow = a.status_line(40);
        assert!(!narrow.contains(&root), "{narrow}");
        assert!(narrow.starts_with("deepseek-flash"), "{narrow}");
    }

    #[test]
    fn a_waiting_prompt_has_an_accent_bar_and_rule() {
        let mut a = colour_app();
        let (tx, _rx) = channel();
        let idle = a.frame(80, 20);
        a.run_command("rename", &tx);
        let asking = a.frame(80, 20);
        let accent = a.palette.paint(Role::Info, "─");
        let accent = &accent[..accent.find('─').unwrap()];
        assert!(asking.contains(&format!("{accent}{}", "─".repeat(80))), "{asking:?}");
        assert!(!idle.contains(&format!("{accent}{}", "─".repeat(80))), "{idle:?}");
        assert!(strip(&asking).contains("▌ new title: "), "{asking:?}");

        let mut plain = app();
        plain.run_command("rename", &tx);
        let frame = plain.frame(80, 20);
        assert!(frame.contains("▌ new title: "), "{frame:?}");
        assert!(!frame.contains("\x1b[38"), "colour with colour off: {frame:?}");
    }

    #[test]
    fn yolo_paints_the_prompt_and_rule_in_the_error_colour() {
        let mut a = colour_app();
        let (tx, _rx) = channel();
        let error = a.palette.paint(Role::Error, "> ");
        let rule = a.palette.paint(Role::Error, &"─".repeat(80));
        let normal = a.frame(80, 20);
        assert!(!normal.contains(&error) && !normal.contains(&rule), "{normal:?}");
        a.run_command("mode yolo", &tx);
        let yolo = a.frame(80, 20);
        assert!(yolo.contains(&error), "glyph: {yolo:?}");
        assert!(yolo.contains(&rule), "rule: {yolo:?}");
        a.run_command("mode normal", &tx);
        assert!(!a.frame(80, 20).contains(&error), "yolo colour stayed");

        let mut plain = app();
        let before = plain.frame(80, 20);
        plain.cfg.set_permissions(crate::config::Permissions::Yolo);
        assert_eq!(plain.frame(80, 20), before, "no-colour frame changed");
    }

    #[test]
    fn escapes_in_replies_and_tools_never_reach_the_terminal() {
        let mut a = app();
        a.push(Role::Assistant, "copy \u{1b}]52;c;aGk=\u{7}done");
        a.push(Role::Tool, "✓ run `ls`\n\u{1b}[31mred\u{1b}[0m");
        let frame = a.frame(80, 12);
        assert!(!frame.contains("]52;"), "{frame:?}");
        assert!(!frame.contains("[31m"), "{frame:?}");
        assert!(strip(&frame).contains("copy done"), "{frame:?}");
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
    fn frames_reuse_rows_until_an_entry_changes() {
        let mut a = app();
        for i in 0..50 {
            a.push(Role::User, format!("question {i}"));
            a.push(Role::Assistant, format!("**answer** {i}\n\n- one\n- two"));
        }
        let full = a.frame(60, 20);
        assert_eq!(a.frame(60, 20), full, "a cached frame differs");
        a.drawn.clear();
        assert_eq!(a.frame(60, 20), full, "a fresh frame differs");

        // A marked slot survives while its entry is unchanged.
        a.drawn[0].1 = vec![(Role::Info, Line::Text("cached".into()))];
        a.frame(60, 20);
        assert!(matches!(&a.drawn[0].1[0].1, Line::Text(t) if t == "cached"), "re-rendered");
        a.entries[0].text.push('!');
        a.frame(60, 20);
        assert!(matches!(&a.drawn[0].1[0].1, Line::Text(t) if t == "> question 0!"), "stale row kept");
        // A new width renders everything again.
        a.drawn[2].1.clear();
        a.frame(50, 20);
        assert!(!a.drawn[2].1.is_empty());
    }

    #[test]
    #[ignore]
    fn time_a_long_transcript() {
        let mut a = app();
        for i in 0..400 {
            a.push(Role::User, format!("question {i} about the parser"));
            a.push(Role::Assistant, "**answer** with `code` and a list\n\n- one\n- two\n".repeat(3));
        }
        a.frame(80, 24);
        let t = Instant::now();
        for _ in 0..50 {
            a.entries.last_mut().unwrap().text.push_str(" more");
            a.frame(80, 24);
        }
        eprintln!("{:?} per streamed frame", t.elapsed() / 50);
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
        assert!(
            !out.replace("\r\n", "").contains('\n'),
            "bare line feed in frame"
        );
    }

    #[test]
    fn ctrl_e_expands_the_last_tool_call() {
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
            detail: String::new(),
            ms: 4,
        });
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('o'), &tx);
        assert!(!a.entries[0].text.contains("args:"), "Ctrl+O still expands");
        assert_eq!(a.entries.last().unwrap().text, "nothing waiting");
        a.handle_key(Key::Ctrl('e'), &tx);
        let text = a.entries[0].text.clone();
        assert!(text.contains("args:"), "{text}");
        assert!(text.contains("ls -la"), "{text}");
        assert!(text.contains("total 8"), "{text}");
        a.handle_key(Key::Ctrl('e'), &tx);
        assert!(!a.entries[0].text.contains("args:"), "it did not collapse");
    }

    #[test]
    fn inline_code_is_bold_in_the_code_colour() {
        let mut a = colour_app();
        a.push(Role::Assistant, "could not `bash` it");
        let out = a.frame(60, 12);
        assert!(out.contains("\x1b[1m"), "not bold: {out:?}");
        let code = a.palette.paint(Role::Code, "bash");
        assert!(out.contains(&format!("\x1b[1m{code}")), "not the code colour: {out:?}");
    }

    #[test]
    fn the_picker_highlight_has_a_background() {
        let mut a = colour_app();
        open_many(&mut a, 3);
        let out = a.frame(100, 30);
        let [r, g, b] = a.palette.theme.rgb(crate::theme::Slot::Select);
        let bg = format!("\x1b[48;2;{r};{g};{b}m");
        let at = out.find(&bg).expect("no select background");
        assert!(out[at..].contains("▸ profile 0"), "{out:?}");
        let border = a.palette.paint(Role::Border, "│");
        assert!(out.contains(&border), "the frame is not in the border colour");
    }

    #[test]
    fn the_theme_picker_previews_and_esc_restores() {
        let mut a = colour_app();
        let (tx, _rx) = channel();
        let start = a.palette.theme_name();
        a.run_command("theme", &tx);
        a.sync_preview();
        a.handle_key(Key::Down, &tx);
        let shown = {
            let p = open_picker(&a);
            p.items[p.index].value.clone()
        };
        assert_ne!(shown, start);
        assert_eq!(a.palette.theme_name(), shown, "the highlight was not previewed");
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none());
        assert_eq!(a.palette.theme_name(), start, "Esc did not restore");
    }

    #[test]
    fn a_chosen_theme_stays_after_the_picker_closes() {
        // The Enter path saves to the state directory, so this drives the
        // same steps without touching it: choose, clear the start, close.
        let mut a = colour_app();
        let (tx, _rx) = channel();
        a.run_command("theme", &tx);
        a.handle_key(Key::Down, &tx);
        let chosen = a.palette.theme_name();
        a.preview_from = None;
        a.pending = None;
        a.sync_preview();
        assert_eq!(a.palette.theme_name(), chosen, "closing undid the choice");
    }

    #[test]
    fn mono_previews_without_colour_and_back() {
        let mut a = colour_app();
        let mono = crate::theme::by_name("mono").unwrap();
        let off = a.palette.with_theme(mono);
        assert!(!off.on);
        assert!(off.with_theme(crate::theme::by_name("nord").unwrap()).on);
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
        assert!(
            erase < text,
            "erase below runs after the status text: {row:?}"
        );
    }
    #[test]
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
        assert!(
            !out.contains("row 1\r\n"),
            "composer ignored its row budget"
        );
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
        let plain = Palette {
            on: true,
            allowed: true,
            truecolor: false,
            theme: &crate::theme::THEMES[0],
        };
        assert!(plain.paint(Role::Error, "x").contains("38;5;"));
        let rgb = Palette {
            on: true,
            allowed: true,
            truecolor: true,
            theme: &crate::theme::THEMES[0],
        };
        assert!(rgb.paint(Role::Error, "x").contains("38;2;"));
        let off = Palette {
            on: false,
            allowed: false,
            truecolor: true,
            theme: &crate::theme::THEMES[0],
        };
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
        let cost =
            crate::pricing::cost("deepseek-flash", crate::pricing::now(), 900, 100, 50).unwrap();
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
        let total =
            crate::pricing::cost("deepseek-flash", crate::pricing::now(), 1300, 300, 75).unwrap();
        let line = a.status_line(120);
        assert!(line.contains(&format!("${total:.2}")), "{line}");
    }

    #[test]
    fn an_unknown_model_shows_tokens_but_no_cost() {
        let mut a = app();
        a.cfg.set_provider(crate::providers::Provider::OpenAi);
        a.status.tokens = 100;
        a.status.hit = 90;
        a.status.miss = 10;
        a.status.output = 5;
        let line = a.status_line(120);
        assert!(line.contains("100 tok"), "{line}");
        assert!(!line.contains('$'), "{line}");
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
        assert!(
            a.wants_render(false),
            "a dirty screen did not queue a frame"
        );
    }

    #[test]
    fn the_spinner_runs_at_ten_frames_a_second() {
        let mut a = app();
        a.status.streaming = true;
        a.last_redraw = Instant::now() - Duration::from_millis(101);
        assert!(!a.spinner_due(), "a turn without a tool ticked the spinner");

        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `sleep 1`".into(),
            args: "{}".into(),
        });
        a.last_redraw = Instant::now();
        assert!(!a.spinner_due(), "spinner ran faster than 100 ms");
        a.last_redraw = Instant::now() - Duration::from_millis(101);
        assert!(a.spinner_due(), "spinner stalled after 100 ms");

        let before = a.entries[0].text.clone();
        a.tick_spinner();
        assert_ne!(a.entries[0].text, before, "the row did not advance");
        assert!(a.entries[0].text.starts_with(spinner_frame(a.spinner)));

        a.apply(Ev::ToolEnd {
            ok: true,
            preview: "done".into(),
            output: "done".into(),
            detail: String::new(),
            ms: 7,
        });
        assert!(!a.spinner_due(), "a finished tool kept ticking");
    }

    #[test]
    fn the_status_line_has_no_spinner() {
        let mut a = app();
        a.status.streaming = true;
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `sleep 1`".into(),
            args: "{}".into(),
        });
        let line = a.status_line(80);
        for frame in SPINNER {
            assert!(
                !line.contains(frame),
                "status line carried {frame:?}: {line:?}"
            );
        }
        assert!(line.starts_with("deepseek-flash"), "{line:?}");
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
    fn mode_changes_the_shared_mode_immediately() {
        let mut a = app();
        a.status.streaming = true;
        let (tx, rx) = channel();
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Normal);
        a.run_command("mode yolo", &tx);
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);
        assert!(matches!(rx.try_recv(), Ok(Cmd::SetPermissions(crate::config::Permissions::Yolo))));
        a.run_command("mode strict", &tx);
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Strict);
        assert!(a.entries.last().unwrap().text.starts_with("mode: strict"), "no mode line");
        // Queued commands each carry their target; the last wins.
        a.run_command("mode yolo", &tx);
        a.run_command("mode normal", &tx);
        let sent: Vec<_> = rx.try_iter().filter_map(|c| match c {
            Cmd::SetPermissions(m) => Some(m),
            _ => None,
        }).collect();
        assert_eq!(sent.last(), Some(&crate::config::Permissions::Normal));
    }

    #[test]
    fn mode_rejects_an_unknown_name() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("mode wild", &tx);
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Normal);
        assert!(rx.try_recv().is_err(), "an unknown mode reached the worker");
        let last = a.entries.last().unwrap();
        assert_eq!(last.role, Role::Error);
        assert_eq!(last.text, "unknown mode wild; use normal, strict, or yolo");
    }

    #[test]
    fn mode_picker_opens_on_the_current_mode() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.cfg.set_permissions(crate::config::Permissions::Strict);
        a.run_command("mode", &tx);
        let Some(Pending::Pick(p)) = &a.pending else { panic!("no picker") };
        assert_eq!(p.items.iter().map(|i| i.value.as_str()).collect::<Vec<_>>(), ["normal", "strict", "yolo"]);
        assert_eq!(p.items[p.index].value, "strict");
        assert!(p.items[1].desc.starts_with("current"), "{}", p.items[1].desc);
        assert!(!p.items[0].desc.contains("current"));
        a.handle_key(Key::Down, &tx);
        a.handle_key(Key::Enter, &tx);
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);
        // Tab on a partial command opens the values, like other value commands.
        for c in "/mo".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Tab, &tx);
        let Some(Pending::Pick(p)) = &a.pending else { panic!("Tab did not open the mode values") };
        assert_eq!(p.title, "mode");
    }

    #[test]
    fn shell_command_sends_the_attached_runner() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("shell", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Shell)));
    }

    #[test]
    fn model_command_sets_the_session_and_records_recents() {
        let dir = scratch("models");
        let config = dir.join("config");
        let mut a = app();
        a.config_path = config.clone();
        a.models_path = config.with_file_name("models");
        let (tx, rx) = channel();
        a.run_command("model deepseek-v4-pro", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::SetModel(name)) if name == "deepseek-v4-pro"));
        assert_eq!(a.cfg.model, "deepseek-v4-pro");

        a.run_command("model other", &tx);
        assert_eq!(
            load_models(&config.with_file_name("models")),
            vec!["other", "deepseek-v4-pro"]
        );

        a.run_command("model", &tx);
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Model);
        let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels[0], "other");
        assert_eq!(picker.items[0].desc, "current");
        assert_eq!(labels[1], "deepseek-flash");
        assert_eq!(picker.items[1].desc, "default");
        assert!(labels.contains(&"deepseek-v4-pro"), "{labels:?}");
        assert!(labels.iter().all(|l| !l.contains("(current)")), "{labels:?}");
    }

    #[test]
    fn pickers_mark_the_current_entry_one_way() {
        let mut a = app();
        open_agent_picker(&mut a);
        let picker = open_picker(&a);
        let current = &picker.items[picker.index];
        assert_eq!(current.desc, "current · default profile");
        assert_eq!(picker.items[1].desc, "reviewer profile");
        a.pending = None;
        let (tx, _rx) = channel();
        a.run_command("theme", &tx);
        let picker = open_picker(&a);
        let marked: Vec<&str> = picker
            .items
            .iter()
            .filter(|i| i.desc.starts_with("current"))
            .map(|i| i.label.as_str())
            .collect();
        assert_eq!(marked, vec![a.palette.theme_name()]);
    }

    #[test]
    fn the_model_picker_lists_the_provider_catalog() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("model", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::ListModels)));
        let picker = open_picker(&a);
        let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.len() >= 3, "only {} model rows", labels.len());
        assert!(labels.contains(&"deepseek-chat"), "{labels:?}");
        assert!(labels.contains(&"deepseek-reasoner"), "{labels:?}");
    }

    #[test]
    fn a_live_model_list_replaces_the_catalog_rows() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("model", &tx);
        if let Some(Pending::Pick(picker)) = &mut a.pending {
            picker.filter = "be".into();
        }
        a.apply(Ev::Models(vec!["alpha".into(), "beta".into()]));
        let picker = open_picker(&a);
        let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"alpha"), "{labels:?}");
        assert!(labels.contains(&"beta"), "{labels:?}");
        assert!(!labels.contains(&"deepseek-chat"), "catalog row stayed: {labels:?}");
        assert_eq!(picker.filter, "be", "the filter was lost");
    }

    #[test]
    fn switching_provider_swaps_the_model_catalog() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("provider use openai", &tx);
        assert!(a.available_models.iter().any(|m| m == "gpt-4.1"));
        a.run_command("model", &tx);
        let labels: Vec<&str> = open_picker(&a)
            .items
            .iter()
            .map(|i| i.label.as_str())
            .collect();
        assert!(labels.iter().any(|l| l.starts_with("gpt-4.1")), "{labels:?}");
    }

    #[test]
    fn stash_adds_lists_pops_deletes_and_pops_all() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("stash check the tests", &tx);
        a.run_command("stash write the docs", &tx);
        a.run_command("stash", &tx);
        let listed = &a.entries.last().unwrap().text;
        assert!(listed.contains("1. check the tests"), "{listed}");
        assert!(listed.contains("2. write the docs"), "{listed}");

        a.run_command("stash pop 1", &tx);
        assert_eq!(a.input, "check the tests");
        assert_eq!(a.stash.len(), 1);

        a.run_command("stash delete 1", &tx);
        assert!(a.stash.is_empty());
        assert_eq!(a.input, "check the tests", "delete touched the composer");

        a.run_command("stash first", &tx);
        a.run_command("stash second", &tx);
        a.run_command("stash pop all", &tx);
        assert_eq!(a.input, "check the tests\n\nfirst\n\nsecond");
        assert!(a.stash.is_empty());

        a.run_command("stash pop 9", &tx);
        assert_eq!(a.entries.last().unwrap().role, Role::Error);
    }

    #[test]
    fn rename_command_prompts_or_sends_the_title() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("rename fix the parser", &tx);
        assert!(
            matches!(rx.try_recv(), Ok(Cmd::RenameCurrent(title)) if title == "fix the parser")
        );

        a.run_command("rename", &tx);
        assert!(matches!(a.pending, Some(Pending::RenameCurrent { .. })));
        for c in "new title".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::RenameCurrent(title)) if title == "new title"));
    }

    fn provider_app() -> (App, std::path::PathBuf) {
        let dir = scratch("providers");
        let config = dir.join("config");
        std::fs::write(&config, "# keep me\nmodel = old\n").unwrap();
        let mut a = app();
        a.config_path = config.clone();
        (a, config)
    }

    #[test]
    fn provider_add_edits_the_managed_block() {
        let (mut a, config) = provider_app();
        let (tx, _rx) = channel();
        a.run_command("provider add work", &tx);
        assert!(matches!(a.pending, Some(Pending::ProviderForm { .. })));
        for key in [
            "http://box:8080/v1",
            "local-coder",
            "openai",
            "WORK_KEY",
        ] {
            for c in key.chars() {
                a.handle_key(Key::Char(c), &tx);
            }
            a.handle_key(Key::Enter, &tx);
        }
        assert!(a.pending.is_none(), "the form stayed open");
        let text = std::fs::read_to_string(&config).unwrap();
        assert!(text.starts_with("# keep me\nmodel = old\n"), "{text}");
        assert!(text.contains("provider.work.base = http://box:8080/v1"), "{text}");
        assert!(text.contains("provider.work.dialect = openai"), "{text}");
        assert_eq!(a.cfg.named.len(), 1);

        a.run_command("provider edit work", &tx);
        for _ in 0..2 {
            a.handle_key(Key::Enter, &tx);
        }
        for _ in 0.."openai".len() {
            a.handle_key(Key::Backspace, &tx);
        }
        for c in "ollama".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Enter, &tx);
        a.handle_key(Key::Enter, &tx);
        let text = std::fs::read_to_string(&config).unwrap();
        assert!(text.contains("provider.work.dialect = ollama"), "{text}");

        a.run_command("provider delete work", &tx);
        let text = std::fs::read_to_string(&config).unwrap();
        assert!(!text.contains("provider.work"), "{text}");
        assert!(text.contains("# keep me"), "{text}");
    }

    #[test]
    fn provider_use_reaches_the_worker_and_delete_refuses_the_active_one() {
        let (mut a, _config) = provider_app();
        let (tx, rx) = channel();
        a.run_command("provider add work", &tx);
        for key in ["http://box/v1", "m", "openai", "K"] {
            for c in key.chars() {
                a.handle_key(Key::Char(c), &tx);
            }
            a.handle_key(Key::Enter, &tx);
        }

        a.run_command("provider use work", &tx);
        match rx.try_recv() {
            Ok(Cmd::UseProvider {
                name,
                dialect,
                base,
                model,
                key_env,
            }) => {
                assert_eq!(name, "work");
                assert_eq!(dialect, "openai");
                assert_eq!(base, "http://box/v1");
                assert_eq!(model, "m");
                assert_eq!(key_env, "K");
            }
            other => panic!("no use command: {}", other.is_ok()),
        }
        assert_eq!(a.cfg.active_name, "work");

        a.run_command("provider delete work", &tx);
        assert_eq!(a.entries.last().unwrap().role, Role::Error);
        assert_eq!(a.cfg.named.len(), 1, "the active profile was deleted");
    }

    #[test]
    fn a_value_picker_opens_after_the_command_and_space() {
        let mut a = app();
        let (tx, _rx) = channel();
        for c in "/theme ".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Theme);
        assert_eq!(picker.prefix.as_deref(), Some("/theme "));
        assert!(picker.filter.is_empty());

        for c in "gru".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        let picker = open_picker(&a);
        assert_eq!(picker.filter, "gru");
        assert_eq!(a.input, "/theme gru", "the composer did not mirror");
        let labels: Vec<&str> = picker
            .matches()
            .iter()
            .map(|&i| picker.items[i].label.as_str())
            .collect();
        assert_eq!(labels, vec!["gruvbox-dark", "gruvbox-light"], "{labels:?}");

        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none());
        assert_eq!(a.input, "/theme gru", "Esc discarded the typed text");
    }

    #[test]
    fn bare_theme_and_provider_open_pickers_on_the_current_entry() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("theme", &tx);
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Theme);
        let current = a.palette.theme_name();
        assert_eq!(picker.items[picker.index].value, current);
        a.pending = None;
        a.run_command("provider", &tx);
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Provider);
        assert_eq!(picker.items[picker.index].value, a.cfg.active_name);
    }

    #[test]
    fn a_space_in_a_value_filter_hands_the_line_back() {
        let mut a = app();
        a.config_path = scratch("providers-space").join("config");
        let (tx, _rx) = channel();
        for c in "/provider add work".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        assert!(a.pending.is_none(), "a picker kept the free text");
        assert_eq!(a.input, "/provider add work");
        a.handle_key(Key::Enter, &tx);
        assert!(
            matches!(a.pending, Some(Pending::ProviderForm { .. })),
            "Enter did not start the provider form"
        );
    }

    #[test]
    fn a_value_picker_applies_and_clears_the_line() {
        let mut a = app();
        let (tx, _rx) = channel();
        for c in "/verbosity ".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.handle_key(Key::Char('h'), &tx);
        a.handle_key(Key::Enter, &tx);
        assert!(a.verbose, "Enter did not apply high verbosity");
        assert!(a.pending.is_none());
        assert!(a.input.is_empty(), "the command line stayed: {:?}", a.input);
    }

    #[test]
    fn backspace_from_a_value_picker_returns_to_the_command() {
        let mut a = app();
        let (tx, _rx) = channel();
        for c in "/model ".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        assert_eq!(open_picker(&a).kind, PickKind::Model);
        a.handle_key(Key::Backspace, &tx);
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Command);
        assert_eq!(picker.filter, "model");
        assert_eq!(a.input, "/model");
    }

    #[test]
    fn free_text_arguments_keep_the_composer() {
        let mut a = app();
        let (tx, _rx) = channel();
        for c in "/stash remember the tests".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        assert!(a.pending.is_none(), "a stash draft opened a picker");
        assert_eq!(a.input, "/stash remember the tests");
    }

    #[test]
    fn a_typed_cd_path_takes_over_from_the_browser() {
        let mut a = app();
        let (tx, _rx) = channel();
        for c in "/cd ".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        assert_eq!(open_picker(&a).kind, PickKind::Dir);
        a.handle_key(Key::Char('/'), &tx);
        assert!(a.pending.is_none(), "the browser kept the slash");
        assert_eq!(a.input, "/cd /");
    }

    #[test]
    fn a_long_label_gives_way_to_the_right_column() {
        let items = ["short", &"a very long session title ".repeat(4)]
            .iter()
            .enumerate()
            .map(|(i, t)| PickItem {
                label: t.to_string(),
                value: i.to_string(),
                actions: Vec::new(),
                desc: format!("{} msgs", 10 + i * 50),
            })
            .collect();
        let picker = Picker::new("sessions", items, PickKind::Session);
        let short = picker_row(&picker, 0, 0, 50);
        let long = picker_row(&picker, 1, 1, 50);
        assert!(long.contains('…') && long.ends_with("60 msgs"), "{long}");
        assert!(long.chars().count() <= 50, "{long}");
        assert_eq!(short.find("10 msgs"), long.find("60 msgs"), "{short}\n{long}");
    }

    #[test]
    fn command_rows_show_aligned_descriptions() {
        let picker = Picker::new("commands", command_items(), PickKind::Command);
        let row = |label: &str| {
            let at = picker
                .items
                .iter()
                .position(|item| item.label == label)
                .unwrap_or_else(|| panic!("no row {label}"));
            picker_row(&picker, at, at, 70)
        };
        let compact = row("/compact");
        let help = row("/help");
        assert!(compact.contains("compact the conversation now"), "{compact}");
        assert!(help.contains("show keys and commands"), "{help}");
        let desc_col = |line: &str, desc: &str| {
            line[..line.find(desc).unwrap()].chars().count()
        };
        assert_eq!(
            desc_col(&compact, "compact the conversation now"),
            desc_col(&help, "show keys and commands"),
            "descriptions are not aligned:\n{compact}\n{help}"
        );
        assert!(help.contains("▸ /help"), "no marker: {help}");
    }

    #[test]
    fn the_picker_shows_its_filter_cursor() {
        let mut a = app();
        let (tx, _rx) = channel();
        let idle = a.frame(80, 20);
        assert!(!idle.contains("\x1b[?25h"), "idle frame showed a cursor");
        a.handle_key(Key::Char('/'), &tx);
        let open = a.frame(80, 20);
        assert!(open.contains("\x1b[?25h"), "the filter has no cursor");
        a.handle_key(Key::Esc, &tx);
        let closed = a.frame(80, 20);
        assert!(!closed.contains("\x1b[?25h"), "the cursor stayed after closing");
    }

    #[test]
    fn the_directory_browser_stops_at_the_filesystem_root() {
        let root = scratch("fsroot");
        let mut picker = Picker::browse("cd", PickKind::Dir, &root);
        picker.current = std::path::PathBuf::from("/");
        assert!(!picker.parent(), "Left moved above /");
        assert_eq!(picker.current, std::path::PathBuf::from("/"));
        picker.reload();
        assert!(picker.items.iter().all(|i| i.label != "../"), "/ has a parent row");
    }

    #[test]
    fn browsers_name_their_directory() {
        let base = scratch("title");
        let root = base.join("work");
        std::fs::create_dir_all(root.join("src")).unwrap();
        let mut a = app_at(root.clone());
        let (tx, _rx) = channel();
        a.run_command("cd", &tx);
        highlight(&mut a, &tx, "src/");
        a.handle_key(Key::Tab, &tx);
        let frame = strip(&a.frame(200, 30));
        let start = frame.find("╭─ cd ").expect("title");
        let top = &frame[start..start + frame[start..].find('╮').unwrap()];
        assert!(top.contains("/work/src "), "{top}");
        assert!(top.chars().count() <= 72, "{top}");
        assert_eq!(tilde(std::path::Path::new(&std::env::var("HOME").unwrap())), "~");
    }

    #[test]
    fn the_listing_starts_with_this_folder_and_its_parent() {
        let base = scratch("up");
        let root = base.join("work");
        std::fs::create_dir_all(root.join("leaf")).unwrap();
        let mut a = app_at(root.clone());
        let (tx, rx) = channel();
        a.run_command("cd", &tx);
        let labels: Vec<String> = open_picker(&a).items.iter().map(|i| i.label.clone()).collect();
        assert_eq!(labels, vec!["./", "../", "leaf/"]);
        assert!(open_picker(&a).footer().starts_with("Enter choose · Tab/→ open · ← up"));

        // Tab on `../` opens the parent; Tab on `./` stays put.
        a.handle_key(Key::Down, &tx);
        a.handle_key(Key::Tab, &tx);
        let above = base.canonicalize().unwrap();
        assert_eq!(open_picker(&a).current, above, "Tab did not open the parent");
        a.handle_key(Key::Tab, &tx);
        assert_eq!(open_picker(&a).current, above, "Tab on ./ moved");

        // A leaf folder: open it, then Enter on `./` chooses it.
        highlight(&mut a, &tx, "work/");
        a.handle_key(Key::Right, &tx);
        highlight(&mut a, &tx, "leaf/");
        a.handle_key(Key::Right, &tx);
        let labels: Vec<String> = open_picker(&a).items.iter().map(|i| i.label.clone()).collect();
        assert_eq!(labels, vec!["./", "../"]);
        a.handle_key(Key::Enter, &tx);
        match rx.try_recv() {
            Ok(Cmd::SetRoot(path)) => assert_eq!(path, root.join("leaf")),
            _ => panic!("Enter on ./ did not choose the folder"),
        }
        assert!(a.pending.is_none());

        a.run_command("image", &tx);
        assert!(open_picker(&a).items.iter().all(|i| i.label != "../" && i.label != "./"));
    }

    #[test]
    fn cwd_commands_report_and_change_the_root() {
        let mut a = app();
        let (tx, rx) = channel();
        assert_eq!(a.root, std::env::temp_dir(), "constructor root ignored");

        a.run_command("cwd", &tx);
        assert_eq!(a.entries.last().unwrap().text, a.root.display().to_string());

        a.run_command("cd", &tx);
        assert_eq!(
            open_picker(&a).kind,
            PickKind::Dir,
            "bare cd opened no browser"
        );
        let (tx, rx) = channel();
        a.run_command("cd", &tx);
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none(), "Esc left the browser open");
        assert!(rx.try_recv().is_err(), "Esc changed the root");

        let before = a.entries.len();
        a.run_command("cd .", &tx);
        match rx.try_recv() {
            Ok(Cmd::SetRoot(path)) => {
                assert!(path.is_dir());
                // The worker's single report is the only cwd line.
                a.apply(Ev::Info(root_report(&path)));
            }
            _ => panic!("cd did not send SetRoot"),
        }
        let lines: Vec<&str> = a.entries[before..]
            .iter()
            .filter(|e| e.text.starts_with("cwd:"))
            .map(|e| e.text.as_str())
            .collect();
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].ends_with("(prompt cache reset)"), "{lines:?}");
        let home = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Projects");
        assert_eq!(root_report(&home), "cwd: ~/Projects (prompt cache reset)");

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

    fn highlight(a: &mut App, tx: &Sender<Cmd>, label: &str) {
        let at = {
            let picker = open_picker(a);
            picker
                .matches()
                .iter()
                .position(|&i| picker.items[i].label == label)
                .unwrap_or_else(|| panic!("no row {label:?}"))
        };
        for _ in 0..at {
            a.handle_key(Key::Down, tx);
        }
    }

    #[test]
    fn directory_browser_navigates_selects_and_climbs() {
        let base = scratch("dir");
        let root = base.join("work");
        std::fs::create_dir_all(root.join("src").join("nested")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::create_dir_all(base.join("sibling")).unwrap();
        std::fs::write(root.join("notes.txt"), "x").unwrap();

        let mut a = app_at(root.clone());
        let (tx, rx) = channel();
        a.run_command("cd", &tx);
        {
            let picker = open_picker(&a);
            let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
            assert!(labels.contains(&"src/"), "{labels:?}");
            assert!(labels.contains(&"docs/"), "{labels:?}");
            assert!(!labels.contains(&"notes.txt"), "a file was listed");
        }

        highlight(&mut a, &tx, "src/");
        a.handle_key(Key::Tab, &tx);
        {
            let picker = open_picker(&a);
            assert_eq!(picker.current, root.join("src"), "Tab did not open");
            assert!(picker.items.iter().any(|i| i.label == "nested/"));
            assert!(!picker.items.iter().any(|i| i.label == "docs/"));
        }
        assert_eq!(a.root, root, "opening changed the session root");

        a.handle_key(Key::Left, &tx);
        assert_eq!(open_picker(&a).current, root, "Left did not go up");
        a.handle_key(Key::Left, &tx);
        let above = base.canonicalize().unwrap();
        assert_eq!(open_picker(&a).current, above, "Left did not climb");
        assert!(rx.try_recv().is_err(), "navigating changed the root");

        let back = format!("{}/", root.file_name().unwrap().to_string_lossy());
        highlight(&mut a, &tx, &back);
        a.handle_key(Key::Right, &tx);
        assert_eq!(open_picker(&a).current, root, "Right did not open");

        highlight(&mut a, &tx, "src/");
        a.handle_key(Key::Enter, &tx);
        match rx.try_recv() {
            Ok(Cmd::SetRoot(path)) => assert_eq!(path, root.join("src")),
            _ => panic!("Enter did not choose the directory"),
        }
        assert!(a.pending.is_none(), "Enter left the browser open");
        assert_eq!(a.root, root, "the root moved before the worker replied");
    }

    #[test]
    fn the_image_browser_opens_folders_with_right() {
        let root = scratch("img-right");
        std::fs::create_dir_all(root.join("shots")).unwrap();
        std::fs::write(root.join("shots").join("a.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        let mut a = app_at(root.clone());
        let (tx, _rx) = channel();
        a.run_command("image", &tx);
        assert!(open_picker(&a).footer().starts_with("Enter attach · Tab/→ open"));
        highlight(&mut a, &tx, "shots/");
        a.handle_key(Key::Right, &tx);
        let picker = open_picker(&a);
        assert_eq!(picker.current, root.join("shots"), "Right did not open");
        assert!(picker.items.iter().any(|i| i.label == "a.png"));
    }

    #[test]
    fn image_browser_lists_images_and_attaches_on_enter() {
        let root = scratch("img");
        std::fs::create_dir_all(root.join("Pictures")).unwrap();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        let png = b"\x89PNG\r\n\x1a\n1234567890";
        std::fs::write(root.join("Pictures").join("shot.png"), png).unwrap();
        std::fs::write(root.join("Pictures").join("notes.txt"), "text").unwrap();

        let mut a = app_at(root.clone());
        let (tx, rx) = channel();
        a.run_command("image", &tx);
        {
            let picker = open_picker(&a);
            assert_eq!(picker.kind, PickKind::Image);
            let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
            assert!(labels.contains(&"Pictures/"), "{labels:?}");
            assert!(labels.contains(&"docs/"), "{labels:?}");
            assert!(!labels.contains(&"notes.txt"), "a text file was listed");
        }

        highlight(&mut a, &tx, "Pictures/");
        a.handle_key(Key::Enter, &tx);
        assert_eq!(open_picker(&a).current, root.join("Pictures"));
        assert!(rx.try_recv().is_err(), "Enter on a directory attached");

        {
            let picker = open_picker(&a);
            let labels: Vec<&str> = picker.items.iter().map(|i| i.label.as_str()).collect();
            assert!(labels.contains(&"shot.png"), "{labels:?}");
            assert!(!labels.contains(&"notes.txt"), "a text file was listed");
        }
        highlight(&mut a, &tx, "shot.png");
        a.handle_key(Key::Enter, &tx);
        assert!(a.pending.is_none(), "the image picker stayed open");
        match rx.try_recv() {
            Ok(Cmd::Attach(images)) => {
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].name, "shot.png");
            }
            _ => panic!("Enter did not attach the image"),
        }
        assert!(
            a.entries
                .last()
                .map(|e| e.text.contains("shot.png"))
                .unwrap_or(false),
            "no attachment marker"
        );
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
    fn slash_commands_with_values_reach_the_parser() {
        let mut a = app();
        let (tx, rx) = channel();
        a.input = "/cd .".into();
        a.cursor = a.input.len();
        a.submit(&tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::SetRoot(_))));
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
            detail: String::new(),
            ms: 4,
        });
        let text = &a.entries[1].text;
        assert!(text.contains("args:"), "{text:?}");
        assert!(text.contains("foo"), "{text:?}");

        a.run_command("verbosity low", &tx);
        assert!(!a.verbose);
        a.run_command("verbosity", &tx);
        let picker = open_picker(&a);
        assert_eq!(
            picker.kind,
            PickKind::Verbosity,
            "bare verbosity opened no picker"
        );
        assert_eq!(
            picker.selected().map(|i| i.value.as_str()),
            Some("low"),
            "the current level is not highlighted"
        );
        a.handle_key(Key::Down, &tx);
        a.handle_key(Key::Enter, &tx);
        assert!(a.verbose, "choosing high did nothing");
        assert!(a.pending.is_none(), "the picker stayed open");
        a.run_command("verbosity loud", &tx);
        assert!(a.entries.last().unwrap().text.contains("use low or high"));
    }

    #[test]
    fn agent_picker_enter_switches_and_esc_does_not() {
        let mut a = app();
        open_agent_picker(&mut a);
        let (tx, rx) = channel();
        a.handle_key(Key::Down, &tx);
        a.handle_key(Key::Enter, &tx);
        match rx.try_recv() {
            Ok(Cmd::SetAgent(name)) => assert_eq!(name, "reviewer"),
            _ => panic!("the agent picker sent no switch"),
        }
        assert!(a.pending.is_none(), "the picker stayed open");
        assert_eq!(a.agent, "reviewer");

        open_agent_picker(&mut a);
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none());
        assert!(rx.try_recv().is_err(), "Esc sent a command");
        assert_eq!(a.agent, "reviewer", "Esc changed the agent");
    }

    #[test]
    fn session_picker_esc_sends_nothing() {
        let mut a = picker();
        let (tx, rx) = channel();
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none(), "the picker stayed open");
        assert!(rx.try_recv().is_err(), "Esc resumed anyway");
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
            detail: String::new(),
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
            detail: String::new(),
            ms: 1,
        });
        let out = a.frame(80, 12);
        let success = a.palette.paint(Role::Success, "✓");
        assert!(out.contains(&format!("\x1b[1m{success}")), "mark not bold success: {out:?}");
        let tool = a.palette.paint(Role::Tool, " run `true`");
        assert!(out.contains(&tool), "summary left the tool colour: {out:?}");

        a.palette = Palette {
            on: false,
            allowed: false,
            truecolor: false,
            theme: &crate::theme::THEMES[0],
        };
        let plain = a.frame(80, 12);
        assert!(
            !plain.contains("\x1b[1m"),
            "bold with colour off: {plain:?}"
        );
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

    fn tool_call(a: &mut App, summary: String) {
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary,
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: String::new(),
            output: String::new(),
            detail: String::new(),
            ms: 4,
        });
    }

    #[test]
    fn tool_rows_start_in_the_same_column() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `ls`".into(),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: String::new(),
            output: String::new(),
            detail: String::new(),
            ms: 4,
        });
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `false`".into(),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: false,
            preview: "exit 1".into(),
            output: "exit 1".into(),
            detail: String::new(),
            ms: 7,
        });
        let painted = strip(&a.frame(60, 12));
        for row in painted.split("\r\n") {
            if row.contains('✓') || row.contains('✗') {
                assert!(
                    row.starts_with('✓') || row.starts_with('✗'),
                    "tool row is indented: {row:?}"
                );
            }
        }
    }

    #[test]
    fn a_long_tool_summary_wraps_and_keeps_its_duration() {
        let mut a = app();
        let command = format!("echo {}", "a".repeat(300));
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: crate::agent::describe("bash", &serde_json::json!({"command": command})),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: String::new(),
            output: String::new(),
            detail: String::new(),
            ms: 12,
        });
        let painted = strip(&a.frame(80, 30));
        let rows: Vec<&str> = painted
            .split("\r\n")
            .filter(|row| row.contains("aaaa"))
            .collect();
        assert!(rows.len() > 1, "the summary did not wrap: {painted:?}");
        let last = rows.last().unwrap();
        assert!(last.ends_with("12ms"), "the duration left the row end: {painted:?}");
        assert_eq!(last.chars().count(), 80, "{last:?}");
        assert!(!painted.contains('…'), "the summary was cut: {painted:?}");
    }

    #[test]
    fn durations_sit_at_the_right_edge() {
        let finish = |summary: &str, ms: u128| {
            let mut a = app();
            a.apply(Ev::ToolStart {
                name: "bash".into(),
                summary: summary.into(),
                args: "{}".into(),
            });
            a.apply(Ev::ToolEnd {
                ok: true,
                preview: String::new(),
                output: String::new(),
                detail: String::new(),
                ms,
            });
            strip(&a.frame(40, 12))
        };
        let fast = finish("run `ls`", 5);
        let row = fast.split("\r\n").find(|r| r.contains("run `ls`")).unwrap();
        assert!(row.starts_with("✓ run `ls`") && row.ends_with("5ms"), "{row:?}");
        assert_eq!(row.chars().count(), 40);
        let slow = finish("run `ls`", 1400);
        assert!(slow.contains("1.4s"), "{slow:?}");
        // 38 columns of summary leave no room: the time takes the next row.
        let full = finish(&"x".repeat(36), 7);
        let rows: Vec<&str> = full.split("\r\n").collect();
        let at = rows.iter().position(|r| r.contains("xxxx")).unwrap();
        assert!(!rows[at].contains("7ms"), "{rows:?}");
        assert!(rows[at + 1].trim() == "7ms" && rows[at + 1].ends_with("7ms"), "{rows:?}");
    }

    #[test]
    fn a_multi_line_summary_keeps_its_duration_last() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `cat <<EOF\nhello\nEOF`".into(),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: true,
            preview: String::new(),
            output: String::new(),
            detail: String::new(),
            ms: 9,
        });
        let frame = strip(&a.frame(60, 12));
        let rows: Vec<&str> = frame.split("\r\n").collect();
        let first = rows.iter().position(|r| r.contains("cat <<EOF")).unwrap();
        assert!(!rows[first].contains("9ms"), "{rows:?}");
        let last = rows.iter().position(|r| r.contains("EOF`")).unwrap();
        assert!(rows[last].ends_with("9ms"), "{rows:?}");
    }

    #[test]
    fn a_long_failure_preview_wraps_under_the_indent() {
        let mut a = app();
        a.apply(Ev::ToolStart {
            name: "bash".into(),
            summary: "run `false`".into(),
            args: "{}".into(),
        });
        a.apply(Ev::ToolEnd {
            ok: false,
            preview: "exit 1 timed out after 30s ".to_string() + &"reason ".repeat(20),
            output: String::new(),
            detail: String::new(),
            ms: 30_000,
        });
        let painted = strip(&a.frame(70, 20));
        let rows: Vec<&str> = painted
            .split("\r\n")
            .filter(|row| row.contains("reason"))
            .collect();
        assert!(rows.len() > 1, "the failure preview did not wrap: {painted:?}");
        for row in rows {
            assert!(row.starts_with("  "), "preview lost its indent: {row:?}");
        }
    }

    #[test]
    fn an_ask_question_wraps_instead_of_clipping() {
        let mut a = app();
        let question = "delete all five files? ".repeat(12);
        a.apply(Ev::ToolStart {
            name: "ask".into(),
            summary: crate::agent::describe("ask", &serde_json::json!({"question": question})),
            args: "{}".into(),
        });
        let painted = strip(&a.frame(80, 30));
        let rows: Vec<&str> = painted
            .split("\r\n")
            .filter(|row| row.contains("delete all five files?"))
            .collect();
        assert!(rows.len() > 1, "the question did not wrap: {painted:?}");
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
        assert!(
            a.entries[0].text.starts_with(spinner_frame(a.spinner)),
            "no spinner while running"
        );
        a.apply(Ev::ToolEnd {
            ok: false,
            preview: "exit 1".into(),
            output: "exit 1".into(),
            detail: String::new(),
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
        a.pending = Some(Pending::Pick(Picker::new(
            "session",
            vec![PickItem {
                label: "abc123  3m  old title  4 msgs".into(),
                value: "abc123".into(),
                actions: Vec::new(),
                desc: String::new(),
            }],
            PickKind::Session,
        )));
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
        let frame = strip(&a.frame(100, 30));
        assert!(frame.contains("delete abc123? y/N"), "{frame}");
        assert!(frame.contains("old title"), "the list vanished: {frame}");
        a.handle_key(Key::Char('y'), &tx);
        match rx.try_recv() {
            Ok(Cmd::Delete(id)) => assert_eq!(id, "abc123"),
            _ => panic!("expected delete"),
        }
        let picker = open_picker(&a);
        assert!(picker.items.is_empty(), "the deleted row stayed");
    }

    #[test]
    fn the_delete_question_names_the_title() {
        let mut a = picker();
        let Some(Pending::Pick(list)) = a.pending.take() else { panic!("no picker") };
        a.pending = Some(Pending::ConfirmDelete {
            id: "51280befc".into(),
            title: "Rate my machine.".into(),
            picker: Box::new(list),
        });
        let frame = strip(&a.frame(100, 30));
        assert!(frame.contains("delete \"Rate my machine.\" (51280befc)? y/N"), "{frame}");
    }

    #[test]
    fn text_prompts_use_the_block_caret() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("rename", &tx);
        a.handle_key(Key::Char('x'), &tx);
        let frame = a.frame(80, 20);
        assert!(frame.contains("new title: x\x1b[7m \x1b[27m"), "{frame:?}");
        assert!(!strip(&frame).contains("x_"), "{frame:?}");
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
    fn a_row_shortcut_runs_only_while_the_filter_is_empty() {
        let mut a = picker();
        let (tx, _rx) = channel();
        a.handle_key(Key::Char('r'), &tx);
        assert!(
            matches!(a.pending, Some(Pending::Prompt { .. })),
            "r did not rename the highlighted session"
        );

        let mut a = picker();
        a.handle_key(Key::Char('x'), &tx);
        a.handle_key(Key::Char('r'), &tx);
        let picker = open_picker(&a);
        assert_eq!(picker.filter, "xr", "r bypassed the filter");
        assert!(matches!(a.pending, Some(Pending::Pick(_))));
    }

    #[test]
    fn the_footer_names_the_row_actions() {
        let mut a = picker();
        let frame = strip(&a.frame(80, 24));
        assert!(frame.contains("Enter choose"), "{frame}");
        assert!(frame.contains("Esc close"), "{frame}");
        assert!(frame.contains("n new"), "{frame}");
        assert!(frame.contains("r rename"), "{frame}");
        assert!(frame.contains("d delete"), "{frame}");
    }

    #[test]
    fn slash_autocomplete_uses_the_picker() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.handle_key(Key::Char('/'), &tx);
        assert_eq!(open_picker(&a).kind, PickKind::Command);

        a.handle_key(Key::Char('c'), &tx);
        a.handle_key(Key::Char('o'), &tx);
        {
            let picker = open_picker(&a);
            let labels: Vec<&str> = picker
                .matches()
                .iter()
                .map(|&i| picker.items[i].label.as_str())
                .collect();
            assert_eq!(labels.first().copied(), Some("/compact"), "{labels:?}");
            assert!(!labels.contains(&"/help"), "{labels:?}");
        }

        a.handle_key(Key::Tab, &tx);
        assert!(a.pending.is_none(), "Tab left the overlay open");
        assert_eq!(a.input, "/compact", "Tab did not complete the command");

        let (tx, rx) = channel();
        a.handle_key(Key::Enter, &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Compact)));
        assert!(a.input.is_empty(), "the input did not clear");
    }

    #[test]
    fn tab_completes_and_opens_values() {
        let type_tab = |text: &str| {
            let mut a = app();
            let (tx, _rx) = channel();
            for c in text.chars() {
                a.handle_key(Key::Char(c), &tx);
            }
            a.handle_key(Key::Tab, &tx);
            a
        };
        let a = type_tab("/the");
        assert_eq!(a.input, "/theme ");
        let picker = open_picker(&a);
        assert_eq!(picker.kind, PickKind::Theme);
        assert_eq!(picker.items[picker.index].value, a.palette.theme_name());

        let a = type_tab("/ren");
        assert_eq!(a.input, "/rename ");
        assert!(a.pending.is_none());

        let a = type_tab("/comp");
        assert_eq!(a.input, "/compact");
        assert!(a.pending.is_none());
        assert!(!a.entries.iter().any(|e| e.text.contains("compacting")));

        let a = type_tab("/sess");
        assert_eq!(a.input, "/sessions ");
        assert_eq!(open_picker(&a).kind, PickKind::Session);
    }

    #[test]
    fn slash_autocomplete_runs_and_dismisses() {
        let mut a = app();
        let (tx, rx) = channel();
        a.handle_key(Key::Char('/'), &tx);
        a.handle_key(Key::Char('h'), &tx);
        a.handle_key(Key::Char('e'), &tx);
        a.handle_key(Key::Enter, &tx);
        assert!(a.pending.is_none(), "Enter left the overlay open");
        assert!(a.input.is_empty(), "the input did not clear");
        assert!(a.entries.iter().any(|e| e.text.contains("/help")));

        let mut a = app();
        let (tx, _rx) = channel();
        a.handle_key(Key::Char('/'), &tx);
        a.handle_key(Key::Char('c'), &tx);
        a.handle_key(Key::Char('o'), &tx);
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none(), "Esc left the overlay open");
        assert_eq!(a.input, "/co", "Esc discarded the typed text");
    }

    #[test]
    fn a_replay_draws_the_restored_conversation() {
        use crate::api::{Message, ToolCall};
        let mut asked = Message::assistant("");
        asked.tool_calls.push(ToolCall {
            id: "c1".into(),
            name: "bash".into(),
            arguments: "{\"command\":\"ls\"}".into(),
        });
        let history = vec![
            Message::system("prompt"),
            Message::user("list the files"),
            asked,
            Message::tool("c1", "a\nb"),
            Message::assistant("two files"),
        ];
        let mut a = app();
        a.push(Role::User, "old conversation");
        let (tx, rx) = channel();
        send_replay(&tx, &history);
        while let Ok(ev) = rx.try_recv() {
            a.apply(ev);
        }
        let frame = strip(&a.frame(80, 24));
        assert!(!frame.contains("old conversation"), "{frame}");
        let user = frame.find("> list the files").expect("user row");
        let tool = frame.find("▸ ").expect("tool row");
        let reply = frame.find("two files").expect("reply");
        assert!(user < tool && tool < reply, "{frame}");
        assert!(!frame.contains("prompt"), "{frame}");
        assert!(!frame.contains(" 0 tok"), "{frame}");
    }

    #[test]
    fn a_typed_session_id_resumes() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("sessions 51280befc", &tx);
        match rx.try_recv() {
            Ok(Cmd::Resume(id)) => assert_eq!(id, "51280befc"),
            _ => panic!("expected a resume command"),
        }
    }

    #[test]
    fn replies_use_one_form() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("nope", &tx);
        let last = a.entries.last().unwrap();
        assert_eq!(last.role, Role::Error);
        assert!(last.text.contains("/nope"), "{}", last.text);
        a.run_command("think", &tx);
        let last = a.entries.last().unwrap();
        assert!(last.text == "thinking: on" || last.text == "thinking: off", "{}", last.text);
        a.run_command("verbosity loud", &tx);
        let last = a.entries.last().unwrap();
        assert_eq!(last.role, Role::Error);
        assert!(last.text.contains("low or high"), "{}", last.text);
    }

    #[test]
    fn choosing_a_session_sends_resume() {
        let mut a = app();
        a.pending = Some(Pending::Pick(Picker::new(
            "session",
            vec![PickItem {
                label: "abc123  title  3 msgs".into(),
                value: "abc123".into(),
                actions: Vec::new(),
                desc: String::new(),
            }],
            PickKind::Session,
        )));
        let (tx, rx) = channel();
        a.handle_key(Key::Enter, &tx);
        match rx.try_recv() {
            Ok(Cmd::Resume(id)) => assert_eq!(id, "abc123"),
            _ => panic!("expected a resume command"),
        }
    }

    fn open_agent_picker(a: &mut App) {
        a.agents = ["default", "reviewer", "researcher", "writer"]
            .iter()
            .map(|n| (n.to_string(), format!("{n} profile")))
            .collect();
        let (tx, _rx) = channel();
        a.run_command("agent", &tx);
    }

    fn open_picker<'a>(a: &'a App) -> &'a Picker {
        match &a.pending {
            Some(Pending::Pick(picker)) => picker,
            _ => panic!("no picker is open"),
        }
    }

    #[test]
    fn picker_filter_narrows_and_resets_the_highlight() {
        let mut a = app();
        open_agent_picker(&mut a);
        let (tx, _rx) = channel();
        a.handle_key(Key::Down, &tx);
        a.handle_key(Key::Down, &tx);
        assert_eq!(open_picker(&a).index, 2, "Down did not move");

        for c in "rev".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        let picker = open_picker(&a);
        assert_eq!(picker.filter, "rev");
        let labels: Vec<&str> = picker
            .matches()
            .iter()
            .map(|&i| picker.items[i].label.as_str())
            .collect();
        assert_eq!(labels, vec!["reviewer"], "filter kept the wrong rows");
        assert_eq!(picker.index, 0, "filter did not reset the highlight");

        a.handle_key(Key::Backspace, &tx);
        assert_eq!(open_picker(&a).filter, "re");
        assert_eq!(open_picker(&a).matches().len(), 2);
    }

    #[test]
    fn picker_pages_eight_rows_at_a_time() {
        let mut a = app();
        let items = (0..40)
            .map(|i| PickItem {
                label: format!("profile {i}"),
                value: format!("p{i}"),
                actions: Vec::new(),
                desc: String::new(),
            })
            .collect();
        a.pending = Some(Pending::Pick(Picker::new("agent", items, PickKind::Agent)));
        let (tx, _rx) = channel();

        a.handle_key(Key::PageDown, &tx);
        assert_eq!(open_picker(&a).index, 8, "PgDn did not move eight rows");
        assert_eq!(open_picker(&a).offset, 1, "the window did not scroll");

        a.handle_key(Key::PageDown, &tx);
        a.handle_key(Key::PageUp, &tx);
        assert_eq!(open_picker(&a).index, 8, "PgUp did not move eight rows");
    }

    #[test]
    fn picker_enter_on_an_empty_result_does_nothing() {
        let mut a = app();
        open_agent_picker(&mut a);
        let (tx, rx) = channel();
        for c in "zzz".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        assert!(open_picker(&a).matches().is_empty());
        assert!(
            strip(&a.frame(80, 20)).contains("no matches"),
            "no no-match row"
        );
        a.handle_key(Key::Enter, &tx);
        assert!(a.pending.is_some(), "Enter closed an empty picker");
        assert!(rx.try_recv().is_err(), "Enter sent a command");
    }

    #[test]
    fn picker_esc_cancels_without_changing_anything() {
        let mut a = app();
        open_agent_picker(&mut a);
        let (tx, rx) = channel();
        a.handle_key(Key::Char('w'), &tx);
        a.handle_key(Key::Esc, &tx);
        assert!(a.pending.is_none(), "the picker stayed open");
        assert!(rx.try_recv().is_err(), "Esc sent a command");
        assert_eq!(a.agent, "default", "Esc changed the agent");
    }

    #[test]
    fn agent_picker_highlights_the_current_profile() {
        let mut a = app();
        a.agent = "writer".into();
        open_agent_picker(&mut a);
        let picker = open_picker(&a);
        assert_eq!(picker.selected().map(|i| i.label.as_str()), Some("writer"));
    }

    fn open_many(a: &mut App, count: usize) {
        let items = (0..count)
            .map(|i| PickItem {
                label: format!("profile {i}"),
                value: format!("p{i}"),
                actions: Vec::new(),
                desc: String::new(),
            })
            .collect();
        a.pending = Some(Pending::Pick(Picker::new("agent", items, PickKind::Agent)));
    }

    #[test]
    fn overlay_is_forty_columns_in_a_narrow_terminal() {
        let mut a = app();
        open_many(&mut a, 3);
        let frame = a.frame(50, 24);
        let start = frame
            .find('╭')
            .unwrap_or_else(|| panic!("no overlay in {frame:?}"));
        let end = frame[start..]
            .find('╮')
            .map(|at| start + at)
            .unwrap_or_else(|| panic!("no top border in {frame:?}"));
        assert_eq!(
            frame[start..end].chars().count() + 1,
            40,
            "wrong box width: {:?}",
            &frame[start..end]
        );
    }

    #[test]
    fn overlay_shows_eight_rows_and_scrolls() {
        let mut a = app();
        open_many(&mut a, 40);
        let frame = a.frame(100, 30);
        let rows = frame.matches("H│▸").count() + frame.matches("H│  ").count();
        assert_eq!(rows, 8, "wrong number of item rows in {frame:?}");
        assert!(strip(&frame).contains("profile 0"), "{frame:?}");

        let (tx, _rx) = channel();
        for _ in 0..8 {
            a.handle_key(Key::Down, &tx);
        }
        let scrolled = strip(&a.frame(100, 30));
        assert!(!scrolled.contains("profile 0"), "the window did not scroll");
        assert!(scrolled.contains("profile 8"), "{scrolled:?}");
    }

    #[test]
    fn the_overlay_sets_title_position_and_keys_in_its_edges() {
        let mut a = app();
        open_many(&mut a, 40);
        let frame = strip(&a.frame(200, 30));
        let start = frame.find('╭').expect("top edge");
        let end = start + frame[start..].find('╮').expect("top corner");
        let top = &frame[start..end + '╮'.len_utf8()];
        assert!(top.starts_with("╭─ agent "), "{top}");
        assert!(top.ends_with(" 1/40 ─╮"), "{top}");
        assert_eq!(top.chars().count(), 72, "{top}");
        let bottom = &frame[frame.find('╰').expect("bottom edge")..];
        assert!(bottom.starts_with("╰─ Enter choose · Esc close "), "{bottom}");
        assert!(frame.contains("│ › "), "no filter row: {frame}");
    }

    #[test]
    fn the_filter_cursor_follows_the_typed_text() {
        let mut a = app();
        open_many(&mut a, 3);
        let (tx, _rx) = channel();
        a.handle_key(Key::Char('r'), &tx);
        let (_, x, y) = a.overlay(100, 30).expect("overlay");
        // Box at column 15 (72 wide in 100): wall, " › ", then `r`.
        assert_eq!(x, 15 + 1 + 3 + 1);
        assert!(y >= 1);
    }

    #[test]
    fn closing_the_picker_leaves_no_border() {
        let mut a = app();
        a.push(Role::User, "a transcript row");
        let before = a.frame(80, 20);
        open_many(&mut a, 40);
        let open = a.frame(80, 20);
        assert!(open.contains('╭'), "the picker did not draw");
        let (tx, _rx) = channel();
        a.handle_key(Key::Esc, &tx);
        let after = a.frame(80, 20);
        assert!(!after.contains('╭'), "border residue after close");
        assert_eq!(before, after, "closing the picker changed the frame");
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
        // Turns are spaced now, so probe whichever row the page showed.
        let probe = (0..40)
            .map(|i| format!("> line {i}\r"))
            .find(|l| before.contains(l.as_str()))
            .unwrap_or_else(|| panic!("no row visible: {before:?}"));
        assert!(after.contains(&probe), "rows moved: {after:?}");
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
    fn wrap_keeps_leading_spaces() {
        assert_eq!(wrap("  /new  start", 40), vec!["  /new  start"]);
    }

    #[test]
    fn wrap_keeps_blank_lines() {
        assert_eq!(wrap("a\n\nb", 20).len(), 3);
    }
    #[test]
    fn a_typed_command_on_the_deny_list_is_refused() {
        let mut a = app();
        a.run_local("echo mkfs-probe");
        let last = a.entries.last().unwrap();
        assert_eq!(last.role, Role::Error);
        assert!(last.text.contains("refused"), "{}", last.text);
        assert!(!last.text.contains("\nmkfs-probe"), "it ran: {}", last.text);
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
    fn ctrl_c_clears_a_draft_instead_of_quitting() {
        let mut a = app();
        a.input = "half written".into();
        a.cursor = 4;
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(!a.quit, "a draft did not protect the session");
        assert!(a.input.is_empty(), "the draft stayed: {:?}", a.input);
        assert_eq!(a.cursor, 0);
    }

    #[test]
    fn ctrl_c_clears_a_draft_without_cancelling_the_turn() {
        let mut a = app();
        a.status.streaming = true;
        a.input = "draft".into();
        a.cursor = 5;
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(!a.quit, "a draft did not protect the turn");
        assert!(a.input.is_empty());
        assert!(
            !a.cancel.load(std::sync::atomic::Ordering::Relaxed),
            "clearing a draft cancelled the turn"
        );
    }

    #[test]
    fn ctrl_c_closes_a_picker_and_clears_its_draft() {
        let mut a = app();
        a.input = "/sess".into();
        a.cursor = 5;
        a.pending = Some(Pending::Pick(Picker::new(
            "commands",
            command_items(),
            PickKind::Command,
        )));
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(!a.quit, "a picker with a draft exited");
        assert!(a.pending.is_none(), "the picker stayed open");
        assert!(a.input.is_empty(), "the draft stayed: {:?}", a.input);
    }

    #[test]
    fn ctrl_c_exits_while_streaming() {
        let mut a = app();
        a.status.streaming = true;
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('c'), &tx);
        assert!(a.quit, "Ctrl+C did not exit mid-turn");
    }

    #[test]
    fn the_loop_sleeps_when_nothing_moves() {
        let mut a = app();
        let quiet = Duration::from_secs(3);
        assert_eq!(a.loop_wait(quiet), Duration::from_secs(5));
        assert_eq!(a.loop_wait(Duration::from_millis(500)), Duration::from_millis(50), "a recent key");
        a.status.streaming = true;
        assert_eq!(a.loop_wait(quiet), Duration::from_millis(50));
        a.status.streaming = false;
        a.pending_since = Some(Instant::now());
        assert_eq!(a.loop_wait(quiet), Duration::from_secs(1));
    }

    #[test]
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
            always: false,
        });
        let out = strip(&a.frame(90, 12));
        assert!(out.contains("run it?   [y] yes  [n] no"), "no verb prompt: {out:?}");
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
            always: false,
        });
        assert!(strip(&a.frame(80, 12)).contains("approve?   [y] yes  [n] no"));
    }

    #[test]
    fn ctrl_c_exits_from_a_picker() {
        let mut a = app();
        a.pending = Some(Pending::Pick(Picker::new(
            "session",
            vec![PickItem {
                label: "one".into(),
                value: "one".into(),
                actions: Vec::new(),
                desc: String::new(),
            }],
            PickKind::Session,
        )));
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
            always: false,
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
