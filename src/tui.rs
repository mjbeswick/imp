//! Terminal interface: raw ANSI, colours by role, one chat column.
//!
//! The worker thread owns the session; the UI thread owns the screen.
//! They talk over two channels, and confirmations block the worker until
//! the user answers.

use crate::agent::{Answer, Hooks, Session, Ui};
use crate::config::{ColorMode, Config, Verbosity};
use std::ffi::{c_int, c_ulong};
use std::io::Write;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

const ALT_ON: &str = "\x1b[?1049h\x1b[?25l\x1b[>1u\x1b[?2004h";
const ALT_OFF: &str = "\x1b[?1006l\x1b[?1002l\x1b[?1000l\x1b[?2004l\x1b[<u\x1b[?25h\x1b[?1049l";
/// SGR mouse reports, so the wheel reaches imp instead of the scrollback.
const MOUSE_ON: &str = "\x1b[?1000h\x1b[?1002h\x1b[?1006h";
/// How long a confirmation stays in the status line.
const TOAST: Duration = Duration::from_secs(3);
const MOUSE_OFF: &str = "\x1b[?1006l\x1b[?1002l\x1b[?1000l";
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

/// The chat screen as `cols << 16 | rows`, 0 until the first frame.
// ponytail: one global, as a process has one screen.
pub static SCREEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The chat screen's columns and rows, or None before the first frame.
pub fn screen() -> Option<(u16, u16)> {
    let v = SCREEN.load(Ordering::Relaxed);
    (v != 0).then(|| ((v >> 16) as u16, v as u16))
}

/// Give an agent command `COLUMNS` and `LINES` for the screen, when known,
/// and `IMP_TMP` for the scratch folder.
pub fn agent_env(cmd: &mut std::process::Command) {
    if let Some((cols, rows)) = screen() {
        cmd.env("COLUMNS", cols.to_string()).env("LINES", rows.to_string());
    }
    if let Some(dir) = crate::tools::scratch() {
        cmd.env("IMP_TMP", dir);
    }
}

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
const RESTORE: &str = "\x1b[?1006l\x1b[?1002l\x1b[?1000l\x1b[?2004l\x1b[<u\x1b[?25h\x1b[?1049l";

extern "C" {
    fn write(fd: std::ffi::c_int, buf: *const std::ffi::c_void, len: usize) -> isize;
    fn _exit(code: std::ffi::c_int) -> !;
    fn signal(sig: std::ffi::c_int, handler: usize) -> usize;
    fn tcgetattr(fd: c_int, termios: *mut Termios) -> c_int;
    fn tcsetattr(fd: c_int, action: c_int, termios: *const Termios) -> c_int;
    fn read(fd: c_int, buf: *mut std::ffi::c_void, len: usize) -> isize;
    fn pipe(fds: *mut c_int) -> c_int;
    fn poll(fds: *mut PollFd, n: Nfds, timeout: c_int) -> c_int;
}

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

#[cfg(target_os = "linux")]
type Nfds = std::ffi::c_ulong;
#[cfg(not(target_os = "linux"))]
type Nfds = std::ffi::c_uint;

/// A command, such as `/shell` or a password prompt, owns the terminal. The
/// key reader must not read then: a read from the background stops imp, which
/// ends the user's own shell, or fails and leaves imp deaf.
static ATTACHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The key reader has seen `ATTACHED` and stopped reading.
static PARKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Read keys from `fd` and send them on, never reading while `attached` is
/// set. Waits in `poll` so the flag is checked at least every 50 ms.
fn read_keys(fd: c_int, tx: Sender<Vec<u8>>, attached: &std::sync::atomic::AtomicBool, parked: &std::sync::atomic::AtomicBool) {
    use std::sync::atomic::Ordering::SeqCst;
    let mut buf = [0u8; 64];
    loop {
        if attached.load(SeqCst) {
            parked.store(true, SeqCst);
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        parked.store(false, SeqCst);
        let mut p = PollFd { fd, events: 1, revents: 0 };
        // A timeout or a signal: look at the flag again.
        if unsafe { poll(&mut p, 1, 50) } <= 0 || attached.load(SeqCst) {
            continue;
        }
        let n = unsafe { read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n == 0 {
            break;
        }
        if n < 0 {
            // ponytail: a failed read is retried, not fatal; SIGHUP still ends imp.
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        if tx.send(buf[..n as usize].to_vec()).is_err() {
            break;
        }
    }
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
    unsafe { write(1, ask.as_ptr().cast(), ask.len()) };
    let start = Instant::now();
    let mut reply = Vec::new();
    let mut buf = [0u8; 64];
    while start.elapsed() < Duration::from_millis(100) {
        let n = unsafe { read(0, buf.as_mut_ptr().cast(), buf.len()) };
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

/// The write end of the watchdog pipe, for the signal handler.
static QUIT_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

extern "C" fn on_signal(_: std::ffi::c_int) {
    // If the reset below blocks on a terminal that stopped reading, the
    // watchdog ends the process anyway.
    let fd = QUIT_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        unsafe {
            write(fd, [1u8].as_ptr().cast(), 1);
        }
    }
    if let Ok(mut slot) = SAVED.try_lock() {
        if let Some(saved) = slot.take() {
            unsafe {
                tcsetattr(0, TCSANOW, &saved);
            }
        }
    }
    unsafe {
        write(1, RESTORE.as_ptr().cast(), RESTORE.len());
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
            write(fd, [1u8].as_ptr().cast(), 1);
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
        while unsafe { read(fds[0], byte.as_mut_ptr().cast(), 1) } > 0 {
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
            write(1, RESTORE.as_ptr().cast(), RESTORE.len());
        }
        eprintln!("{info}");
    }));
    unsafe {
        signal(15, on_signal as *const () as usize);
        signal(2, on_signal as *const () as usize);
        signal(1, on_signal as *const () as usize);
    }
    let mut fds = [0 as c_int; 2];
    if unsafe { pipe(fds.as_mut_ptr()) } == 0 {
        QUIT_FD.store(fds[1], Ordering::Relaxed);
        std::thread::spawn(move || {
            let mut byte = [0u8; 1];
            if unsafe { read(fds[0], byte.as_mut_ptr().cast(), 1) } > 0 {
                std::thread::sleep(Duration::from_secs(1));
                unsafe { _exit(1) };
            }
        });
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
    /// Diff context and notes under a step: muted, without thinking's `~`.
    Context,
    Border,
    Success,
    Code,
    Link,
}

pub(crate) struct Palette {
    pub(crate) on: bool,
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

    /// Text on a background shading from the select colour toward the accent,
    /// one colour per cell. Without truecolor it is the flat select row.
    pub(crate) fn gradient(&self, text: &str) -> String {
        if !self.on || !self.truecolor {
            return self.select(text);
        }
        use crate::theme::Slot;
        let from = self.theme.rgb(Slot::Select);
        let accent = self.theme.rgb(Slot::Accent);
        // Halfway to the accent at the right edge keeps the text readable.
        let to: [u8; 3] = std::array::from_fn(|i| ((from[i] as u16 + accent[i] as u16) / 2) as u8);
        let fg = self.theme.rgb(Slot::User);
        let n = text.chars().count().max(2) - 1;
        let mut out = format!("\x1b[38;2;{};{};{}m", fg[0], fg[1], fg[2]);
        for (i, c) in text.chars().enumerate() {
            let at = |k: usize| (from[k] as usize * (n - i.min(n)) + to[k] as usize * i.min(n)) / n;
            out.push_str(&format!("\x1b[48;2;{};{};{}m{c}", at(0), at(1), at(2)));
        }
        out + "\x1b[0m"
    }

    /// The highlighted picker row: user text on the select background.
    pub(crate) fn select(&self, text: &str) -> String {
        if !self.on {
            // Mono: no background, so the row is reversed.
            return self.styled("7", text.to_string());
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

    /// A run of a diff row: text colour on the row's tint, or on a tint
    /// twice as strong and bold when the words changed.
    pub(crate) fn diff_run(&self, role: Role, text: &str, mark: u8) -> String {
        use crate::syntax::{COMMENT, KEYWORD, NUMBER, STRING};
        use crate::theme::{strong_text, tint_256, tint_256_strong, tint_by, to_256, Slot};
        let band = self.theme.rgb(if role == Role::Add { Slot::Success } else { Slot::Error });
        let fg = self.theme.rgb(Slot::Text);
        let changed = mark & CHANGED != 0;
        let kind = mark & !CHANGED;
        // Changed words take white or black for contrast; otherwise strings,
        // numbers, and comments take their colours.
        let colour = match kind {
            _ if changed => None,
            STRING => Some(Slot::Success),
            NUMBER => Some(Slot::Accent),
            COMMENT => Some(Slot::Muted),
            _ => None,
        };
        let mut attrs = String::new();
        if changed || kind == KEYWORD {
            attrs.push_str("\x1b[1m");
        }
        if kind == COMMENT {
            attrs.push_str("\x1b[3m");
        }
        // A syntax colour too faint on this tint gives way to the text colour;
        // the bold or italic stays.
        use crate::theme::{contrast, from_256};
        if self.truecolor {
            let bg = tint_by(band, fg, if changed { 2 } else { 1 });
            let [x, y, z] = match colour.map(|slot| self.theme.rgb(slot)) {
                Some(c) if contrast(c, bg) >= 4.5 => c,
                _ if changed => strong_text(fg),
                _ => fg,
            };
            let [r, g, b] = bg;
            format!("\x1b[48;2;{r};{g};{b}m\x1b[38;2;{x};{y};{z}m{attrs}{text}\x1b[0m")
        } else {
            let (bg, f) = if changed { tint_256_strong(band, fg) } else { tint_256(band, fg) };
            let f = match colour.map(|slot| to_256(self.theme.rgb(slot))) {
                Some(c) if contrast(from_256(c), from_256(bg)) >= 4.5 => c,
                _ => f,
            };
            format!("\x1b[48;5;{bg}m\x1b[38;5;{f}m{attrs}{text}\x1b[0m")
        }
    }

    /// SGR attributes ahead of painted text. `mono` keeps them; `NO_COLOR`
    /// and `never` drop them with every other escape.
    pub(crate) fn styled(&self, codes: &str, painted: String) -> String {
        match (self.allowed && !codes.is_empty(), self.on) {
            (false, _) => painted,
            // `paint` already ends in a full reset.
            (true, true) => format!("\x1b[{codes}m{painted}"),
            (true, false) => format!("\x1b[{codes}m{painted}\x1b[0m"),
        }
    }

    /// Inline code: text on a tint of the code colour, as diff rows are,
    /// since the code colour on its own tint is too faint in some themes.
    /// Reversed under mono.
    pub(crate) fn code(&self, text: &str) -> String {
        if !self.on {
            return self.styled("7", text.to_string());
        }
        use crate::theme::{tint, tint_256, Slot};
        let (c, fg) = (self.theme.rgb(Slot::Code), self.theme.rgb(Slot::Text));
        if self.truecolor {
            let ([r, g, b], [x, y, z]) = (tint(c, fg), fg);
            format!("\x1b[48;2;{r};{g};{b}m\x1b[38;2;{x};{y};{z}m{text}\x1b[0m")
        } else {
            let (bg, fg) = tint_256(c, fg);
            format!("\x1b[48;5;{bg}m\x1b[38;5;{fg}m{text}\x1b[0m")
        }
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
            Role::Thinking | Role::Context => Slot::Muted,
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
        // Diff lines: plain text on a tint, so they never read as a ✓.
        if matches!(role, Role::Add | Role::Delete) {
            // The indent stays on the terminal background; the tint starts at the sign.
            let body = text.trim_start_matches(' ');
            let lead = &text[..text.len() - body.len()];
            let text = body;
            let fg = self.theme.rgb(Slot::Text);
            let bg = crate::theme::tint([r, g, b], fg);
            return if self.truecolor {
                format!(
                    "{lead}\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m{text}\x1b[0m",
                    bg[0], bg[1], bg[2], fg[0], fg[1], fg[2]
                )
            } else {
                let (bg, fg) = crate::theme::tint_256([r, g, b], fg);
                format!("{lead}\x1b[48;5;{bg}m\x1b[38;5;{fg}m{text}\x1b[0m")
            };
        }
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

/// What a click does: replay a key, or choose a picker match by index.
#[derive(Debug, Clone, PartialEq)]
enum Target {
    Key(Key),
    Row(usize),
    /// Copy entry `0`, or its fenced block `1` when set.
    Copy(usize, Option<usize>),
    /// Expand or collapse the latest turn's tool row at this entry.
    Tool(usize),
}

/// The copy label at a reply's or code block's right edge.
const COPY: char = '⧉';

/// Text without colour codes or control bytes, so columns match the screen.
/// Rows as terminal bytes inside one synchronised update. With `before`,
/// rows equal to it are skipped and the rest are placed by cursor moves;
/// without it every row is written from the top.
///
/// A row is written first and then erased to its end, so it is never blank
/// on screen. A row that fills the width is not erased: with wrap off, the
/// cursor rests on its last cell and the erase would take that cell.
fn paint_rows(screen: &[String], before: Option<&[String]>, cols: usize, tail: &str) -> String {
    let mut out = String::from("\x1b[?2026h\x1b[?25l\x1b[?7l");
    let last = screen.len().saturating_sub(1);
    if before.is_none() {
        out.push_str("\x1b[H");
    }
    for (i, row) in screen.iter().enumerate() {
        match before {
            Some(b) if b.get(i) == Some(row) => continue,
            Some(_) => out.push_str(&format!("\x1b[{};1H", i + 1)),
            // A full frame clears from the status row down, leftovers of a resize included.
            None if i == last => out.push_str("\x1b[J"),
            None => {}
        }
        out.push_str(row);
        if unpaint(row).chars().count() < cols {
            out.push_str("\x1b[K");
        }
        if before.is_none() && i < last {
            out.push_str("\r\n");
        }
    }
    out.push_str("\x1b[?7h");
    out.push_str(tail);
    out.push_str("\x1b[?2026l");
    out
}

fn unpaint(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            for n in it.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out
}

/// `y` and `n` pick only when the choices are Yes and No; elsewhere they type.
fn yes_no(opts: &[String]) -> bool {
    matches!(opts, [y, n] if y.eq_ignore_ascii_case("yes") && n.eq_ignore_ascii_case("no"))
}

/// Click targets in one drawn row: each `[k] label` chip replays `k`, and
/// `Esc stop` replays Esc. A chip runs to the next double space.
fn chip_targets(row: usize, text: &str) -> Vec<(usize, usize, usize, Target)> {
    let chars: Vec<char> = text.chars().collect();
    let end_of = |from: usize| {
        let mut j = from;
        while j < chars.len() && !(chars[j] == ' ' && chars.get(j + 1) == Some(&' ')) {
            j += 1;
        }
        j
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' && chars.get(i + 2) == Some(&']') && chars[i + 1].is_ascii_alphanumeric() {
            let end = end_of(i);
            out.push((row, i + 1, end, Target::Key(Key::Char(chars[i + 1]))));
            i = end;
        } else if chars[i..].starts_with(&['E', 's', 'c', ' ']) {
            let end = end_of(i);
            out.push((row, i + 1, end, Target::Key(Key::Esc)));
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Click targets in a picker footer such as `Enter choose · Esc close`,
/// which starts at column `x`: each hint replays its key.
fn footer_targets(row: usize, x: usize, text: &str) -> Vec<(usize, usize, usize, Target)> {
    let mut out = Vec::new();
    let mut col = x;
    for part in text.split(" · ") {
        let len = part.chars().count();
        let key = match part.split(' ').next().unwrap_or("") {
            "Enter" => Some(Key::Enter),
            "Esc" => Some(Key::Esc),
            "Tab/→" => Some(Key::Tab),
            "←" => Some(Key::Left),
            k if k.chars().count() == 1 => k.chars().next().map(Key::Char),
            _ => None,
        };
        if let Some(key) = key {
            out.push((row, col, col + len - 1, Target::Key(key)));
        }
        col += len + 3;
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
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
    /// Forward delete; Shift+Delete too.
    Delete,
    /// Shift with Up, Down, Left, Right, Home, or End: extend the selection.
    Select(Move),
    /// Option or Alt with ←/→ (or Alt+B/F): move by word.
    Word(Move),
    /// Shift+Option with ←/→: extend the selection by word.
    SelectWord(Move),
    /// Delete a word: after the caret when true, before it otherwise.
    DeleteWord(bool),
    Ctrl(char),
    WheelUp,
    WheelDown,
    /// A left-button press at a 1-based column and row.
    Click(u16, u16),
    /// Text the terminal marked as pasted, line breaks as `\n`.
    Paste(String),
    /// The pointer moved with the left button held, and where it let go.
    Drag(u16, u16),
    Release(u16, u16),
    Unknown,
}

/// A caret move that a Shift turns into a selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}

pub fn decode(buf: &mut Vec<u8>) -> Vec<Key> {
    let mut keys = Vec::new();
    while !buf.is_empty() {
        let b = buf[0];
        if b == 0x1b {
            if buf.starts_with(b"\x1b[200~") {
                // A paste may span reads: wait for its end marker.
                let Some(end) = buf.windows(6).position(|w| w == b"\x1b[201~") else { break };
                let text = String::from_utf8_lossy(&buf[6..end]).replace("\r\n", "\n").replace('\r', "\n");
                buf.drain(..end + 6);
                keys.push(Key::Paste(text));
                continue;
            }
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
            // Alt sends ESC and the key in one write; a lone Esc arrives alone.
            let alt = match buf.get(1) {
                Some(b'b') => Some(Key::Word(Move::Left)),
                Some(b'f') => Some(Key::Word(Move::Right)),
                Some(b'd') => Some(Key::DeleteWord(true)),
                Some(0x7f) | Some(0x08) => Some(Key::DeleteWord(false)),
                _ => None,
            };
            if let Some(key) = alt {
                buf.drain(..2);
                keys.push(key);
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
                return Some((i + 1, mouse(sgr, b == b'M')));
            }
            // `1;2` is Shift, `1;3` Alt, `1;4` Shift+Alt; other modifiers
            // keep the plain key.
            let shift = params == b"1;2";
            let (alt, shift_alt) = (params == b"1;3", params == b"1;4");
            let key = match b {
                b'C' if alt => Key::Word(Move::Right),
                b'D' if alt => Key::Word(Move::Left),
                b'C' if shift_alt => Key::SelectWord(Move::Right),
                b'D' if shift_alt => Key::SelectWord(Move::Left),
                b'A' if shift => Key::Select(Move::Up),
                b'B' if shift => Key::Select(Move::Down),
                b'C' if shift => Key::Select(Move::Right),
                b'D' if shift => Key::Select(Move::Left),
                b'H' if shift => Key::Select(Move::Home),
                b'F' if shift => Key::Select(Move::End),
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
                    b"3" | b"3;2" => Key::Delete,
                    b"3;3" => Key::DeleteWord(true),
                    // tmux, screen, and rxvt send Home and End this way.
                    b"1" | b"7" => Key::Home,
                    b"4" | b"8" => Key::End,
                    b"1;2" | b"7;2" => Key::Select(Move::Home),
                    b"4;2" | b"8;2" => Key::Select(Move::End),
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

/// An SGR mouse report `b;x;y`, ending `M` for a press and `m` for a release.
/// The wheel and a left press matter; releases, drags, and other buttons do not.
fn mouse(params: &[u8], press: bool) -> Key {
    let text = std::str::from_utf8(params).unwrap_or("");
    let mut parts = text.split(';').map(|p| p.parse::<u32>().ok());
    let button = parts.next().flatten().unwrap_or(0);
    let x = parts.next().flatten().unwrap_or(0) as u16;
    let y = parts.next().flatten().unwrap_or(0) as u16;
    // Bits 4, 8, and 16 are shift, alt, and ctrl held during the event.
    match button & !0b11100 {
        64 => Key::WheelUp,
        65 => Key::WheelDown,
        0 if press && x > 0 && y > 0 => Key::Click(x, y),
        32 if press => Key::Drag(x, y),
        0 if x > 0 && y > 0 => Key::Release(x, y),
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
    /// `/edit`: run this argument list in the foreground.
    Edit(Vec<String>),
    /// `/open`: start this argument list detached, without waiting.
    Open(Vec<String>),
    Prompt(String),
    /// Send this typed message again, dropping what followed it.
    Retry(String),
    Attach(Vec<crate::api::Image>),
    /// Drop the last unsent attachment.
    Detach,
    /// `/diff` with its arguments.
    Diff(String),
    SetThinking(bool),
    SetEffort(Option<String>),
    SetAgent(String),
    /// A setting changed in the `/settings` form, applied to the session.
    SetConfig(String, String),
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
    RevokeAllow { shape: String, always: bool },
    AllowAlways(String),
    SetRoot(std::path::PathBuf),
    Compact,
    /// Work across turns until the goal is met.
    Goal(String),
    GoalOff,
    /// Pause the goal after its current round.
    GoalPause,
    /// Change the goal's text; the round stays.
    GoalEdit(String),
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
    /// A turn the session started on its own, with a one-line summary.
    Woke(String),
    /// Output for the user, with its colours.
    Show(String),
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
    /// A command shape the user allowed, for this session or for good.
    Allow { shape: String, always: bool },
    /// A command shape a confirmation was shown for, and how it was answered.
    ShapeAsked { shape: String, allowed: bool },
    Usage {
        /// The context estimate, when this event carries one.
        tokens: Option<u64>,
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
        /// One per option; empty when it has none.
        descs: Vec<String>,
        multiple: bool,
        reply: Sender<Option<String>>,
    },
    Done(Option<String>),
    /// A failed request waits `secs` before retry `attempt` of `of`.
    Retry { secs: u64, attempt: u32, of: u32 },
    /// The config as re-read by `/reload`; the chat applies what changed.
    Config(Box<Config>),
    /// The user's likely next message, drawn greyed out in an empty input.
    Suggest(String),
    /// A summary of a long first message, as the session title. Ignored when
    /// the user has renamed the session.
    Title(String),
    /// A notice from a session's work: a finished job. The chat shows it as an
    /// info row, and a background session shows it as a toast.
    Notice(String),
    /// Changed files for `/diff`; true for `/diff git`.
    Diff(Vec<crate::agent::FileDiff>, bool),
    /// A goal round begins.
    GoalRound { text: String, round: u32, rounds: u32 },
    /// The goal ended, and why.
    GoalEnd(String),
    /// The goal stopped at a round boundary because it was paused.
    GoalPaused,
}

struct WorkerHooks {
    tx: Sender<Ev>,
    waited: Duration,
}

impl Hooks for WorkerHooks {
    fn allowed(&mut self, shape: &str, always: bool) {
        let _ = self.tx.send(Ev::Allow {
            shape: shape.to_string(),
            always,
        });
    }

    fn answered(&mut self, shape: &str, answer: Answer) {
        let _ = self.tx.send(Ev::ShapeAsked {
            shape: shape.to_string(),
            allowed: matches!(answer, Answer::Yes | Answer::Session | Answer::Always),
        });
    }

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
            Ui::Notice(t) => Ev::Notice(t),
            Ui::Retry { secs, attempt, of } => Ev::Retry { secs, attempt, of },
            Ui::Error(t) => Ev::Error(t),
            Ui::Show(t) => Ev::Show(t),
            Ui::Usage { hit, miss, output, model, context } => Ev::Usage { tokens: Some(context), hit, miss, output, model },
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
        attach(|| std::process::Command::new("sh").arg("-c").arg(command).status())
    }

    fn ask(&mut self, question: &str, options: &[String]) -> Option<String> {
        let plain: Vec<(String, String)> = options.iter().map(|o| (o.clone(), String::new())).collect();
        self.ask_choices(question, &plain, false)
    }

    fn can_foreground(&self) -> bool {
        true
    }

    fn ask_choices(&mut self, question: &str, options: &[(String, String)], multiple: bool) -> Option<String> {
        let (tx, rx) = channel();
        if self
            .tx
            .send(Ev::Question {
                q: question.to_string(),
                opts: options.iter().map(|(l, _)| l.clone()).collect(),
                descs: options.iter().map(|(_, d)| d.clone()).collect(),
                multiple,
                reply: tx,
            })
            .is_err()
        {
            return None;
        }
        // A question waits until answered, as a confirmation does.
        rx.recv().ok().flatten()
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
    /// For a diff row, one mark per character of `text`: its syntax kind in
    /// the low bits and `CHANGED` for a word that differs from its pair.
    marks: Vec<u8>,
}

/// The mark bit for a word that differs between a removed and an added line.
const CHANGED: u8 = 0x80;

/// One drawn row: plain text, or styled spans from the markdown renderer.
enum Line {
    Text(String),
    /// A tool row's last summary line, with its duration for the right edge.
    Timed(String, String),
    Spans(Vec<crate::render::Span>),
    /// A diff row and its marks, one per character.
    Diff(String, Vec<u8>),
}

/// One entry's rows at `cols`, led by a blank row when `gap` opens a turn.
fn entry_lines(entry: &Entry, cols: usize, markdown: bool, gap: bool) -> Vec<(Role, Line)> {
    if !entry.marks.is_empty() {
        return diff_rows(entry, cols);
    }
    let mut lines = Vec::new();
    if gap {
        lines.push((Role::User, Line::Text(String::new())));
    }
    if let Some(shown) = entry.text.strip_prefix(RAW) {
        // Info text lays itself out, as `/help` does, so it stays flush left.
        let indent = if entry.role == Role::Info { "" } else { "  " };
        for row in crate::render::raw_rows(shown, cols.saturating_sub(indent.len())) {
            let mut row = row;
            row.insert(0, crate::render::Span::styled(indent, crate::render::Style::default()));
            lines.push((entry.role, Line::Spans(row)));
        }
        return lines;
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

/// A diff entry cut into rows by character, so the marks stay aligned.
/// Continuations start under the code, past the indent and the sign.
/// Diffs go side by side from this many columns; narrower stays inline.
const SIDE_BY_SIDE_COLS: usize = 120;

/// Whether a diff is drawn side by side at this terminal width.
pub(crate) fn side_by_side(cols: usize) -> bool {
    cols >= SIDE_BY_SIDE_COLS
}

/// One cell of a side-by-side row: padded when short, cut with an ellipsis when long.
pub(crate) fn fit_cell(text: &str, width: usize) -> String {
    if text.chars().count() > width {
        let mut cut: String = text.chars().take(width.saturating_sub(1)).collect();
        cut.push('…');
        cut
    } else {
        format!("{text:<width$}")
    }
}

/// A side-by-side row for `cols` columns: old text, a divider, new text.
pub(crate) fn sbs_line(left: &str, right: &str, cols: usize) -> String {
    let left_w = cols.saturating_sub(3) / 2;
    let right_w = cols.saturating_sub(3 + left_w);
    format!("{} │ {}", fit_cell(left, left_w), fit_cell(right, right_w))
}

fn diff_rows(entry: &Entry, cols: usize) -> Vec<(Role, Line)> {
    let chars: Vec<char> = entry.text.chars().collect();
    let cols = cols.max(8);
    let mut rows = Vec::new();
    let mut at = 0;
    while at < chars.len() || rows.is_empty() {
        let lead = if at == 0 { 0 } else { 3 };
        let end = (at + cols - lead).min(chars.len());
        let text: String = " ".repeat(lead) + &chars[at..end].iter().collect::<String>();
        let mut marks = vec![0u8; lead];
        marks.extend((at..end).map(|i| entry.marks.get(i).copied().unwrap_or(0)));
        rows.push((entry.role, Line::Diff(text, marks)));
        at = end;
    }
    rows
}

/// Marks for diff lines: syntax kinds by extension, and the words that
/// differ in each removed line paired, by position, with an added line in
/// the run that follows it. One mark per character, the sign included.
fn diff_marks(lines: &[String], ext: &str) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = lines
        .iter()
        .map(|l| {
            let mut m = vec![0u8];
            m.extend(crate::syntax::kinds(ext, l.get(1..).unwrap_or("")));
            m.resize(l.chars().count().max(1), 0);
            m
        })
        .collect();
    let sign = |i: usize, c: char| lines.get(i).is_some_and(|l| l.starts_with(c));
    let mut i = 0;
    while i < lines.len() {
        let dels = (i..lines.len()).take_while(|&j| sign(j, '-')).count();
        let adds = (i + dels..lines.len()).take_while(|&j| sign(j, '+')).count();
        for k in 0..dels.min(adds) {
            let (a, b) = (i + k, i + dels + k);
            let (old, new) = (&lines[a][1..], &lines[b][1..]);
            // ponytail: O(n·m) on tokens; lines over 200 characters keep the plain tint.
            if old.chars().count() <= 200 && new.chars().count() <= 200 {
                let (gone, came) = changed_words(old, new);
                for c in gone {
                    out[a][c + 1] |= CHANGED;
                }
                for c in came {
                    out[b][c + 1] |= CHANGED;
                }
            }
        }
        i += (dels + adds).max(1);
    }
    out
}

/// The character positions of words in `old` and `new` outside their
/// longest common run of tokens: words, single punctuation, and spaces.
fn changed_words(old: &str, new: &str) -> (Vec<usize>, Vec<usize>) {
    fn tokens(s: &str) -> Vec<(usize, String)> {
        let mut out: Vec<(usize, String)> = Vec::new();
        for (i, c) in s.chars().enumerate() {
            let word = |c: char| c.is_alphanumeric() || c == '_';
            match out.last_mut() {
                Some((_, t)) if word(c) && t.chars().all(word) => t.push(c),
                Some((_, t)) if c == ' ' && t.chars().all(|x| x == ' ') => t.push(c),
                _ => out.push((i, c.to_string())),
            }
        }
        out
    }
    let (a, b) = (tokens(old), tokens(new));
    let mut dp = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            dp[i][j] = if a[i].1 == b[j].1 { dp[i + 1][j + 1] + 1 } else { dp[i + 1][j].max(dp[i][j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let (mut gone, mut came) = (Vec::new(), Vec::new());
    let spread = |t: &(usize, String), out: &mut Vec<usize>| {
        if !t.1.starts_with(' ') {
            out.extend(t.0..t.0 + t.1.chars().count());
        }
    };
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i].1 == b[j].1 {
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || dp[i][j + 1] >= dp[i + 1][j]) {
            spread(&b[j], &mut came);
            j += 1;
        } else {
            spread(&a[i], &mut gone);
            i += 1;
        }
    }
    (gone, came)
}

/// The extension of the `path` in a tool call's arguments, lower case.
fn args_ext(args: &str) -> String {
    let path = serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(str::to_string))
        .unwrap_or_default();
    path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

/// Starts an entry whose text is shown output with its own colours.
const RAW: char = '\u{2}';

/// Separates a tool summary from its duration in the stored row text.
const TIME: char = '\u{1}';

/// The configured spinner, as an index into `SPINNERS`.
// ponytail: global so every free function can read it; one choice per process.
static SPIN: AtomicUsize = AtomicUsize::new(0);

fn spinner() -> (&'static str, &'static str) {
    crate::config::SPINNERS[SPIN.load(Ordering::Relaxed) % crate::config::SPINNERS.len()]
}

/// The frames of the configured spinner.
fn spinner_frames() -> &'static str {
    spinner().1
}

fn spinner_frame(tick: usize) -> char {
    let (name, frames) = spinner();
    let n = frames.chars().count();
    // A cheap scramble of the tick keeps `random` stable within a frame.
    let i = if name == "random" { ((tick as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as usize } else { tick };
    frames.chars().nth(i % n).unwrap_or(' ')
}

/// Put a spinner frame before the count of each working session's row, and
/// two spaces before the rest, so titles keep their width.
/// An editable text row for a form: `prefix`, the text, and a caret bar at byte
/// `caret` when the field is active. Returns the row and the caret's column.
fn text_row(prefix: &str, text: &str, caret: Option<usize>) -> (String, usize) {
    let Some(caret) = caret else {
        return (format!("{prefix}{text}"), prefix.chars().count());
    };
    let split = caret.min(text.len());
    let (before, after) = text.split_at(split);
    let col = prefix.chars().count() + before.chars().count();
    (format!("{prefix}{before}▏{after}"), col)
}

/// A select row for a form: `prefix` then the current choice between arrows.
fn select_row(prefix: &str, value: &str) -> String {
    format!("{prefix}‹ {value} ›")
}

/// The mark for one session, by priority: waiting for the user, running (the
/// spinner frame), failed, then a reply not yet viewed. `None` when idle.
fn state_mark(waiting: bool, streaming: bool, failed: bool, replied: bool, frame: char) -> Option<char> {
    if waiting {
        Some('?')
    } else if streaming {
        Some(frame)
    } else if failed {
        Some('!')
    } else if replied {
        Some('•')
    } else {
        None
    }
}

/// Whether a character is one of the marks a session row can start with.
fn is_mark(c: char) -> bool {
    matches!(c, '?' | '!' | '•') || spinner_frames().contains(c)
}

/// The description without a leading mark, so a repaint never stacks marks.
fn strip_mark(desc: &str) -> &str {
    let desc = desc.trim_start();
    let mut chars = desc.chars();
    match (chars.next(), chars.next()) {
        (Some(c), Some(' ')) if is_mark(c) => desc[c.len_utf8() + 1..].trim_start(),
        _ => desc,
    }
}

/// Write each session row's mark in front of its description. A session
/// without a mark gets two spaces, so the columns line up.
fn mark_rows(items: &mut [PickItem], marks: &[(String, char)]) {
    for item in items {
        let plain = strip_mark(&item.desc).to_string();
        item.desc = match marks.iter().find(|(id, _)| *id == item.value) {
            Some((_, mark)) => format!("{mark} {plain}"),
            None => format!("  {plain}"),
        };
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
    finish: Finish,
    ms: u128,
    running: bool,
    expanded: bool,
    started: Instant,
}

fn preview_detail(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.lines().take(12).map(str::to_string).collect();
    let total = text.lines().count();
    if total > 12 {
        lines.push(format!("… +{} lines (Ctrl+E)", total - 12));
    }
    lines
}

/// A result that only restates the step past tense, word for word after the verb.
fn echoes(summary: &str, preview: &str) -> bool {
    matches!((summary.split_once(' '), preview.split_once(' ')), (Some((_, a)), Some((_, b))) if a == b)
}

/// A call's arguments as a person reads them: `$ command` for a shell call,
/// then `key: value` lines, strings unquoted. The description is the row
/// itself, so it is left out. Anything but an object stays as it came.
fn readable_args(args: &str) -> Vec<String> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str(args) else {
        return vec![args.to_string()];
    };
    let mut lines = Vec::new();
    if let Some(cmd) = map.get("command").and_then(|v| v.as_str()) {
        lines.push(format!("$ {cmd}"));
    }
    // The path says where; the rest follow in key order.
    if let Some(path) = map.get("path").and_then(|v| v.as_str()) {
        lines.push(format!("path: {path}"));
    }
    for (key, value) in &map {
        let shown = matches!(key.as_str(), "command" | "path") && value.is_string();
        if key == "description" || shown {
            continue;
        }
        match value.as_str() {
            Some(text) => lines.push(format!("{key}: {text}")),
            None => lines.push(format!("{key}: {value}")),
        }
    }
    lines
}

/// How a tool step ended, shared by both front-ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Finish {
    Done,
    /// A command ran and exited non-zero: a result, not a breakage.
    Exited,
    Declined,
    Failed,
}

impl Finish {
    /// Every command result starts `exit N`, so one reading serves both
    /// front-ends; every decline starts `refused: the user declined`.
    pub(crate) fn of(ok: bool, output: &str) -> Self {
        let code = output.lines().next().and_then(|l| l.strip_prefix("exit ")?.trim().parse::<i32>().ok());
        if ok {
            Self::Done
        } else if output.starts_with("refused: the user declined") {
            Self::Declined
        } else if code.is_some_and(|n| n > 0) {
            Self::Exited
        } else {
            Self::Failed
        }
    }

    pub(crate) fn mark(self) -> &'static str {
        match self {
            Self::Done => "✓",
            Self::Exited => "!",
            Self::Declined | Self::Failed => "✗",
        }
    }

    pub(crate) fn role(self) -> Role {
        match self {
            Self::Done | Self::Exited => Role::Tool,
            Self::Declined => Role::Context,
            Self::Failed => Role::Error,
        }
    }
}

fn tool_row_text(row: &ToolRow, level: Verbosity) -> String {
    let mark = row.finish.mark();
    let ok = row.finish == Finish::Done;
    // At medium a success says what came back, on the same line.
    let result = match level {
        // `edit x (1 change)` → `edited x (1 change)` says it twice.
        Verbosity::Medium if ok && !row.preview.is_empty() && !row.expanded && !echoes(&row.summary, &row.preview) => {
            format!(" → {}", row.preview)
        }
        _ => String::new(),
    };
    let mut text = format!("{mark} {}{result}{TIME}{}", row.summary, duration(row.ms));
    if row.expanded || level == Verbosity::High {
        // Capped like the output, so a `write` row is not the whole file.
        let args = readable_args(&row.args).join("\n");
        let total = args.lines().count();
        for line in args.lines().take(20) {
            text.push_str("\n  ");
            text.push_str(line);
        }
        if total > 20 {
            text.push_str(&format!("\n  … +{} lines", total - 20));
        }
        if !row.output.is_empty() {
            text.push_str("\n  ");
            text.push_str(&row.output.lines().take(20).collect::<Vec<_>>().join("\n  "));
        }
    } else if row.finish == Finish::Declined {
        // The model reads why; the user knows, so one word is enough.
        text.push_str("\ndeclined");
    } else if row.finish == Finish::Exited {
        // Errors come last, so the tail says more than the head.
        let mut lines = row.output.lines();
        let code = lines.next().unwrap_or("");
        let body: Vec<&str> = lines.filter(|l| !l.trim().is_empty()).collect();
        let hidden = body.len().saturating_sub(3);
        text.push('\n');
        text.push_str(code);
        if hidden > 0 {
            text.push_str(&format!(" · {hidden} more above (Ctrl+E)"));
        }
        for line in &body[hidden..] {
            text.push('\n');
            text.push_str(line);
        }
    } else if !row.preview.is_empty() && !ok {
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
    /// Where a Shift move began; the selection runs from here to the caret.
    anchor: Option<usize>,
    /// The open question's option descriptions, whether it takes several
    /// picks, and which are marked.
    ask_descs: Vec<String>,
    ask_multiple: bool,
    ask_marked: Vec<bool>,
    /// The caret in a typed answer to the open question.
    // ponytail: one for the app, not saved per session; clamped where used,
    // so a switch mid-answer moves the caret, never past the text.
    ask_caret: usize,
    /// Messages waiting for the running turn to end, oldest first.
    queued: Vec<String>,
    /// The queued message open in the composer, while one is.
    queue_at: Option<usize>,
    /// The stored title: set by `/rename`, or cut from the first message.
    title: String,
    /// Where the caret sits in the provider form's active field, in bytes.
    form_caret: usize,
    /// The last turn ended with an error, until the next one finishes.
    failed: bool,
    /// The turn ended while a queued message was open; send on close.
    queue_due: bool,
    /// Shared with the session: messages that join the running turn, and
    /// the flag that holds it while a queued message is edited.
    steer: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    hold: std::sync::Arc<std::sync::atomic::AtomicBool>,
    history: Vec<String>,
    hist_pos: Option<usize>,
    follow: bool,
    top: usize,
    body_rows: usize,
    max_top: usize,
    /// The time the title bar last showed, to repaint on a new minute.
    /// The provider-has-no-effort note was shown this run.
    effort_noted: bool,
    clock_shown: String,
    unseen: usize,
    pending: Option<Pending>,
    /// The theme active when the theme picker opened, while it previews.
    preview_from: Option<&'static crate::theme::Theme>,
    /// Where managed provider profiles are written.
    config_path: std::path::PathBuf,
    /// Where recently chosen models are remembered.
    models_path: std::path::PathBuf,
    /// The provider's models, catalog first and then whatever it reported.
    available_models: Vec<String>,
    /// Messages parked for this session, never persisted.
    bell_pending: bool,
    status: Status,
    verbosity: Verbosity,
    root: std::path::PathBuf,
    agent: String,
    /// The agent Tab chose during a turn, applied when it ends.
    agent_next: Option<String>,
    /// Agent profiles as name and description.
    agents: Vec<(String, String)>,
    sessions_dir: std::path::PathBuf,
    last_redraw: Instant,
    /// The rows last written, the size they were written at, and whether an
    /// overlay was drawn over them.
    shown: Vec<String>,
    shown_size: (usize, usize),
    shown_overlay: bool,
    last_size: (usize, usize),
    dirty: bool,
    quit: bool,
    /// When an idle Ctrl+C asked for a second one to quit.
    quit_armed: Option<Instant>,
    spinner: usize,
    /// Rendered rows per entry, keyed by a hash of what shapes them.
    drawn: Vec<(u64, Vec<(Role, Line)>)>,
    /// What a click on the last frame lands on: row, first and last column
    /// (1-based, inclusive), and the target.
    targets: Vec<(usize, usize, usize, Target)>,
    /// `[a] always` was clicked once; a second click on it applies.
    armed: bool,
    /// Command shapes allowed here, with whether each is saved for good.
    allows: Vec<(String, bool)>,
    /// Shapes a confirmation was shown for, with whether they were allowed.
    recent: Vec<(String, bool)>,
    /// First run: a missing key is being entered, so Esc quits.
    onboard: bool,
    /// The latest copy, pasted by Ctrl+V.
    clip: String,
    /// The command that prints a clipboard image, if this session has one.
    paste_cmd: Option<String>,
    /// The "cannot paste images here" note has been shown.
    paste_noted: bool,
    /// An OSC 52 sequence for the terminal clipboard, written after the next frame.
    osc52: Option<String>,
    /// Ctrl+G handed the mouse to the terminal until the next key.
    released: bool,
    /// A drag selection: where the press began and where the pointer is.
    drag: Option<(u16, u16, u16, u16)>,
    /// Mouse reporting to switch on or off with the next frame.
    mouse_seq: Option<&'static str>,
    /// The newest tool row; `done` holds the latest turn's earlier ones.
    tool: Option<ToolRow>,
    done: Vec<ToolRow>,
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
    /// Where a Shift move began; the selection runs from here to the caret.
    anchor: Option<usize>,
    /// The open question's option descriptions, whether it takes several
    /// picks, and which are marked.
    ask_descs: Vec<String>,
    ask_multiple: bool,
    ask_marked: Vec<bool>,
    /// Messages waiting for the running turn to end, oldest first.
    queued: Vec<String>,
    /// The queued message open in the composer, while one is.
    queue_at: Option<usize>,
    /// The stored title: set by `/rename`, or cut from the first message.
    title: String,
    /// The last turn ended with an error, until the next one finishes.
    failed: bool,
    /// The turn ended while a queued message was open; send on close.
    queue_due: bool,
    /// Shared with the session: messages that join the running turn, and
    /// the flag that holds it while a queued message is edited.
    steer: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    hold: std::sync::Arc<std::sync::atomic::AtomicBool>,
    hist_pos: Option<usize>,
    follow: bool,
    top: usize,
    max_top: usize,
    unseen: usize,
    pending: Option<Pending>,
    bell_pending: bool,
    status: Status,
    tool: Option<ToolRow>,
    done: Vec<ToolRow>,
    root: std::path::PathBuf,
    agent: String,
    /// The agent Tab chose during a turn, applied when it ends.
    agent_next: Option<String>,
    worker: Option<Worker>,
    touched: Instant,
    replied: bool,
    since: Instant,
    allows: Vec<(String, bool)>,
    recent: Vec<(String, bool)>,
}

impl Pane {
    fn new(session: &Session, worker: Option<Worker>) -> Self {
        let (session_steer, session_hold) = (session.steer.clone(), session.hold.clone());
        let allows = session.cfg.always_allows().into_iter().map(|s| (s, true)).collect();
        Self {
            cfg: session.cfg.clone(),
            cancel: session.cancel.clone(),
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            anchor: None,
            ask_descs: Vec::new(),
            ask_multiple: false,
            ask_marked: Vec::new(),
            queued: Vec::new(),
            queue_at: None,
            title: session.log.as_ref().map(|log| log.meta().title).unwrap_or_default(),
            failed: false,
            queue_due: false,
            steer: session_steer.clone(),
            hold: session_hold.clone(),
            hist_pos: None,
            follow: true,
            top: 0,
            max_top: 0,
            unseen: 0,
            pending: None,
            bell_pending: false,
            status: Status::default(),
            tool: None,
            done: Vec::new(),
            root: session.tools.root.clone(),
            agent: session.agent.name.clone(),
            agent_next: None,
            worker,
            touched: Instant::now(),
            replied: false,
            since: Instant::now(),
            allows,
            recent: Vec::new(),
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
    /// One text or number setting, typed in the settings modal. `error` holds the
    /// reason the config refused the last value.
    SettingField {
        key: String,
        value: String,
        error: String,
    },
    /// `/goal <text>` while a goal runs: replace it only after a yes.
    GoalReplace {
        text: String,
    },
    /// A typed command shape for an `allow` line.
    AllowRule {
        value: String,
    },
    /// A first-run key: masked while typed, then checked before it is saved.
    KeyEntry {
        provider: String,
        var: String,
        value: String,
        /// A check result or error, shown beside the field.
        status: String,
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
    Palette,
    /// The `/settings` form: each row steps its own value.
    Settings,
    /// The models offered for a provider's model field.
    ProviderModel,
    /// `/skills`: each skill on or off; a choice flips it and saves the config.
    Skill,
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
    /// The provider form to return to when this picker closes, as
    /// `(name, fields, at)`. Set only by the model picker of a provider form.
    form: Option<Box<(String, Vec<(String, String)>, usize)>>,
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

/// The rows of the `/settings` form: config key and label.
const SETTING_ROWS: &[(&str, &str)] = &[
    ("permissions", "start mode"),
    ("thinking", "thinking"),
    ("effort", "effort"),
    ("verbosity", "verbosity"),
    ("mouse", "mouse"),
    ("suggest", "suggestions"),
    ("diffs", "diffs"),
    ("auto_next", "next waiting session"),
    ("title_gradient", "title gradient"),
    ("spinner", "spinner"),
    ("allow_sudo", "allow sudo"),
    ("color", "colour"),
    ("links", "links"),
    ("title_summary", "title summary"),
    ("api_key", "API key"),
    ("model", "model"),
    ("base_url", "base URL"),
    ("context_window", "context window"),
    ("max_tokens", "max tokens"),
    ("max_steps", "max steps"),
    ("goal_rounds", "goal rounds"),
    ("compact_at", "compact at"),
    ("prune_at", "prune at"),
    ("skills_dir", "skills folder"),
];

/// The stream dialects a provider form offers, in the order they step.
const DIALECTS: &[&str] = &["deepseek", "openai", "anthropic", "gemini", "ollama", "custom"];

/// The row of the model picker that types a name instead of choosing one.
const TYPE_ANOTHER: &str = "::type-another";

/// Settings a row edits in a one-field form rather than stepping.
fn is_text_setting(key: &str) -> bool {
    matches!(
        key,
        "model" | "base_url" | "context_window" | "max_tokens" | "max_steps" | "goal_rounds" | "compact_at" | "prune_at" | "skills_dir"
    )
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
/// Hand the terminal to a foreground child, then take it back. `launch` runs the
/// child; the caller decides whether it is a shell or an argument list.
fn attach(launch: impl FnOnce() -> std::io::Result<std::process::ExitStatus>) -> Option<i32> {
    // Stop the key reader before the command takes the terminal.
    ATTACHED.store(true, std::sync::atomic::Ordering::SeqCst);
    let asked = Instant::now();
    while !PARKED.load(std::sync::atomic::Ordering::SeqCst) && asked.elapsed() < Duration::from_millis(500) {
        std::thread::sleep(Duration::from_millis(5));
    }
    // The child gets the mouse back, and imp takes it again after.
    print!("{MOUSE_OFF}\x1b[?2004l\x1b[?25h\x1b[?1049l");
    std::io::stdout().flush().ok();
    let saved = SAVED.lock().ok().and_then(|slot| *slot);
    if let Some(saved) = saved {
        unsafe {
            tcsetattr(0, TCSANOW, &saved);
        }
    }
    let status = launch();
    if let Some(saved) = saved {
        unsafe {
            tcsetattr(0, TCSANOW, &raw_of(saved));
        }
    }
    print!("{}", enter_sequence(MOUSE.load(Ordering::Relaxed)).replace("\x1b[>1u", ""));
    std::io::stdout().flush().ok();
    ATTACHED.store(false, std::sync::atomic::Ordering::SeqCst);
    Some(status.ok().and_then(|s| s.code()).unwrap_or(-1))
}

/// Whether an editor target may open: it must exist, and lie inside the session
/// directory. Refused otherwise, rather than asked, for now.
pub(crate) fn check_target(root: &std::path::Path, target: &str) -> Result<(), String> {
    let Ok(path) = root.join(target).canonicalize() else {
        return Err(format!("{target} does not exist"));
    };
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    if path.starts_with(&root) {
        Ok(())
    } else {
        Err(format!("{target} is outside the session directory"))
    }
}

/// Run an argument list in the foreground, without a shell: the first entry is
/// the program, the rest its arguments.
fn attach_argv(argv: &[String]) -> Option<i32> {
    let (program, args) = argv.split_first()?;
    attach(|| std::process::Command::new(program).args(args).status())
}

/// The argument list for `/edit` (terminal) or `/open` (GUI): the configured
/// editor, else the environment's, else for `/open` the OS opener. The target is
/// one argument; nothing goes through a shell.
pub(crate) fn editor_argv_for(
    cfg: &crate::config::Config,
    lookup: impl Fn(&str) -> Option<String>,
    gui: bool,
    target: &str,
) -> Result<Vec<String>, String> {
    let argv = match cfg.editor_argv(&lookup, gui) {
        Some(argv) => argv,
        None if gui => vec![if cfg!(target_os = "macos") { "open" } else { "xdg-open" }.to_string()],
        None => return Err("no editor set: set editor in the config or $EDITOR".into()),
    };
    Ok(crate::config::editor_launch(argv, target))
}

fn mark_current(current: bool, about: &str) -> String {
    match (current, about.is_empty()) {
        (true, true) => "current".into(),
        (true, false) => format!("current · {about}"),
        (false, _) => about.to_string(),
    }
}

/// One list row as drawn lines: mark, label, and an aligned description when
/// the picker has any. A wrapped row continues its description on a second
/// line under the description column; every other row is cut with `…`.
fn picker_row(picker: &Picker, at: usize, index: usize, inner: usize, more: bool) -> Vec<String> {
    let item = &picker.items[index];
    let marker = if at == picker.index { "▸" } else { " " };
    let Some(start) = desc_column(picker, inner) else {
        return vec![format!("{marker} {}", item.label)];
    };
    let left = start - 4;
    let mut label = item.label.clone();
    let mut desc = item.desc.clone();
    if !matches!(picker.kind, PickKind::Command | PickKind::Keys | PickKind::Palette) {
        label = fit(&label, left);
        if picker.kind == PickKind::Session {
            // Counts line up on their last digit.
            let wide = picker.items.iter().map(|i| i.desc.chars().count()).max().unwrap_or(0);
            desc = format!("{desc:>wide$}");
        }
    }
    let room = inner.saturating_sub(start);
    if more && room >= 8 && picker.kind != PickKind::Session && desc.chars().count() > room {
        let mut parts = wrap(&desc, room);
        if parts.len() > 2 {
            // Two lines are all a row may take; the rest is cut.
            let tail = parts[2..].join(" ");
            parts.truncate(2);
            parts[1] = fit(&format!("{} {}", parts[1], tail), room);
        }
        if let [first, rest] = parts.as_slice() {
            return vec![
                format!("{marker} {label:<left$}  {first}"),
                format!("{}{}", " ".repeat(start), rest),
            ];
        }
    }
    vec![styled_state(&desc, format!("{marker} {label:<left$}  {desc}"))]
}

/// A provider row's state, from its description: switched off is struck
/// through, and unconfigured is dimmed. Other rows keep their plain text.
fn styled_state(desc: &str, row: String) -> String {
    if desc.ends_with("· off") {
        format!("\x1b[9m{row}\x1b[0m")
    } else if desc.ends_with("· no key") {
        format!("\x1b[2m{row}\x1b[0m")
    } else {
        row
    }
}

/// The column a picker's descriptions start at, or `None` when it has none.
fn desc_column(picker: &Picker, inner: usize) -> Option<usize> {
    if !picker.items.iter().any(|item| !item.desc.is_empty()) {
        return None;
    }
    let longest = picker.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0);
    // Names are identifiers: the label column fits the longest one, and the
    // description gives way instead.
    let names = matches!(picker.kind, PickKind::Provider | PickKind::Model | PickKind::Agent | PickKind::Theme | PickKind::Mode);
    let mut left = if names { longest + 2 } else { longest.max(10) + 2 };
    if !names && !matches!(picker.kind, PickKind::Command | PickKind::Keys | PickKind::Palette) {
        // The right column stays whole, one space off the wall; the
        // label gives way.
        let wide = picker.items.iter().map(|i| i.desc.chars().count()).max().unwrap_or(0);
        left = left.min(inner.saturating_sub(2 + 2 + wide + 1)).max(4);
    }
    Some(2 + left + 2)
}

/// The visible list as drawn lines, each with its item index and highlight.
fn picker_lines(picker: &Picker, matches: &[usize], visible: usize, inner: usize, wrap: bool) -> Vec<(String, usize, bool)> {
    if matches.is_empty() {
        return vec![("  no matches".into(), 0, false)];
    }
    let mut out = Vec::new();
    for row in 0..visible {
        let at = picker.offset + row;
        let Some(&index) = matches.get(at) else { break };
        let selected = at == picker.index;
        // Provider rows wrap whether selected or not, so moving the selection
        // never changes the list's height.
        let wraps = wrap && (selected || picker.kind == PickKind::Provider);
        for line in picker_row(picker, at, index, inner, wraps) {
            out.push((line, at, selected));
        }
    }
    out
}

/// A first-run check's verdict.
#[derive(Debug, PartialEq)]
enum KeyOutcome {
    /// Save the key; a warning means the check itself did not run.
    Save(Option<String>),
    /// The provider refused the key, so the row stays open.
    Retry(String),
}

/// A refused key is a 4xx; anything else is the network, which must not lock
/// anyone out of their own machine.
fn first_run_outcome(result: Result<(), String>) -> KeyOutcome {
    match result {
        Ok(()) => KeyOutcome::Save(None),
        Err(e) if e.contains("HTTP 4") => KeyOutcome::Retry(e),
        Err(e) => KeyOutcome::Save(Some(format!("key saved; the check did not run ({e})"))),
    }
}

/// Actions the palette lists with the shortcut that also runs them.
const PALETTE_ACTIONS: &[(&str, &str, &str)] = &[
    ("next session", "Ctrl+N", "next-session"),
    ("stop the turn", "Esc", "stop"),
    ("expand tools", "Ctrl+E", "expand"),
    ("clear the view", "Ctrl+L", "clear"),
    ("scroll up", "PgUp", "page-up"),
    ("scroll down", "PgDn", "page-down"),
    ("release the mouse", "Ctrl+G", "release"),
    ("steer now", "Ctrl+S", "steer"),
    ("paste", "Ctrl+V", "paste"),
    ("quit", "Ctrl+D", "quit"),
];

/// Commands a key does exactly; the palette shows the key after the description.
const COMMAND_KEYS: &[(&str, &str)] = &[("/keys", "?"), ("/next", "Ctrl+O"), ("/think", "Ctrl+T"), ("/retry", "Ctrl+R")];

/// Commands whose argument is typed rather than picked.
const FREE_TEXT: &[&str] = &["queue", "rename", "diff"];

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
            form: None,
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
            PickKind::Settings => {
                return "Enter step or edit · Esc close · more in the config file".into()
            }
            _ => vec!["Enter choose".into(), "Esc close".into()],
        };
        // Typed `/provider` filters with letters, so the key there is Ctrl+E.
        let typed_provider = self.kind == PickKind::Provider && self.prefix.is_some();
        if typed_provider {
            if self.selected().is_some() {
                parts.push("^U use".into());
                parts.push("^E configure".into());
            }
        } else if let Some(item) = self.selected().or_else(|| self.items.first()) {
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
    /// A confirmation shown in place of the status line, and when.
    toast: Option<(String, Instant)>,
    /// A retry's countdown: seconds left, attempt, and how many.
    retry: Option<(u64, u32, u32)>,
    tokens: u64,
    hit: u64,
    miss: u64,
    output: u64,
    served: String,
    streaming: bool,
    /// A likely next message, shown while the input is empty.
    suggestion: Option<String>,
    /// When the running task began; `None` while idle.
    task_start: Option<Instant>,
    steps: usize,
    /// Streamed characters, for an output estimate before usage arrives.
    chars: usize,
    /// The task's own hit, miss, and output tokens.
    task_usage: (u64, u64, u64),
    /// Since when nothing has streamed and no tool has run.
    quiet_since: Option<Instant>,
    /// The running goal: its text, round, and rounds.
    goal: Option<(String, u32, u32)>,
    /// The goal is paused: kept, but no round runs.
    goal_paused: bool,
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
        let verbosity = cfg.verbosity;
        let allows = cfg.always_allows().into_iter().map(|s| (s, true)).collect();
        SPIN.store(cfg.spinner, Ordering::Relaxed);
        let history = load_history();
        // Tests see one agent whatever the machine has installed.
        let agents = if cfg!(test) {
            vec![("default".to_string(), String::new())]
        } else {
            crate::store::load_agents(&cfg, &root).into_iter().map(|a| (a.name, a.description)).collect()
        };
        let sessions_dir = crate::config::dirs_state().join("imp").join("sessions");
        // Handed to the session when its worker starts.
        let session_steer = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let session_hold = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self {
            cfg,
            cancel,
            cols,
            palette,
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            anchor: None,
            ask_descs: Vec::new(),
            ask_multiple: false,
            ask_marked: Vec::new(),
            ask_caret: 0,
            queued: Vec::new(),
            queue_at: None,
            title: String::new(),
            form_caret: 0,
            failed: false,
            queue_due: false,
            steer: session_steer.clone(),
            hold: session_hold.clone(),
            history,
            hist_pos: None,
            follow: true,
            top: 0,
            body_rows: 1,
            max_top: 0,
            effort_noted: false,
            clock_shown: String::new(),
            unseen: 0,
            pending: None,
            preview_from: None,
            config_path: Config::config_path(),
            models_path: crate::config::dirs_state().join("imp").join("models"),
            available_models: crate::providers::catalog(provider),
            bell_pending: false,
            status: Status {
                tokens: 0,
                hit: 0,
                miss: 0,
                output: 0,
                served: String::new(),
                streaming: false,
                ..Default::default()
            },
            verbosity,
            root,
            agent,
            agent_next: None,
            agents,
            sessions_dir,
            last_redraw: Instant::now() - Duration::from_secs(1),
            shown: Vec::new(),
            shown_size: (0, 0),
            shown_overlay: false,
            last_size: (0, 0),
            dirty: true,
            quit: false,
            quit_armed: None,
            spinner: 0,
            drawn: Vec::new(),
            targets: Vec::new(),
            armed: false,
            allows,
            recent: Vec::new(),
            onboard: false,
            clip: String::new(),
            // Tests never read the real clipboard.
            paste_cmd: if cfg!(test) { None } else { crate::images::clipboard_command() },
            paste_noted: false,
            osc52: None,
            released: false,
            drag: None,
            mouse_seq: None,
            tool: None,
            done: Vec::new(),
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
        swap(&mut self.anchor, &mut p.anchor);
        swap(&mut self.ask_descs, &mut p.ask_descs);
        swap(&mut self.ask_multiple, &mut p.ask_multiple);
        swap(&mut self.ask_marked, &mut p.ask_marked);
        swap(&mut self.queued, &mut p.queued);
        swap(&mut self.queue_at, &mut p.queue_at);
        swap(&mut self.title, &mut p.title);
        swap(&mut self.failed, &mut p.failed);
        swap(&mut self.queue_due, &mut p.queue_due);
        swap(&mut self.steer, &mut p.steer);
        swap(&mut self.hold, &mut p.hold);
        swap(&mut self.hist_pos, &mut p.hist_pos);
        swap(&mut self.follow, &mut p.follow);
        swap(&mut self.top, &mut p.top);
        swap(&mut self.max_top, &mut p.max_top);
        swap(&mut self.unseen, &mut p.unseen);
        swap(&mut self.pending, &mut p.pending);
        swap(&mut self.bell_pending, &mut p.bell_pending);
        swap(&mut self.status, &mut p.status);
        swap(&mut self.tool, &mut p.tool);
        swap(&mut self.done, &mut p.done);
        swap(&mut self.root, &mut p.root);
        swap(&mut self.agent, &mut p.agent);
        swap(&mut self.agent_next, &mut p.agent_next);
        swap(&mut self.worker, &mut p.worker);
        swap(&mut self.touched, &mut p.touched);
        swap(&mut self.replied, &mut p.replied);
        swap(&mut self.since, &mut p.since);
        swap(&mut self.allows, &mut p.allows);
        swap(&mut self.recent, &mut p.recent);
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
        self.dirty = true;
    }

    /// Apply a worker event to a pane out of view, through the same code.
    fn apply_to(&mut self, index: usize, ev: Ev) {
        if index == self.at {
            return self.apply(ev);
        }
        let Some(mut p) = self.panes[index].take() else { return };
        self.swap_pane(&mut p);
        // What the background session needs the user to know, named while its
        // title is in view; the toast is set once the pane is back in place.
        let note = match &ev {
            Ev::Done(None) => Some("replied".to_string()),
            Ev::Done(Some(err)) => Some(format!("error: {err}")),
            Ev::Confirm { .. } | Ev::Question { .. } => Some("needs an answer".to_string()),
            Ev::Woke(what) => Some(format!("⏰ {what}")),
            Ev::Notice(text) => Some(text.clone()),
            _ => None,
        };
        let toast = note.map(|note| format!("{}/{} {}: {note}", index + 1, self.panes.len(), self.session_name()));
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
        if let Some(text) = toast {
            self.toast(text);
        }
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

    /// Diff rows for a file with extension `ext`, marked for syntax and for
    /// the words that changed.
    fn push_diffs(&mut self, lines: Vec<String>, ext: &str) {
        let marks = diff_marks(&lines, ext);
        for (line, m) in lines.into_iter().zip(marks) {
            let role = match line.chars().next() {
                Some('+') => Role::Add,
                Some('-') => Role::Delete,
                _ => Role::Context,
            };
            let marks = if matches!(role, Role::Add | Role::Delete) { [vec![0, 0], m].concat() } else { Vec::new() };
            self.entries.push(Entry { role, text: format!("  {line}"), marks });
        }
        self.dirty = true;
    }

    /// A new session from the factory, with the config as `/reload` left it
    /// rather than as it was at startup. `None` without a factory.
    fn fresh_session(&self) -> Option<Session> {
        let mut session = (self.fresh.as_ref()?)(&self.root);
        session.cfg.reload_from(&self.cfg);
        Some(session)
    }

    /// Start another live session, or reset this one without a factory.
    fn new_session(&mut self, tx: &Sender<Cmd>) {
        let Some(mut session) = self.fresh_session() else {
            let _ = tx.send(Cmd::Reset);
            self.clear_entries();
            self.push(Role::Info, "new conversation");
            return;
        };
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
        self.clear_entries();
        self.push(Role::Info, "new conversation");
    }

    /// Resume a stored session in another live pane, or in this one.
    fn resume(&mut self, id: String, tx: &Sender<Cmd>) {
        let Some(mut session) = self.fresh_session() else {
            self.clear_entries();
            self.push(Role::Info, format!("resuming {id}"));
            let _ = tx.send(Cmd::Resume(id));
            return;
        };
        match session.resume_id(&self.sessions_dir, &id) {
            Ok(()) => {
                if self.open(session) {
                    self.push(Role::Info, format!("resumed {id}"));
                }
            }
            Err(e) => self.push(Role::Error, e),
        }
    }

    /// Empty the transcript, and the tool rows that point into it.
    /// A paste goes whole into what takes text: the draft keeps its line
    /// breaks, a prompt or answer row gets spaces, a confirmation gets nothing.
    fn paste(&mut self, text: String, tx: &Sender<Cmd>) {
        self.dirty = true;
        let flat: String = text.chars().map(|c| if c == '\n' || c == '\t' { ' ' } else { c }).filter(|c| !c.is_control()).collect();
        match &mut self.pending {
            // A picker's letters can act on rows, such as `d` revoking an allow.
            Some(Pending::Confirm { .. } | Pending::ConfirmDelete { .. } | Pending::Pick(_)) => {}
            Some(Pending::Question { mode, typed, .. }) => {
                *mode = AskMode::Custom;
                let at = self.ask_caret.min(typed.len());
                typed.insert_str(at, &flat);
                self.ask_caret = at + flat.len();
            }
            Some(_) => {
                for c in flat.chars() {
                    self.handle_key_inner(Key::Char(c), tx);
                }
            }
            None => {
                self.follow_tail();
                if let Some((from, to)) = self.selection() {
                    self.input.replace_range(from..to, "");
                    self.cursor = from;
                }
                self.anchor = None;
                self.input.insert_str(self.cursor, &text);
                self.cursor += text.len();
                self.sync_menu();
            }
        }
    }

    /// Drop the rows after the last user message and send it again.
    fn retry(&mut self, tx: &Sender<Cmd>) {
        if self.status.streaming {
            return self.push(Role::Info, "a turn is running");
        }
        // Only what the user typed: `!cmd` rows never reach the agent, and a
        // goal's later rounds are messages the chat does not show.
        let typed = |e: &Entry| e.role == Role::User && !e.text.starts_with('!') && !e.text.starts_with("/goal ");
        let Some(i) = self.entries.iter().rposition(typed) else {
            return self.push(Role::Info, "nothing to retry");
        };
        let text = self.entries[i].text.trim_end_matches(" (steered)").to_string();
        // ponytail: rows drop before the agent confirms; if compaction took the
        // message, the view is shorter than the history until the next turn.
        self.entries.truncate(i + 1);
        self.tool = None;
        self.done.retain(|r| r.index <= i);
        self.follow_tail();
        self.status.streaming = true;
        self.begin_task();
        let _ = tx.send(Cmd::Retry(text));
    }

    fn clear_entries(&mut self) {
        self.entries.clear();
        self.tool = None;
        self.done.clear();
    }

    /// The top row: the session's position and title, then the local time.
    /// The name of the session in view: the stored title, or the cut first message
    /// for a session that has none yet, or `imp` before any message.
    fn session_name(&self) -> String {
        if !self.title.is_empty() {
            self.title.clone()
        } else if self.entries.iter().any(|e| e.role == Role::User) {
            title_of(&self.entries)
        } else {
            "imp".into()
        }
    }

    fn title_bar(&mut self, cols: usize) -> String {
        let name = self.session_name();
        let name = if self.panes.len() > 1 { format!("{}/{} {name}", self.at + 1, self.panes.len()) } else { name };
        let time = clock();
        let room = cols.saturating_sub(time.len() + 3);
        let text = format!(" {:<room$} {time} ", fit(&name, room));
        self.clock_shown = time;
        if self.cfg.title_gradient { self.palette.gradient(&text) } else { self.palette.select(&text) }
    }

    /// Whether the minute moved on since the title bar was drawn.
    fn clock_due(&mut self) -> bool {
        let due = !self.clock_shown.is_empty() && clock() != self.clock_shown;
        self.dirty |= due;
        due
    }

    /// A confirmation of what the user just did, shown in the status line.
    fn toast(&mut self, text: impl Into<String>) {
        self.status.toast = Some((text.into(), Instant::now()));
        self.dirty = true;
    }

    /// Drop a toast whose time is up; true when the screen needs a repaint.
    fn expire_toast(&mut self) -> bool {
        let due = self.status.toast.as_ref().is_some_and(|(_, at)| at.elapsed() >= TOAST);
        if due {
            self.status.toast = None;
            self.dirty = true;
        }
        due
    }

    /// Keep `text` for Ctrl+V and say how much was copied.
    fn copy(&mut self, text: String) {
        let n = text.chars().count();
        self.osc52 = Some(format!("\x1b]52;c;{}\x07", crate::api::base64(text.as_bytes())));
        self.clip = text;
        self.toast(format!("copied {n} characters"));
    }

    fn push(&mut self, role: Role, text: impl Into<String>) {
        self.entries.push(Entry {
            role,
            text: text.into(),
            marks: Vec::new(),
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
            Ev::Show(text) => {
                self.push(Role::Tool, format!("{RAW}{text}"));
                self.dirty = true;
            }
            Ev::GoalRound { text, round, rounds } => {
                if round > 1 {
                    self.push(Role::Info, format!("◎ goal round {round}/{rounds}"));
                }
                self.status.streaming = true;
                self.begin_task();
                self.status.goal = Some((text, round, rounds));
                self.status.goal_paused = false;
                self.dirty = true;
            }
            Ev::GoalEnd(why) => {
                self.push(Role::Info, format!("◎ {why}"));
                self.status.goal = None;
                self.status.goal_paused = false;
                self.dirty = true;
            }
            Ev::GoalPaused => {
                if let Some((text, round, rounds)) = &self.status.goal {
                    self.push(Role::Info, format!("◎ goal paused at round {round}/{rounds}: {text}"));
                }
                self.status.goal_paused = true;
                self.status.streaming = false;
                self.dirty = true;
            }
            Ev::Notice(text) => self.apply(Ev::Info(text)),
            Ev::Title(text) => {
                // Only the cut title is replaced; a rename stays.
                let first = self.entries.iter().find(|e| e.role == Role::User).map(|e| e.text.clone()).unwrap_or_default();
                if self.title.is_empty() || self.title == crate::store::cut_title(&first) {
                    self.title = text;
                    self.dirty = true;
                }
            }
            Ev::Suggest(text) => {
                // Late: the user has typed, or another turn began.
                if self.input.is_empty() && !self.status.streaming && self.pending.is_none() {
                    self.status.suggestion = Some(text);
                    self.dirty = true;
                }
            }
            Ev::Woke(what) => {
                self.status.suggestion = None;
                self.push(Role::Info, format!("⏰ {what}"));
                self.begin_task();
                self.status.streaming = true;
                self.dirty = true;
            }
            Ev::Token(t) => {
                self.status.chars += t.len();
                self.status.quiet_since = None;
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
                self.done.extend(self.tool.take());
                self.tool = Some(ToolRow {
                    index: self.entries.len(),
                    summary: summary.clone(),
                    args,
                    preview: String::new(),
                    output: String::new(),
                    detail: String::new(),
                    detail_start: None,
                    detail_count: 0,
                    finish: Finish::Done,
                    ms: 0,
                    running: true,
                    expanded: false,
                    started: Instant::now(),
                });
                self.status.steps += 1;
                self.status.quiet_since = None;
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
                self.status.quiet_since = Some(Instant::now());
                let finish = Finish::of(ok, &output);
                let role = finish.role();
                match self.tool.take() {
                    Some(mut row) => {
                        row.finish = finish;
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
                            self.push_diffs(preview, &args_ext(&row.args));
                        }
                        let text = tool_row_text(&row, self.verbosity);
                        if let Some(entry) = self.entries.get_mut(row.index) {
                            entry.role = role;
                            entry.text = text;
                        }
                        self.tool = Some(row);
                    }
                    None => self.push(
                        role,
                        format!("{} {preview}{TIME}{}", finish.mark(), duration(ms)),
                    ),
                }
            }
            Ev::Info(t) => self.push(Role::Info, t),
            Ev::Allow { shape, always } => {
                if !self.allows.iter().any(|(s, _)| s == &shape) {
                    self.allows.push((shape.clone(), always));
                }
                self.recent.retain(|(s, _)| s != &shape);
                self.dirty = true;
            }
            Ev::ShapeAsked { shape, allowed } => {
                self.recent.retain(|(s, _)| s != &shape);
                self.recent.push((shape, allowed));
                if self.recent.len() > 10 {
                    self.recent.remove(0);
                }
                self.dirty = true;
            }
            Ev::Diff(files, git) => self.show_diff(files, git),
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
                self.clear_entries();
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
                if let Some(tokens) = tokens {
                    self.status.tokens = tokens;
                }
                self.status.task_usage.0 += hit;
                self.status.task_usage.1 += miss;
                self.status.task_usage.2 += output;
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
                self.tick_spinner();
                self.since = Instant::now();
                self.bell_pending = true;
                self.dirty = true;
            }
            Ev::Question { q, opts, descs, multiple, reply } => {
                self.ask_marked = vec![false; opts.len()];
                self.ask_descs = descs;
                self.ask_multiple = multiple;
                self.pending = Some(Pending::Question {
                    q,
                    opts,
                    reply,
                    mode: AskMode::Choose,
                    typed: String::new(),
                });
                self.since = Instant::now();
                self.bell_pending = true;
                self.dirty = true;
                self.tick_spinner();
            }
            Ev::Config(new) => {
                self.cfg.reload_from(&new);
            }
            Ev::Retry { secs, attempt, of } => {
                self.status.retry = (secs > 0).then_some((secs, attempt, of));
                self.dirty = true;
            }
            Ev::Done(err) => {
                self.failed = err.is_some();
                if let Some(next) = self.agent_next.take() {
                    self.agent = next;
                }
                self.status.streaming = false;
                self.status.quiet_since = None;
                if let Some(start) = self.status.task_start.take() {
                    if self.verbosity != Verbosity::Low {
                        let (hit, miss, out) = self.status.task_usage;
                        let cost = self.task_cost(hit, miss, out);
                        self.push(Role::Thinking, task_line(start.elapsed(), self.status.steps, hit, miss, out, cost));
                    }
                }
                self.status.retry = None;
                if let Some(e) = err {
                    self.push(Role::Error, format!("{e} · Ctrl+R retry"));
                }
                self.dirty = true;
                // The next queued message starts, unless one is being edited or
                // the user stopped the turn: then the queue waits.
                let stopped = self.cancel.load(Ordering::Relaxed);
                // Steered too late for the turn: it starts the next one, ahead
                // of the queue. Its row is already in the transcript.
                let late: Vec<String> = self.steer.lock().map(|mut b| std::mem::take(&mut *b)).unwrap_or_default();
                if !late.is_empty() && stopped {
                    // Stopped: kept at the front of the queue, not lost.
                    self.queued.splice(0..0, late);
                } else if !late.is_empty() {
                    if let Some(tx) = self.worker.as_ref().map(|w| w.0.clone()) {
                        self.status.streaming = true;
                        self.begin_task();
                        let _ = tx.send(Cmd::Prompt(late.join("\n\n")));
                        return;
                    }
                }
                // Ended while a queued message is open: it goes on when closed.
                self.queue_due = self.queue_at.is_some() && !stopped;
                if self.queue_at.is_none() && !stopped && !self.queued.is_empty() {
                    let text = self.queued.remove(0);
                    if let Some(tx) = self.worker.as_ref().map(|w| w.0.clone()) {
                        self.start_turn(text, &tx);
                    }
                }
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
            // Stopping is not answering: the view stays on this session.
            let interrupt = matches!(key, Key::Ctrl('c') | Key::Esc);
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
        // A click replays what it lands on; anywhere else it does nothing.
        // Any key takes the mouse back, then does what it always does.
        if std::mem::take(&mut self.released) {
            self.mouse_seq = Some(MOUSE_ON);
            self.dirty = true;
        }
        if matches!(key, Key::Ctrl('g')) {
            if self.cfg.mouse {
                self.released = true;
                self.mouse_seq = Some(MOUSE_OFF);
                self.dirty = true;
            }
            return;
        }
        match key {
            Key::Drag(x, y) => {
                if let Some(d) = &mut self.drag {
                    (d.2, d.3) = (x, y);
                    self.dirty = true;
                }
                return;
            }
            Key::Release(x, y) => {
                if let Some((ax, ay, _, _)) = self.drag.take() {
                    self.dirty = true;
                    let text = selected_text(&self.shown, (ax, ay), (x, y));
                    if (ax, ay) != (x, y) && !text.is_empty() {
                        self.copy(text);
                    }
                }
                return;
            }
            Key::Click(x, y) => self.drag = Some((x, y, x, y)),
            _ => {}
        }
        let armed = std::mem::take(&mut self.armed);
        if armed {
            self.dirty = true;
        }
        if let Key::Click(x, y) = key {
            let (x, y) = (x as usize, y as usize);
            let hit = self.targets.iter().find(|t| t.0 == y && (t.1..=t.2).contains(&x));
            match hit.map(|t| t.3.clone()) {
                // A stray click must not allow a command for good.
                Some(Target::Key(Key::Char('a')))
                    if !armed && matches!(self.pending, Some(Pending::Confirm { .. })) =>
                {
                    self.armed = true;
                    self.dirty = true;
                }
                Some(Target::Key(k)) => self.handle_key_inner(k, tx),
                Some(Target::Copy(i, block)) => {
                    let text = &self.entries[i].text;
                    let text = match block {
                        Some(k) => crate::render::fences(text).into_iter().nth(k),
                        None => Some(text.clone()),
                    };
                    if let Some(text) = text {
                        self.copy(text);
                    }
                }
                Some(Target::Tool(i)) => self.toggle_row(i),
                Some(Target::Row(i)) => {
                    if let Some(Pending::Pick(picker)) = &mut self.pending {
                        picker.index = i;
                        self.handle_key_inner(Key::Enter, tx);
                    }
                }
                None => {}
            }
            return;
        }
        // Unknown sequences do nothing, and must not answer a prompt.
        if matches!(key, Key::Unknown) {
            return;
        }
        // `?` on an empty composer shows the keys; `?` again closes them.
        if matches!(key, Key::Char('?')) {
            match &self.pending {
                None if self.input.is_empty() => {
                    self.open_keys();
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
            // Stop before quitting: an open prompt is declined as Esc would,
            // a draft is cleared, a running turn is stopped. Only an idle,
            // empty Ctrl+C asks, and a second within two seconds quits.
            self.dirty = true;
            if self.queue_at.is_some() {
                // The editor closes; the message stays queued as it was.
                self.close_queued(tx);
                return;
            }
            if let Some(pending) = self.pending.take() {
                // A picker typed from the draft goes with its text.
                let picker = matches!(pending, Pending::Pick(_));
                self.answer(pending, Key::Esc, tx);
                if picker {
                    self.pending = None;
                    self.input.clear();
                    self.cursor = 0;
                }
            } else if self.status.streaming {
                // A running turn stops, and the draft stays for the next message.
                self.cancel.store(true, Ordering::Relaxed);
            } else if !self.input.is_empty() {
                // Idle with a draft: nothing to stop and no quit. Ctrl+L clears
                // the transcript; the draft is left as it is.
            } else if self.quit_armed.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
                self.quit = true;
            } else {
                self.quit_armed = Some(Instant::now());
                self.push(Role::Info, "Ctrl+C again to quit");
            }
            return;
        }
        if let Key::Paste(text) = &key {
            return self.paste(text.clone(), tx);
        }
        if let Some(s) = self.status.suggestion.take() {
            // Right or Enter takes it into the draft; Enter never sends it.
            if matches!(key, Key::Right | Key::Enter)
                && self.input.is_empty()
                && self.pending.is_none()
            {
                self.cursor = s.len();
                self.input = s;
                self.dirty = true;
                return;
            }
            self.dirty = true;
        }
        if let Some(pending) = self.pending.take() {
            self.answer(pending, key, tx);
            return;
        }
        // A clipboard image comes first; it leaves the draft and selection alone.
        if matches!(key, Key::Ctrl('v')) && self.paste_image(tx, &crate::images::dir()) {
            return;
        }
        // An edit consumes the selection; anything but a Shift move clears it.
        if !matches!(key, Key::Select(_) | Key::SelectWord(_)) {
            if let Some((from, to)) = self.selection() {
                let paste = matches!(key, Key::Ctrl('v')) && !self.clip.is_empty();
                let removes = matches!(key, Key::Backspace | Key::Delete | Key::DeleteWord(_) | Key::Ctrl('w'));
                if removes || paste || matches!(key, Key::Char(_) | Key::Newline) {
                    self.follow_tail();
                    self.input.replace_range(from..to, "");
                    self.cursor = from;
                    self.anchor = None;
                    if removes {
                        self.sync_menu();
                        self.dirty = true;
                        return;
                    }
                }
            }
            self.anchor = None;
        }

        match key {
            Key::Select(m) => {
                self.anchor.get_or_insert(self.cursor);
                self.move_caret(m);
            }
            Key::SelectWord(m) => {
                self.anchor.get_or_insert(self.cursor);
                self.cursor = word_edge(&self.input, self.cursor, m == Move::Right);
                self.dirty = true;
            }
            Key::Word(_) | Key::Ctrl('a' | 'u' | 'k' | 'w') | Key::DeleteWord(_) | Key::Delete => {
                let before = self.input.len();
                edit_line(&mut self.input, &mut self.cursor, &key, "");
                if self.input.len() != before {
                    self.follow_tail();
                    self.sync_menu();
                }
                self.dirty = true;
            }
            Key::Ctrl('s') => {
                // In the queue editor, Ctrl+S sends the open message. Elsewhere
                // it steers during a turn and sends when idle; an empty draft
                // does nothing.
                if self.queue_at.is_some() {
                    self.send_queued(tx);
                } else {
                    self.submit(tx, true);
                }
                self.dirty = true;
            }
            Key::Ctrl('c') => self.quit = true,
            Key::Ctrl('d') if self.input.is_empty() => self.quit = true,
            Key::Ctrl('p') => {
                self.pending = Some(Pending::Pick(Picker::new(
                    "palette",
                    self.palette_items(),
                    PickKind::Palette,
                )));
                self.dirty = true;
            }
            Key::Ctrl('n') if self.panes.len() > 1 => {
                let n = self.panes.len();
                self.show((self.at + 1) % n);
            }
            Key::Ctrl('t') => self.run_command("think", tx),
            Key::Ctrl('r') => self.retry(tx),
            Key::Ctrl('e') => {
                self.toggle_tool();
                self.dirty = true;
            }
            Key::Ctrl('v') if !self.clip.is_empty() => {
                self.follow_tail();
                let clip = self.clip.clone();
                self.input.insert_str(self.cursor, &clip);
                self.cursor += clip.len();
                self.sync_menu();
            }
            Key::Ctrl('l') => {
                self.clear_entries();
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
                } else if self.input.is_empty() {
                    // Nothing left to delete: the last attached image goes.
                    let _ = tx.send(Cmd::Detach);
                }
            }
            Key::Left => self.move_caret(Move::Left),
            Key::Right => self.move_caret(Move::Right),
            Key::Home => self.move_caret(Move::Home),
            Key::End => self.move_caret(Move::End),
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
                let rows = self.draft_rows_now();
                let (row, col) = self.drawn_caret(&rows);
                if row > 0 {
                    self.cursor = self.drawn_at(&rows, row - 1, col, true);
                    self.dirty = true;
                } else if let Some(i) = self.queue_at {
                    // Older queued messages, then history.
                    let kept = self.save_queued(tx, false);
                    match kept.unwrap_or(i).checked_sub(1).filter(|&o| o < self.queued.len()) {
                        Some(o) => self.open_queued(o),
                        None => {
                            if !self.history.is_empty() {
                                let last = self.history.len() - 1;
                                self.hist_pos = Some(last);
                                self.input = self.history[last].clone();
                                self.cursor = self.input.len();
                            }
                            // Left the queue: a turn that ended meanwhile goes on.
                            self.release_queue(tx);
                        }
                    }
                } else if self.input.is_empty() && self.hist_pos.is_none() && !self.queued.is_empty() {
                    self.open_queued(self.queued.len() - 1);
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
                let rows = self.draft_rows_now();
                let (row, col) = self.drawn_caret(&rows);
                if row + 1 < rows.len() {
                    self.cursor = self.drawn_at(&rows, row + 1, col, false);
                    self.dirty = true;
                } else if let Some(i) = self.queue_at {
                    // Newer queued messages, then an empty draft.
                    let next = match self.save_queued(tx, false) {
                        Some(i) => i + 1,
                        None => i,
                    };
                    if next < self.queued.len() {
                        self.open_queued(next);
                    } else {
                        self.release_queue(tx);
                    }
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
            // Tab completes. Shift+Tab changes the agent, with or without a draft.
            Key::Tab => self.sync_menu(),
            Key::BackTab => self.cycle_agent(tx),
            Key::Ctrl('b') => {
                // Ctrl+B steps through the modes; it applies at once.
                use crate::config::Permissions::*;
                let next = match self.cfg.live_permissions() {
                    Normal => "auto",
                    Auto => "yolo",
                    Yolo => "strict",
                    Strict => "normal",
                };
                self.set_mode(next, tx);
                self.dirty = true;
            }
            Key::Esc => {
                if self.queue_at.is_some() {
                    // Leaving the editor does not swallow the stop.
                    self.close_queued(tx);
                }
                if self.status.streaming {
                    self.cancel.store(true, Ordering::Relaxed);
                }
                self.dirty = true;
            }
            Key::Ctrl('x') if self.queue_at.is_some() => self.delete_queued(tx),
            Key::Enter => self.submit(tx, false),
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
    /// Expand every tool row of the latest turn, or collapse them all when
    /// none is collapsed.
    fn toggle_tool(&mut self) {
        let expand = self.done.iter().chain(&self.tool).any(|r| !r.expanded);
        for row in &mut self.done {
            row.expanded = expand;
            if let Some(entry) = self.entries.get_mut(row.index) {
                entry.text = tool_row_text(row, self.verbosity);
            }
        }
        self.expand_newest(expand);
    }

    /// Toggle the one kept tool row at entry `index`, as a click does.
    fn toggle_row(&mut self, index: usize) {
        if let Some(row) = self.done.iter_mut().find(|r| r.index == index) {
            row.expanded = !row.expanded;
            if let Some(entry) = self.entries.get_mut(index) {
                entry.text = tool_row_text(row, self.verbosity);
            }
        } else if let Some(row) = self.tool.as_ref().filter(|r| r.index == index) {
            let expand = !row.expanded;
            self.expand_newest(expand);
        }
    }

    /// The newest row, whose diff can still grow, as it is last on screen.
    fn expand_newest(&mut self, expand: bool) {
        let Some(mut row) = self.tool.take() else {
            return;
        };
        row.expanded = expand;
        let text = tool_row_text(&row, self.verbosity);
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
                self.push_diffs(lines, &args_ext(&row.args));
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
        let active = self.cfg.active_name.clone();
        // The active provider's own models: the current one, its default, and
        // the list it reported. Every row names the provider. The recents file
        // records names without a provider, so it is not listed here; the
        // provider-model picker still offers it.
        let mut names = vec![current.clone(), default.clone()];
        names.extend(self.available_models.iter().cloned());
        let mut items: Vec<PickItem> = Vec::new();
        for name in names {
            let value = format!("{active}::{name}");
            if name.is_empty() || items.iter().any(|i| i.value == value) {
                continue;
            }
            let desc = if name == default { format!("{active}, default") } else { active.clone() };
            items.push(PickItem {
                label: name.clone(),
                desc: mark_current(name == current, &desc),
                value,
                actions: Vec::new(),
            });
        }
        // Every provider's models, each row naming its provider. Choosing one
        // switches to that provider as well.
        let mut providers: Vec<(String, Vec<String>)> = Vec::new();
        // A built-in provider is listed only when it is configured (its key is
        // set, or it needs none) and not switched off. The active one always
        // stays, so the session's own model is never hidden.
        for provider in [
            crate::providers::Provider::DeepSeek,
            crate::providers::Provider::OpenAi,
            crate::providers::Provider::Anthropic,
            crate::providers::Provider::Gemini,
            crate::providers::Provider::Ollama,
            crate::providers::Provider::Custom,
        ] {
            let name = format!("{provider:?}").to_lowercase();
            let usable = name == active
                || (self.cfg.provider_configured(provider, crate::config::key_lookup()) && !self.cfg.provider_disabled(&name));
            if usable {
                providers.push((name, vec![provider.profile().model.to_string()]));
            }
        }
        for named in &self.cfg.named {
            if named.name != active && self.cfg.provider_disabled(&named.name) {
                continue;
            }
            providers.push((named.name.clone(), named.offered_models()));
        }
        for (provider, models) in providers {
            for model in models {
                let value = format!("{provider}::{model}");
                if items.iter().any(|i| i.value == value) {
                    continue;
                }
                let current_pair = provider == self.cfg.active_name && model == current;
                items.push(PickItem {
                    label: model.clone(),
                    desc: mark_current(current_pair, &provider),
                    value,
                    actions: Vec::new(),
                });
            }
        }
        items
    }

    /// Every skill on disk and every switched-off one, each marked on or off.
    fn skill_items(&self) -> Vec<PickItem> {
        let mut found: Vec<(String, String)> = crate::store::load_skills(&self.cfg, &self.root)
            .into_iter()
            .map(|s| (s.name, s.description))
            .collect();
        for name in &self.cfg.disabled_skills {
            if !found.iter().any(|(n, _)| n == name) {
                found.push((name.clone(), "switched off".to_string()));
            }
        }
        found.sort();
        found
            .into_iter()
            .map(|(name, about)| {
                let state = if self.cfg.skill_disabled(&name) { "off" } else { "on" };
                PickItem {
                    label: name.clone(),
                    desc: format!("{state} · {about}"),
                    value: name,
                    actions: Vec::new(),
                }
            })
            .collect()
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

    /// Commands and key actions in one list, each with its shortcut.
    fn palette_items(&self) -> Vec<PickItem> {
        let mut items: Vec<PickItem> = COMMANDS
            .iter()
            .map(|(name, desc)| PickItem {
                label: name.to_string(),
                value: format!("cmd:{}", name.trim_start_matches('/')),
                desc: match COMMAND_KEYS.iter().find(|(c, _)| c == name) {
                    Some((_, key)) => format!("{desc} · {key}"),
                    None => desc.to_string(),
                },
                actions: Vec::new(),
            })
            .collect();
        for (label, key, action) in PALETTE_ACTIONS {
            items.push(PickItem {
                label: label.to_string(),
                value: format!("key:{action}"),
                desc: key.to_string(),
                actions: Vec::new(),
            });
        }
        items
    }

    /// Run the key an action row names, as if it were pressed.
    fn palette_action(&mut self, action: &str, tx: &Sender<Cmd>) {
        match action {
            "next-session" => {
                let n = self.panes.len();
                if n > 1 {
                    self.show((self.at + 1) % n);
                } else {
                    self.push(Role::Info, "one session");
                }
            }
            "steer" => self.handle_key_inner(Key::Ctrl('s'), tx),
            "paste" => self.handle_key_inner(Key::Ctrl('v'), tx),
            "stop" => self.handle_key_inner(Key::Esc, tx),
            "expand" => self.handle_key_inner(Key::Ctrl('e'), tx),
            "clear" => self.handle_key_inner(Key::Ctrl('l'), tx),
            "page-up" => self.handle_key_inner(Key::PageUp, tx),
            "page-down" => self.handle_key_inner(Key::PageDown, tx),
            "release" => self.handle_key_inner(Key::Ctrl('g'), tx),
            "quit" => self.handle_key_inner(Key::Ctrl('d'), tx),
            _ => {}
        }
        self.dirty = true;
    }

    fn mode_items(&self) -> Vec<PickItem> {
        let current = self.cfg.live_permissions().as_str();
        [
            ("normal", "unlisted commands ask"),
            ("auto", "builds, tests and local git run"),
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
        .chain(self.allows.iter().map(|(shape, always)| PickItem {
            label: shape.clone(),
            value: format!("allow:{shape}"),
            desc: if *always { "always" } else { "session" }.into(),
            actions: vec![('d', "revoke")],
        }))
        .chain(std::iter::once(PickItem {
            label: "add a rule".into(),
            value: "add-rule".into(),
            desc: "a".into(),
            actions: vec![('a', "add")],
        }))
        .chain(
            self.recent
                .iter()
                .rev()
                .filter(|(s, _)| !self.allows.iter().any(|(a, _)| a == s))
                .take(5)
                .map(|(shape, allowed)| PickItem {
                    label: shape.clone(),
                    value: format!("asked:{shape}"),
                    desc: if *allowed { "asked · allowed" } else { "asked · declined" }.into(),
                    actions: vec![('a', "allow")],
                }),
        )
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
        Verbosity::ALL
            .iter()
            .map(|(level, about)| PickItem {
                label: level.as_str().into(),
                value: level.as_str().into(),
                desc: mark_current(*level == self.verbosity, about),
                actions: Vec::new(),
            })
            .collect()
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
            let mut about = format!("{}  {}", profile.base_url, profile.model);
            // Switched off wins over unconfigured: an off provider is not offered at all.
            if self.cfg.provider_disabled(&name) {
                about.push_str(" · off");
            } else if !self.cfg.provider_configured(provider, crate::config::key_lookup()) {
                about.push_str(" · no key");
            }
            items.push(PickItem {
                desc: mark_current(name == self.cfg.active_name, &about),
                label: name.clone(),
                value: name.clone(),
                actions: vec![('u', "use"), ('e', "configure")],
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
                actions: vec![('u', "use"), ('e', "configure")],
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
        let mut options: Vec<PickItem> = crate::store::Log::listing(&self.sessions_dir)
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
        mark_rows(&mut options, &self.session_marks());
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
        mark_rows(&mut items, &self.session_marks());
        Picker::new(format!("search \"{}\"", query.trim()), items, PickKind::Session)
    }

    /// The value a picker of this kind should open on.
    fn current_value(&self, kind: PickKind) -> String {
        match kind {
            PickKind::Agent => self.agent.clone(),
            PickKind::Model => self.cfg.model.clone(),
            PickKind::Theme => self.palette.theme_name().to_string(),
            PickKind::Verbosity => self.verbosity.as_str().into(),
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

    /// Send the draft. During a turn, Enter (`steer` false) queues it and
    /// Ctrl+S (`steer` true) hands it to the running turn.
    fn submit(&mut self, tx: &Sender<Cmd>, steer: bool) {
        // Enter on an open queued message saves it; empty, it is removed.
        if self.queue_at.is_some() {
            self.save_queued(tx, true);
            return;
        }
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
        // During a turn, Enter holds the message for the turn's end, and Ctrl+S
        // steers it into the turn. A goal ends on any message, as before.
        if self.status.streaming && self.status.goal.is_none() {
            if steer {
                // The running turn takes it before its next step.
                self.push(Role::User, format!("{text} (steered)"));
                if let Ok(mut box_) = self.steer.lock() {
                    box_.push(text);
                }
            } else {
                self.queued.push(text);
            }
            self.dirty = true;
            return;
        }
        self.start_turn(text, tx);
    }

    /// `/diff`: a summary row per file, then each file's diff in its
    /// syntax, at most 200 rows in all, then what is not tracked.
    fn show_diff(&mut self, files: Vec<crate::agent::FileDiff>, git: bool) {
        if files.is_empty() {
            let none = if git { "git diff shows no changes" } else { "imp has changed no file this session; /diff git shows the repository" };
            self.push(Role::Info, none);
            return;
        }
        let wide = files.iter().map(|f| f.name.chars().count()).max().unwrap_or(0);
        let summary: Vec<String> = files
            .iter()
            .map(|f| match f.untracked {
                true => format!("{:wide$}  not diffed: over 1 MB or not text", f.name),
                false => format!("{:wide$}  +{} −{}", f.name, f.adds, f.dels),
            })
            .collect();
        self.push(Role::Info, summary.join("\n"));
        let mut room = 200usize;
        let mut cut = 0;
        for f in files.iter().filter(|f| !f.untracked) {
            if room == 0 {
                cut += f.rows.len();
                continue;
            }
            self.push(Role::Context, format!("── {}", f.name));
            let take = f.rows.len().min(room);
            cut += f.rows.len() - take;
            room -= take;
            self.push_diffs(f.rows[..take].to_vec(), &f.ext);
        }
        if cut > 0 {
            self.push(Role::Context, format!("… {cut} more lines"));
        }
        if !git {
            self.push(Role::Context, "changes made by shell commands are not tracked; /diff git shows everything");
        }
        self.follow_tail();
    }

    /// The `/settings` form: ten settings, each row showing its value.
    fn open_settings(&mut self) {
        self.open_settings_at(None);
    }

    /// The settings modal, with the row for `focus` highlighted.
    fn open_settings_at(&mut self, focus: Option<&str>) {
        let items = SETTING_ROWS
            .iter()
            .map(|(key, label)| PickItem {
                label: label.to_string(),
                value: key.to_string(),
                desc: self.setting_value(key),
                actions: Vec::new(),
            })
            .collect();
        let mut picker = Picker::new("settings", items, PickKind::Settings);
        if let Some(key) = focus {
            picker.index = picker.items.iter().position(|i| i.value == key).unwrap_or(0);
        }
        self.pending = Some(Pending::Pick(picker));
        self.dirty = true;
    }

    /// The current value of a settings row, as the form shows it.
    fn setting_value(&self, key: &str) -> String {
        let onoff = |on: bool| if on { "on" } else { "off" }.to_string();
        match key {
            "permissions" => self.cfg.live_permissions().as_str().into(),
            "thinking" => onoff(self.cfg.thinking),
            "effort" => self.cfg.effort.clone().unwrap_or_else(|| "default".into()),
            "verbosity" => self.verbosity.as_str().into(),
            "mouse" => onoff(self.cfg.mouse),
            "suggest" => onoff(self.cfg.suggest),
            "diffs" => onoff(self.cfg.diffs),
            "auto_next" => onoff(self.cfg.auto_next),
            "title_gradient" => onoff(self.cfg.title_gradient),
            "spinner" => crate::config::SPINNERS.get(self.cfg.spinner).map_or(String::new(), |s| s.0.into()),
            "allow_sudo" => onoff(self.cfg.allow_sudo),
            "color" => self.cfg.color.as_str().into(),
            "links" => self.cfg.links.as_str().into(),
            "title_summary" => self.cfg.title_summary.as_str().into(),
            "model" => self.cfg.model.clone(),
            "base_url" => self.cfg.base_url.clone(),
            "context_window" => self.cfg.context_window.map_or("unset".into(), |n| n.to_string()),
            "max_tokens" => self.cfg.max_tokens.to_string(),
            "max_steps" => self.cfg.max_steps.to_string(),
            "goal_rounds" => self.cfg.goal_rounds.to_string(),
            "compact_at" => self.cfg.compact_at.to_string(),
            "prune_at" => self.cfg.prune_at.to_string(),
            "skills_dir" => self.cfg.skills_dir.as_ref().map_or("unset".into(), |p| p.display().to_string()),
            "api_key" => if self.cfg.api_key.is_empty() { "not set" } else { "set" }.into(),
            _ => String::new(),
        }
    }

    /// The values a settings row steps through, in order.
    fn setting_choices(key: &str) -> Vec<String> {
        let words = |list: &[&str]| list.iter().map(|s| s.to_string()).collect();
        match key {
            "permissions" => words(&["normal", "auto", "strict", "yolo"]),
            "effort" => words(&["default", "low", "medium", "high"]),
            "verbosity" => words(&["low", "medium", "high"]),
            "thinking" | "mouse" | "suggest" | "diffs" | "auto_next" | "title_gradient" | "allow_sudo" => words(&["off", "on"]),
            "color" => words(&["auto", "always", "never"]),
            "links" => words(&["auto", "always", "never"]),
            "title_summary" => words(&["auto", "on", "off"]),
            "spinner" => crate::config::SPINNERS.iter().map(|s| s.0.to_string()).collect(),
            _ => Vec::new(),
        }
    }

    /// Step the highlighted row by `dir`, apply it to the session, save it
    /// when the config names a file, and refresh the row's value.
    fn step_setting(&mut self, picker: &mut Picker, dir: i64, tx: &Sender<Cmd>) {
        let Some(key) = picker.items.get(picker.index).map(|item| item.value.clone()) else {
            return;
        };
        let choices = Self::setting_choices(&key);
        if choices.is_empty() {
            return;
        }
        let current = self.setting_value(&key);
        let at = choices.iter().position(|c| *c == current).unwrap_or(0) as i64;
        let next = choices[(at + dir).rem_euclid(choices.len() as i64) as usize].clone();
        if let Err(e) = self.cfg.set(&key, &next) {
            self.toast(e);
            return;
        }
        if key == "verbosity" {
            self.verbosity = Verbosity::parse(&next).unwrap_or(self.verbosity);
        }
        if key == "permissions" {
            if let Ok(mode) = crate::config::Permissions::parse(&next) {
                let _ = tx.send(Cmd::SetPermissions(mode));
            }
        } else {
            let _ = tx.send(Cmd::SetConfig(key.clone(), next.clone()));
        }
        if let Some(file) = self.cfg.file.clone() {
            if let Err(e) = crate::config::save_setting(&file, &key, &next) {
                self.toast(e);
            }
        }
        if let Some(item) = picker.items.get_mut(picker.index) {
            item.desc = self.setting_value(&key);
        }
        self.dirty = true;
    }

    /// The keys card, for `?` and `/keys`.
    fn open_keys(&mut self) {
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
    }

    /// Tab: the next agent profile. The worker applies it when free, so a
    /// running turn finishes with the agent it started with.
    fn cycle_agent(&mut self, tx: &Sender<Cmd>) {
        if self.agents.len() < 2 {
            return;
        }
        let current = self.agent_next.as_deref().unwrap_or(&self.agent);
        let at = self.agents.iter().position(|(n, _)| n == current).unwrap_or(0);
        let next = self.agents[(at + 1) % self.agents.len()].0.clone();
        let _ = tx.send(Cmd::SetAgent(next.clone()));
        // The prompt names the agent; no transcript line.
        if self.status.streaming {
            self.agent_next = Some(next);
        } else {
            self.agent = next;
        }
        self.dirty = true;
    }

    /// Open queued message `i` in the composer and hold the running turn.
    fn open_queued(&mut self, i: usize) {
        if self.queued.is_empty() {
            return;
        }
        let i = i.min(self.queued.len() - 1);
        self.queue_at = Some(i);
        self.input = self.queued[i].clone();
        self.cursor = self.input.len();
        self.hold.store(true, Ordering::SeqCst);
        self.dirty = true;
    }

    /// Put the open queued message back, or remove it when emptied, and
    /// release the turn. Returns its index if it was kept. `closing` leaves
    /// the queue rather than moving within it.
    fn save_queued(&mut self, tx: &Sender<Cmd>, closing: bool) -> Option<usize> {
        let i = self.queue_at.take()?;
        self.hold.store(false, Ordering::SeqCst);
        let text = std::mem::take(&mut self.input);
        self.cursor = 0;
        self.dirty = true;
        if i >= self.queued.len() {
            // The queue shrank under the editor: there is nothing to save.
            self.release_queue(tx);
            return None;
        }
        let kept = if text.trim().is_empty() {
            self.queued.remove(i);
            self.push(Role::Info, format!("queue: deleted {}", i + 1));
            None
        } else {
            self.queued[i] = text;
            Some(i)
        };
        if closing {
            self.release_queue(tx);
        }
        kept
    }

    /// Close the editor without writing, and let the queue go on.
    fn close_queued(&mut self, tx: &Sender<Cmd>) {
        self.queue_at = None;
        self.hold.store(false, Ordering::SeqCst);
        self.dirty = true;
        self.release_queue(tx);
    }

    /// Send the open message now: a steer into the running turn, or a turn of
    /// its own when none runs. It leaves the queue either way.
    fn send_queued(&mut self, tx: &Sender<Cmd>) {
        let Some(i) = self.queue_at else { return };
        let text = self.input.trim().to_string();
        if text.is_empty() {
            self.delete_queued(tx);
            return;
        }
        if i < self.queued.len() {
            self.queued.remove(i);
        }
        self.input.clear();
        self.cursor = 0;
        self.queue_at = None;
        self.hold.store(false, Ordering::SeqCst);
        self.history.push(text.clone());
        save_history(&self.history);
        if self.history.len() > 1000 {
            self.history.drain(..self.history.len() - 1000);
        }
        if self.status.streaming && self.status.goal.is_none() {
            self.push(Role::User, format!("{text} (steered)"));
            if let Ok(mut box_) = self.steer.lock() {
                box_.push(text);
            }
        } else {
            self.start_turn(text, tx);
        }
        self.dirty = true;
        self.release_queue(tx);
    }

    /// Delete the open message and open the one that follows, or close.
    fn delete_queued(&mut self, tx: &Sender<Cmd>) {
        let Some(i) = self.queue_at else { return };
        let gone = i < self.queued.len();
        if gone {
            self.queued.remove(i);
            self.push(Role::Info, format!("queue: deleted {}", i + 1));
        }
        self.hold.store(false, Ordering::SeqCst);
        self.input.clear();
        self.cursor = 0;
        self.dirty = true;
        if self.queued.is_empty() {
            self.queue_at = None;
            self.release_queue(tx);
            return;
        }
        self.open_queued(i.min(self.queued.len() - 1));
    }

    /// A turn that ended while a queued message was open hands on now.
    fn release_queue(&mut self, tx: &Sender<Cmd>) {
        if std::mem::take(&mut self.queue_due) && !self.status.streaming && !self.queued.is_empty() {
            let next = self.queued.remove(0);
            self.start_turn(next, tx);
        }
    }

    /// Show a user message and send it as a turn. A turn already running
    /// keeps its meter; the worker takes the message when it is free.
    fn start_turn(&mut self, text: String, tx: &Sender<Cmd>) {
        self.push(Role::User, text.clone());
        if !self.status.streaming {
            self.status.streaming = true;
            self.begin_task();
        }
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
        self.push(role, format!("$ {command}"));
        // Your own command's output keeps its colours and is not capped.
        let shown = out.full.map(|f| crate::secrets::redact(&f)).unwrap_or(out.text);
        if !shown.trim().is_empty() {
            self.push(role, format!("{RAW}{}", crate::agent::first_lines(&shown, crate::agent::SHOW_LINES)));
        }
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
        self.toast(format!("model: {name}"));
    }

    /// Add, list, pop, or delete parked messages.
    fn queue_command(&mut self, rest: &str) {
        if rest.is_empty() {
            if self.queued.is_empty() {
                self.push(Role::Info, "queue is empty");
            } else {
                let lines: Vec<String> = self
                    .queued
                    .iter()
                    .enumerate()
                    .map(|(i, text)| format!("{}. {text}", i + 1))
                    .collect();
                self.push(Role::Info, lines.join("\n"));
            }
            return;
        }
        if rest == "clear" {
            self.queued.clear();
            self.queue_at = None;
            self.hold.store(false, Ordering::SeqCst);
            self.push(Role::Info, "queue: cleared");
            return;
        }
        if rest == "pop all" {
            if self.queued.is_empty() {
                self.push(Role::Info, "queue is empty");
                return;
            }
            let all = std::mem::take(&mut self.queued).join("\n\n");
            self.load_queued(&all);
            self.push(Role::Info, "queue: loaded all");
            return;
        }
        if let Some(n) = rest.strip_prefix("pop").map(str::trim) {
            let index = if n.is_empty() {
                1
            } else {
                match n.parse::<usize>() {
                    Ok(n) if n >= 1 => n,
                    _ => {
                        self.push(Role::Error, format!("queue: no entry {n:?}"));
                        return;
                    }
                }
            };
            if index > self.queued.len() {
                self.push(Role::Error, self.queue_range());
                return;
            }
            let text = self.queued.remove(index - 1);
            self.load_queued(&text);
            self.push(Role::Info, format!("queue: popped {index}"));
            return;
        }
        if let Some(n) = rest.strip_prefix("delete").map(str::trim) {
            let index = match n.parse::<usize>() {
                Ok(n) if n >= 1 => n,
                _ => {
                    self.push(Role::Error, format!("queue: no entry {n:?}"));
                    return;
                }
            };
            if index > self.queued.len() {
                self.push(Role::Error, self.queue_range());
                return;
            }
            self.queued.remove(index - 1);
            self.push(Role::Info, format!("queue: deleted {index}"));
            return;
        }
        self.queued.push(rest.to_string());
        self.push(Role::Info, format!("queue: added ({})", self.queued.len()));
    }

    fn queue_range(&self) -> String {
        if self.queued.is_empty() {
            "queue is empty".into()
        } else {
            format!("queue: pick 1 to {}", self.queued.len())
        }
    }

    /// Put queued text in the composer, after any draft already there.
    fn load_queued(&mut self, text: &str) {
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
                if crate::providers::Provider::parse(name).is_ok() {
                    self.push(Role::Error, format!("provider: {name} is a built-in provider name"));
                    return;
                }
                if !crate::config::valid_profile_name(name) {
                    self.push(Role::Error, "provider: name must be letters, digits, - or _");
                    return;
                }
                if self.cfg.named.iter().any(|n| n.name == name) {
                    self.push(Role::Error, format!("provider {name} already exists"));
                    return;
                }
                let fields = ["base".to_string(), "model".to_string(), "dialect".to_string(), "key_env".to_string(), "models".to_string(), "api_key".to_string()]
                    .into_iter()
                    .map(|label| (label, String::new()))
                    .collect();
                // The caret starts at the end of the field.
                self.form_caret = usize::MAX;
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
                    ("models".to_string(), profile.models.join(", ")),
                    ("api_key".to_string(), String::new()),
                ];
                // The caret starts at the end of the field.
                self.form_caret = usize::MAX;
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

    /// Tab on a provider form's model field: the dialect's default model, the
    /// recent models, then a row that types a name instead.
    fn open_model_picker(&mut self, name: String, fields: Vec<(String, String)>, at: usize) {
        let dialect = fields.iter().find(|f| f.0 == "dialect").map(|f| f.1.clone()).unwrap_or_default();
        let mut names: Vec<String> = Vec::new();
        if let Ok(provider) = crate::providers::Provider::parse(&dialect) {
            names.push(provider.profile().model.to_string());
        }
        names.extend(load_models(&self.models_path));
        let mut items: Vec<PickItem> = Vec::new();
        for model in names {
            if model.is_empty() || items.iter().any(|i| i.value == model) {
                continue;
            }
            items.push(PickItem { label: model.clone(), value: model, actions: Vec::new(), desc: String::new() });
        }
        items.push(PickItem {
            label: "type another".into(),
            value: TYPE_ANOTHER.into(),
            actions: Vec::new(),
            desc: "keep typing in the form".into(),
        });
        let mut picker = Picker::new("model", items, PickKind::ProviderModel);
        picker.form = Some(Box::new((name, fields, at)));
        self.pending = Some(Pending::Pick(picker));
    }

    /// Start a goal now: its first round runs as soon as the worker is free.
    fn start_goal_now(&mut self, text: String, tx: &Sender<Cmd>) {
        self.push(Role::User, format!("/goal {text}"));
        self.status.streaming = true;
        self.begin_task();
        let _ = tx.send(Cmd::Goal(text));
    }

    /// Switch the session to a provider and report it.
    fn use_provider(&mut self, name: &str, tx: &Sender<Cmd>) {
        match self.set_provider(name, tx) {
            Ok(()) => {
                self.pending = None;
                self.push(Role::Info, format!("provider: {name}"));
            }
            Err(e) => self.push(Role::Error, e),
        }
        self.dirty = true;
    }

    /// `e` on a provider row: the form for its configuration. A named profile
    /// opens as saved; a built-in opens from its defaults, to be saved as a
    /// named copy so the built-in stays as it is.
    fn configure_provider(&mut self, value: &str) {
        if let Some(profile) = self.cfg.named.iter().find(|n| n.name == value).cloned() {
            let fields = vec![
                ("base".to_string(), profile.base),
                ("model".to_string(), profile.model),
                ("dialect".to_string(), profile.dialect),
                ("key_env".to_string(), profile.key_env),
                ("models".to_string(), profile.models.join(", ")),
                ("api_key".to_string(), String::new()),
            ];
            // The caret starts at the end of the field.
            self.form_caret = usize::MAX;
            self.pending = Some(Pending::ProviderForm { name: value.to_string(), fields, at: 0 });
        } else if let Ok(provider) = crate::providers::Provider::parse(value) {
            let defaults = provider.profile();
            let fields = vec![
                ("base".to_string(), defaults.base_url.to_string()),
                ("model".to_string(), defaults.model.to_string()),
                ("dialect".to_string(), value.to_string()),
                ("key_env".to_string(), defaults.key_var.unwrap_or("").to_string()),
                ("models".to_string(), String::new()),
                ("api_key".to_string(), String::new()),
            ];
            // The caret starts at the end of the field.
            self.form_caret = usize::MAX;
            self.pending = Some(Pending::ProviderForm { name: format!("{value}-custom"), fields, at: 0 });
        }
        self.dirty = true;
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

    /// Save a shape as an always allow; the worker appends the line and
    /// reports back, so a failed write never shows as a live allow.
    fn save_rule(&mut self, shape: &str, tx: &Sender<Cmd>) {
        let shape = shape.trim();
        if self.allows.iter().any(|(s, _)| s == shape) {
            self.push(Role::Info, format!("{shape} is already allowed"));
            return;
        }
        let _ = tx.send(Cmd::AllowAlways(shape.to_string()));
    }

    /// The variable a provider's key lives in, or none when it needs no key.
    fn provider_key_var(&self, name: &str) -> Option<String> {
        if let Ok(provider) = crate::providers::Provider::parse(name) {
            return provider.profile().key_var.map(str::to_string);
        }
        self.cfg
            .named
            .iter()
            .find(|n| n.name == name)
            .map(|n| n.key_env.clone())
            .filter(|var| !var.is_empty())
    }

    /// First run: ask for a provider and its key before the first turn.
    fn begin_onboard(&mut self, var: String) {
        // The provider picker is the setup: it says what to do, so no line
        // names the missing key.
        let _ = var;
        self.onboard = true;
        self.pending = Some(Pending::Pick(self.value_picker(
            "provider",
            self.provider_items(),
            PickKind::Provider,
        )));
        self.dirty = true;
    }

    /// A chosen first-run provider: apply it, or ask for the key it needs.
    fn choose_onboard_provider(&mut self, name: &str, tx: &Sender<Cmd>) {
        if let Some(var) = self.provider_key_var(name) {
            self.pending = Some(Pending::KeyEntry {
                provider: name.to_string(),
                var,
                value: String::new(),
                status: String::new(),
            });
            self.dirty = true;
            return;
        }
        match self.set_provider(name, tx) {
            Ok(()) => {
                self.pending = None;
                self.onboard = false;
            }
            Err(e) => self.push(Role::Error, e),
        }
        self.dirty = true;
    }

    /// Check a first-run key, then save it and apply its provider.
    fn submit_key(&mut self, provider: String, var: String, value: String, tx: &Sender<Cmd>) {
        let again = |status: String| Pending::KeyEntry {
            provider: provider.clone(),
            var: var.clone(),
            value: value.clone(),
            status,
        };
        let key = value.trim().to_string();
        if key.is_empty() {
            self.pending = Some(again("the key is empty".into()));
            self.dirty = true;
            return;
        }
        self.pending = Some(again("checking…".into()));
        self.dirty = true;
        self.render();
        let warning = match first_run_outcome(self.check_key(&provider, &key)) {
            KeyOutcome::Retry(why) => {
                self.pending = Some(again(why));
                self.dirty = true;
                return;
            }
            KeyOutcome::Save(warning) => warning,
        };
        if let Err(e) = crate::config::save_key(&var, &key) {
            self.pending = Some(again(e));
            self.dirty = true;
            return;
        }
        self.cfg.api_key = key;
        self.pending = None;
        self.onboard = false;
        if let Err(e) = self.set_provider(&provider, tx) {
            self.push(Role::Error, e);
        }
        if let Some(warning) = warning {
            self.push(Role::Info, warning);
        }
        self.dirty = true;
    }

    /// One cheap request with the key, to catch a bad paste now.
    fn check_key(&self, name: &str, key: &str) -> Result<(), String> {
        if let Ok(provider) = crate::providers::Provider::parse(name) {
            return crate::providers::list_models(provider, provider.profile().base_url, key).map(|_| ());
        }
        let Some(profile) = self.cfg.named.iter().find(|n| n.name == name) else {
            return Err(format!("no provider {name:?}"));
        };
        let provider = crate::providers::Provider::parse(&profile.dialect)?;
        crate::providers::list_models(provider, &profile.base, key).map(|_| ())
    }

    /// Save a completed provider form, replacing any profile of that name.
    fn finish_provider_form(&mut self, name: String, fields: Vec<(String, String)>) {
        let value = |at: usize| fields.get(at).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
        let (base, model, dialect, key_env) = (value(0), value(1), value(2), value(3));
        if base.is_empty() || model.is_empty() || dialect.is_empty() {
            self.push(Role::Error, "provider: base, model, and dialect are required");
            // The caret starts at the end of the field.
            self.form_caret = usize::MAX;
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
            // The caret starts at the end of the field.
            self.form_caret = usize::MAX;
            self.pending = Some(Pending::ProviderForm {
                name,
                fields,
                at: 2,
            });
            self.dirty = true;
            return;
        }
        // A typed key goes to the key store under its variable name. It is never
        // written to the config or echoed.
        let api_key = value(5);
        if !api_key.is_empty() {
            if key_env.is_empty() {
                self.push(Role::Error, "provider: an API key needs a key variable (key_env) to save under");
                // The caret starts at the end of the field.
                self.form_caret = usize::MAX;
                self.pending = Some(Pending::ProviderForm { name, fields, at: 3 });
                self.dirty = true;
                return;
            }
            if let Err(e) = crate::config::save_key(&key_env, &api_key) {
                self.push(Role::Error, e);
                // The caret starts at the end of the field.
                self.form_caret = usize::MAX;
                self.pending = Some(Pending::ProviderForm { name, fields, at: 5 });
                self.dirty = true;
                return;
            }
        }
        // The models a provider already lists are kept until the form edits them.
        let models: Vec<String> = value(4).split(',').map(str::trim).filter(|m| !m.is_empty()).map(String::from).collect();
        let profile = crate::config::NamedProvider {
            name: name.clone(),
            base,
            model,
            models,
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
                let help = crate::help::commands(width.saturating_sub(2).max(40), true, self.palette.on);
                // Shown output keeps its codes; help's cyan becomes the accent.
                let accent = self.palette.paint(Role::Info, "").replace("\x1b[0m", "");
                self.push(Role::Info, format!("{RAW}{}", help.replace("\x1b[36m", &accent)))
            }
            "quit" | "exit" => self.quit = true,
            "keys" => self.open_keys(),
            "settings" => self.open_settings(),
            other if other == "diff" || other.starts_with("diff ") => {
                let _ = tx.send(Cmd::Diff(other["diff".len()..].trim().to_string()));
            }
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
                        let _ = tx.send(Cmd::SetAgent(crate::store::resolve_agent(name).to_string()));
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
                self.push(Role::Info, "reloading the config, instructions and skills…");
            }
            "sessions" => {
                self.pending = Some(Pending::Pick(self.session_picker()));
                self.dirty = true;
            }
            other if other.starts_with("sessions ") => {
                let id = other["sessions".len()..].trim().to_string();
                self.resume(id, tx);
            }
            "goal" => {
                let text = match &self.status.goal {
                    Some((goal, round, rounds)) => {
                        let state = if self.status.goal_paused { "paused" } else { "running" };
                        format!("goal: {goal} · round {round}/{rounds} · {state}")
                    }
                    None => "no goal; /goal <text> sets one".into(),
                };
                self.push(Role::Info, text);
            }
            "goal off" | "goal clear" => {
                let _ = tx.send(Cmd::GoalOff);
            }
            "goal pause" => {
                let _ = tx.send(Cmd::GoalPause);
            }
            "goal resume" => {
                // An empty goal text resumes the saved goal instead of starting one.
                let _ = tx.send(Cmd::Goal(String::new()));
            }
            other if other.starts_with("goal edit") => {
                let text = other["goal edit".len()..].trim().to_string();
                if let Some((goal, _, _)) = &mut self.status.goal {
                    *goal = text.clone();
                }
                let _ = tx.send(Cmd::GoalEdit(text));
            }
            other if other.starts_with("goal ") => {
                let text = other["goal ".len()..].trim().to_string();
                if self.status.goal.is_some() {
                    self.push(Role::Info, "a goal is set; replace it? y/N");
                    self.pending = Some(Pending::GoalReplace { text });
                    self.dirty = true;
                } else {
                    self.start_goal_now(text, tx);
                }
            }
            "copy" | "copy code" => {
                let last = self.entries.iter().rev().find(|e| e.role == Role::Assistant);
                let text = match (cmd, last) {
                    ("copy", Some(e)) => Some(e.text.clone()),
                    (_, Some(_)) => self
                        .entries
                        .iter()
                        .rev()
                        .filter(|e| e.role == Role::Assistant)
                        .find_map(|e| crate::render::fences(&e.text).pop()),
                    _ => None,
                };
                match text {
                    Some(text) => self.copy(text),
                    None => self.push(Role::Info, "nothing to copy"),
                }
            }
            "compact" => {
                let _ = tx.send(Cmd::Compact);
                self.push(Role::Info, "compacting…");
            }
            "retry" => self.retry(tx),
            other if other == "effort" || other.starts_with("effort ") => {
                let value = other.strip_prefix("effort").unwrap_or("").trim();
                if value.is_empty() {
                    let now = self.cfg.effort.as_deref().unwrap_or("default");
                    return self.toast(format!("effort: {now}"));
                }
                match crate::config::parse_effort(value) {
                    Ok(level) => {
                        if level.is_some() && !self.effort_noted && !crate::providers::has_effort(self.cfg.provider) {
                            self.effort_noted = true;
                            self.push(Role::Info, format!("{:?} has no effort setting; it is kept for other providers", self.cfg.provider).to_lowercase());
                        }
                        let _ = tx.send(Cmd::SetEffort(level.clone()));
                        // OpenAI-compatible providers take effort only while thinking.
                        let gated = level.is_some()
                            && !self.cfg.thinking
                            && !matches!(self.cfg.provider.caps().thinking, crate::providers::Thinking::None);
                        let note = if gated { " · applies with thinking on" } else { "" };
                        self.toast(format!("effort: {}{note}", level.as_deref().unwrap_or("default")));
                        self.cfg.effort = level;
                    }
                    Err(e) => self.push(Role::Error, e),
                }
            }
            "think" => {
                self.cfg.thinking = !self.cfg.thinking;
                let _ = tx.send(Cmd::SetThinking(self.cfg.thinking));
                self.toast(format!("thinking: {}", if self.cfg.thinking { "on" } else { "off" }));
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
                    self.toast(format!("theme: {rest}"));
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
                        picker.index = Verbosity::ALL.iter().position(|(l, _)| *l == self.verbosity).unwrap_or(1);
                        self.pending = Some(Pending::Pick(picker));
                        self.dirty = true;
                    }
                    level => match Verbosity::parse(level) {
                        Ok(v) => {
                            self.verbosity = v;
                            self.toast(format!("verbosity: {}", v.as_str()));
                        }
                        Err(e) => self.push(Role::Error, e),
                    },
                }
            }
            "shell" => {
                let _ = tx.send(Cmd::Shell);
            }
            "skills" => {
                let items = self.skill_items();
                self.pending = Some(Pending::Pick(Picker::new("skills", items, PickKind::Skill)));
                self.dirty = true;
            }
            other if other == "edit" || other.starts_with("edit ") || other == "open" || other.starts_with("open ") => {
                let gui = other == "open" || other.starts_with("open ");
                let rest = other.trim_start_matches(if gui { "open" } else { "edit" }).trim();
                // With no target, the session directory is the target.
                let target = if rest.is_empty() { self.root.display().to_string() } else { rest.to_string() };
                let inside = if rest.is_empty() { Ok(()) } else { check_target(&self.root, &target) };
                let argv = inside.and_then(|()| editor_argv_for(&self.cfg, crate::config::key_lookup(), gui, &target));
                match argv {
                    Ok(argv) => {
                        let _ = tx.send(if gui { Cmd::Open(argv) } else { Cmd::Edit(argv) });
                    }
                    Err(e) => self.push(Role::Error, e),
                }
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
                    self.title = rest.to_string();
                    let _ = tx.send(Cmd::RenameCurrent(rest.to_string()));
                    self.push(Role::Info, format!("renamed to {rest:?}"));
                }
            }
            other if other == "queue" || other.starts_with("queue ") => {
                let rest = other.strip_prefix("queue").unwrap_or("").trim();
                self.queue_command(rest);
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
                            self.title = title.clone();
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
            Pending::GoalReplace { text } => {
                match key {
                    Key::Char('y' | 'Y') => {
                        self.start_goal_now(text, tx);
                        self.pending = None;
                    }
                    Key::Char('n' | 'N') | Key::Esc | Key::Enter => {
                        self.pending = None;
                        self.push(Role::Info, "kept the current goal");
                    }
                    _ => self.pending = Some(Pending::GoalReplace { text }),
                }
                self.dirty = true;
                return;
            }
            Pending::SettingField { key: name, mut value, error } => {
                match key {
                    Key::Esc => self.open_settings_at(Some(&name)),
                    Key::Enter => match self.cfg.set(&name, &value) {
                        Ok(()) => {
                            let _ = tx.send(Cmd::SetConfig(name.clone(), value.clone()));
                            if let Some(file) = self.cfg.file.clone() {
                                if let Err(e) = crate::config::save_setting(&file, &name, &value) {
                                    self.toast(e);
                                }
                            }
                            self.open_settings_at(Some(&name));
                        }
                        Err(reason) => {
                            self.pending = Some(Pending::SettingField { key: name, value, error: reason });
                        }
                    },
                    Key::Char(c) if !c.is_control() => {
                        value.push(c);
                        self.pending = Some(Pending::SettingField { key: name, value, error: String::new() });
                    }
                    Key::Backspace => {
                        value.pop();
                        self.pending = Some(Pending::SettingField { key: name, value, error: String::new() });
                    }
                    _ => self.pending = Some(Pending::SettingField { key: name, value, error }),
                }
                self.dirty = true;
                return;
            }
            Pending::ProviderForm {
                name,
                mut fields,
                mut at,
            } => {
                let label = fields.get(at).map(|f| f.0.clone()).unwrap_or_default();
                match key {
                    Key::Esc => {
                        self.push(Role::Info, "provider cancelled");
                        self.dirty = true;
                    }
                    // The dialect is chosen from a list: Left, Right and Space step it.
                    Key::Left | Key::Right | Key::Char(' ') if label == "dialect" => {
                        let current = fields[at].1.clone();
                        let next = match DIALECTS.iter().position(|d| *d == current) {
                            None => 0,
                            Some(i) if matches!(key, Key::Left) => (i + DIALECTS.len() - 1) % DIALECTS.len(),
                            Some(i) => (i + 1) % DIALECTS.len(),
                        };
                        fields[at].1 = DIALECTS[next].to_string();
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    // Typed text cannot reach the dialect.
                    Key::Char(_) | Key::Backspace if label == "dialect" => {
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                    }
                    // Tab on the model opens the models to choose from.
                    Key::Tab if label == "model" => {
                        self.open_model_picker(name, fields, at);
                        self.dirty = true;
                    }
                    Key::Enter => {
                        if at + 1 < fields.len() {
                            at += 1;
                            self.form_caret = fields.get(at).map_or(0, |f| f.1.len());
                            self.pending = Some(Pending::ProviderForm { name, fields, at });
                        } else {
                            self.finish_provider_form(name, fields);
                        }
                        self.dirty = true;
                    }
                    Key::Char(c) => {
                        if let Some(field) = fields.get_mut(at) {
                            let caret = self.form_caret.min(field.1.len());
                            field.1.insert(caret, c);
                            self.form_caret = caret + c.len_utf8();
                        }
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        if let Some(field) = fields.get_mut(at) {
                            let caret = self.form_caret.min(field.1.len());
                            if let Some(prev) = field.1[..caret].chars().last() {
                                let from = caret - prev.len_utf8();
                                field.1.replace_range(from..caret, "");
                                self.form_caret = from;
                            }
                        }
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    // Left and Right move the caret through a text field.
                    Key::Left | Key::Right if label != "dialect" => {
                        if let Some(field) = fields.get(at) {
                            let caret = self.form_caret.min(field.1.len());
                            self.form_caret = if matches!(key, Key::Left) {
                                field.1[..caret].chars().last().map_or(0, |c| caret - c.len_utf8())
                            } else {
                                field.1[caret..].chars().next().map_or(caret, |c| caret + c.len_utf8())
                            };
                        }
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    // Back to an earlier field to fix it; each field keeps its text.
                    Key::Up | Key::BackTab => {
                        at = at.saturating_sub(1);
                        self.form_caret = fields.get(at).map_or(0, |f| f.1.len());
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    Key::Down => {
                        at = (at + 1).min(fields.len().saturating_sub(1));
                        self.form_caret = fields.get(at).map_or(0, |f| f.1.len());
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::ProviderForm { name, fields, at });
                    }
                }
                return;
            }
            Pending::AllowRule { mut value } => {
                match key {
                    Key::Esc => {
                        self.pending = None;
                        self.push(Role::Info, "allow cancelled");
                        self.dirty = true;
                    }
                    Key::Enter => {
                        let shape = value.trim().to_string();
                        if shape.is_empty() {
                            self.push(Role::Error, "allow: a shape is needed");
                            self.pending = Some(Pending::AllowRule { value });
                        } else {
                            self.pending = None;
                            self.save_rule(&shape, tx);
                        }
                        self.dirty = true;
                    }
                    Key::Char(c) if !c.is_control() => {
                        value.push(c);
                        self.pending = Some(Pending::AllowRule { value });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        value.pop();
                        self.pending = Some(Pending::AllowRule { value });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::AllowRule { value });
                    }
                }
                return;
            }
            Pending::KeyEntry {
                provider,
                var,
                mut value,
                status,
            } => {
                match key {
                    Key::Esc => {
                        self.onboard = false;
                        self.pending = None;
                        self.push(Role::Info, "no key entered; nothing saved");
                        self.quit = true;
                        self.dirty = true;
                    }
                    Key::Enter => self.submit_key(provider, var, value, tx),
                    Key::Char(c) if !c.is_control() => {
                        value.push(c);
                        self.pending = Some(Pending::KeyEntry { provider, var, value, status });
                        self.dirty = true;
                    }
                    Key::Backspace => {
                        value.pop();
                        self.pending = Some(Pending::KeyEntry { provider, var, value, status });
                        self.dirty = true;
                    }
                    _ => {
                        self.pending = Some(Pending::KeyEntry { provider, var, value, status });
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
                    Key::Esc if picker.form.is_some() => {
                        if let Some(form) = picker.form.take() {
                            let (name, fields, at) = *form;
                            self.pending = Some(Pending::ProviderForm { name, fields, at });
                        }
                        self.dirty = true;
                        return;
                    }
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
                    // Ctrl+E configures the highlighted provider from a typed
                    // `/provider`, where letters filter the list.
                    Key::Ctrl('u') if picker.kind == PickKind::Provider => {
                        if let Some(choice) = picker.selected().cloned() {
                            self.use_provider(&choice.value, tx);
                        }
                        self.dirty = true;
                        return;
                    }
                    Key::Ctrl('e') if picker.kind == PickKind::Provider => {
                        if let Some(choice) = picker.selected().cloned() {
                            self.configure_provider(&choice.value);
                        }
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
                            Some("add") => {
                                self.pending = Some(Pending::AllowRule { value: String::new() });
                            }
                            Some("configure") => {
                                if let Some(choice) = picker.selected().cloned() {
                                    self.configure_provider(&choice.value);
                                }
                            }
                            Some("use") => {
                                if let Some(choice) = picker.selected().cloned() {
                                    self.use_provider(&choice.value, tx);
                                }
                            }
                            Some("allow") => {
                                if let Some(choice) = picker.selected() {
                                    let shape = choice.value.trim_start_matches("asked:").to_string();
                                    self.save_rule(&shape, tx);
                                    picker.items = self.mode_items();
                                    picker.offset = picker.offset.min(picker.index);
                                }
                                self.pending = Some(Pending::Pick(picker));
                            }
                            Some("revoke") => {
                                if let Some(choice) = picker.selected() {
                                    let value = choice.value.clone();
                                    let shape = value.trim_start_matches("allow:").to_string();
                                    let always = self
                                        .allows
                                        .iter()
                                        .any(|(s, a)| s == &shape && *a);
                                    self.allows.retain(|(s, _)| s != &shape);
                                    let _ = tx.send(Cmd::RevokeAllow {
                                        shape: shape.clone(),
                                        always,
                                    });
                                    picker.items.retain(|i| i.value != value);
                                    picker.index = picker.index.min(picker.items.len().saturating_sub(1));
                                    picker.offset = picker.offset.min(picker.index);
                                    self.push(Role::Info, format!("revoked {shape}"));
                                    self.pending = Some(Pending::Pick(picker));
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
                    // The settings form steps a value on Space or the arrows; Space
                    // never reaches the filter there.
                    Key::Char(' ') | Key::Right | Key::Left if picker.kind == PickKind::Settings => {
                        let dir = if matches!(key, Key::Left) { -1 } else { 1 };
                        self.step_setting(&mut picker, dir, tx);
                        self.pending = Some(Pending::Pick(picker));
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
                                self.submit(tx, false);
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
                            }
                            PickKind::Session => self.resume(choice.value, tx),
                            PickKind::Model => {
                                // A row from another provider names it as "provider::model".
                                match choice.value.split_once("::") {
                                    Some((provider, model)) => match self.set_provider(provider, tx) {
                                        Ok(()) => self.set_model(model, tx),
                                        Err(e) => self.push(Role::Error, e),
                                    },
                                    None => self.set_model(&choice.value, tx),
                                }
                            }
                            PickKind::Palette => {
                                let value = choice.value.clone();
                                self.pending = None;
                                if let Some(cmd) = value.strip_prefix("cmd:") {
                                    self.run_command(cmd, tx);
                                } else if let Some(action) = value.strip_prefix("key:") {
                                    self.palette_action(action, tx);
                                }
                                self.dirty = true;
                                return;
                            }
                            PickKind::Mode if choice.value == "add-rule" => {
                                self.pending = Some(Pending::AllowRule { value: String::new() });
                                self.dirty = true;
                                return;
                            }
                            PickKind::Mode if choice.value.starts_with("asked:") => {
                                let shape = choice.value.trim_start_matches("asked:").to_string();
                                self.pending = Some(Pending::AllowRule { value: shape });
                                self.dirty = true;
                                return;
                            }
                            PickKind::Mode if choice.value.starts_with("allow:") => {
                                // An allow row is not a mode; choosing it does nothing.
                                self.pending = Some(Pending::Pick(picker));
                                self.dirty = true;
                                return;
                            }
                            PickKind::Mode => self.set_mode(&choice.value, tx),
                            PickKind::Keys => {}
                            PickKind::ProviderModel => {
                                if let Some(form) = picker.form.take() {
                                    let (name, mut fields, _) = *form;
                                    let model_at = fields.iter().position(|f| f.0 == "model").unwrap_or(1);
                                    if choice.value != TYPE_ANOTHER {
                                        fields[model_at].1 = choice.value.clone();
                                    }
                                    self.pending = Some(Pending::ProviderForm { name, fields, at: model_at });
                                }
                                self.dirty = true;
                                return;
                            }
                            PickKind::Settings => {
                                let key = choice.value.clone();
                                if is_text_setting(&key) {
                                    // Starts empty: the row shows the current value, and typing gives the new one.
                                    self.pending = Some(Pending::SettingField { key, value: String::new(), error: String::new() });
                                } else if key == "api_key" {
                                    match self.provider_key_var(&self.cfg.active_name.clone()) {
                                        Some(var) => {
                                            self.pending = Some(Pending::KeyEntry {
                                                provider: self.cfg.active_name.clone(),
                                                var,
                                                value: String::new(),
                                                status: String::new(),
                                            });
                                        }
                                        None => {
                                            self.push(Role::Error, "this provider has no key to set");
                                            self.pending = Some(Pending::Pick(picker));
                                        }
                                    }
                                } else {
                                    self.step_setting(&mut picker, 1, tx);
                                    self.pending = Some(Pending::Pick(picker));
                                }
                                self.dirty = true;
                                return;
                            }
                            PickKind::Verbosity => {
                                self.verbosity = Verbosity::parse(&choice.value).unwrap_or(self.verbosity);
                                self.toast(format!("verbosity: {}", choice.value));
                            }
                            PickKind::Skill => {
                                let name = choice.value.clone();
                                let enable = self.cfg.skill_disabled(&name);
                                let key = format!("skill.{name}.enabled");
                                let text = if enable { "true" } else { "false" };
                                match crate::config::save_setting(&self.config_path, &key, text) {
                                    Ok(()) => {
                                        let _ = self.cfg.set(&key, text);
                                        // The running session picks this up on /reload.
                                        self.toast(format!("{name}: {} · applies on /reload", if enable { "on" } else { "off" }));
                                    }
                                    Err(e) => self.push(Role::Error, e),
                                }
                            }
                            PickKind::Theme => {
                                let _ = crate::theme::save(&choice.value);
                                if let Some(theme) = crate::theme::by_name(&choice.value) {
                                    self.palette = self.palette.with_theme(theme);
                                }
                                // Chosen, so closing must not restore.
                                self.preview_from = None;
                                self.toast(format!("theme: {}", choice.value));
                            }
                            PickKind::Provider if self.onboard => {
                                self.choose_onboard_provider(&choice.value, tx);
                            }
                            // Enter opens the form; switching is `u`.
                            PickKind::Provider => {
                                self.configure_provider(&choice.value);
                                self.dirty = true;
                                return;
                            }
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
                        if self.onboard {
                            self.push(Role::Info, "no key entered; nothing saved");
                            self.quit = true;
                        }
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
                    Key::Char('n' | 'N') => Answer::No,
                    // Esc declines and stops the turn, so the prompt comes back.
                    Key::Esc => {
                        self.cancel.store(true, Ordering::Relaxed);
                        Answer::No
                    }
                    _ => {
                        self.pending = Some(Pending::Confirm { what, reply, always });
                        return;
                    }
                };
                // The tool row carries the result, a decline included, so
                // the answer needs no row of its own.
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
                    Key::Esc => {
                        self.cancel.store(true, Ordering::Relaxed);
                        let _ = reply.send(None);
                        self.dirty = true;
                    }
                    Key::Char('y') | Key::Char('Y') if yes_no(&opts) && !self.ask_multiple => {
                        let _ = reply.send(Some(opts[0].clone()));
                        self.dirty = true;
                    }
                    Key::Char('n') | Key::Char('N') if yes_no(&opts) && !self.ask_multiple => {
                        let _ = reply.send(Some(opts[1].clone()));
                        self.dirty = true;
                    }
                    Key::Char(c) if c.is_ascii_digit() => {
                        let n = c.to_digit(10).unwrap_or(0) as usize;
                        let valid = n >= 1 && n <= opts.len();
                        if valid && !self.ask_multiple {
                            let _ = reply.send(Some(opts[n - 1].clone()));
                        } else {
                            // Several picks: the key marks or unmarks. Out of
                            // range: the question stays open.
                            if valid {
                                if let Some(m) = self.ask_marked.get_mut(n - 1) {
                                    *m = !*m;
                                }
                            }
                            self.pending = Some(Pending::Question { q, opts, reply, mode: AskMode::Choose, typed });
                        }
                        self.dirty = true;
                    }
                    Key::Enter if self.ask_multiple => {
                        let picked: Vec<String> =
                            opts.iter().zip(&self.ask_marked).filter(|(_, m)| **m).map(|(o, _)| o.clone()).collect();
                        if picked.is_empty() {
                            self.pending = Some(Pending::Question { q, opts, reply, mode: AskMode::Choose, typed });
                        } else {
                            let _ = reply.send(Some(picked.join(", ")));
                        }
                        self.dirty = true;
                    }
                    // Typing starts the answer, keeping the key typed.
                    Key::Char(c) => {
                        if c != ' ' {
                            typed.push(c);
                        }
                        self.ask_caret = typed.len();
                        self.pending = Some(Pending::Question { q, opts, reply, mode: AskMode::Custom, typed });
                        self.dirty = true;
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
                        self.cancel.store(true, Ordering::Relaxed);
                        let _ = reply.send(None);
                        self.dirty = true;
                    }
                    Key::Enter if typed.is_empty() => {
                        self.pending = Some(Pending::Question { q, opts, reply, mode: AskMode::Custom, typed });
                    }
                    Key::Enter => {
                        let _ = reply.send(Some(typed.clone()));
                        self.dirty = true;
                    }
                    key => {
                        if edit_line(&mut typed, &mut self.ask_caret, &key, &self.clip) {
                            self.dirty = true;
                        }
                        self.pending = Some(Pending::Question { q, opts, reply, mode: AskMode::Custom, typed });
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
        (self.tool.as_ref().is_some_and(|row| row.running) || self.thinking_row().is_some())
            && self.last_redraw.elapsed() >= Duration::from_millis(100)
    }

    /// What a task cost, where imp knows the provider's prices.
    fn task_cost(&self, hit: u64, miss: u64, out: u64) -> Option<f64> {
        (self.cfg.provider == crate::providers::Provider::DeepSeek)
            .then(|| crate::pricing::cost(&self.cfg.model, crate::pricing::now(), hit, miss, out))
            .flatten()
    }

    /// Start counting a task: its time, steps, and tokens.
    fn begin_task(&mut self) {
        // Ctrl+E reaches the rows of the latest turn only.
        self.tool = None;
        self.done.clear();
        let now = Instant::now();
        let s = &mut self.status;
        (s.task_start, s.steps, s.chars, s.task_usage, s.quiet_since) = (Some(now), 0, 0, (0, 0, 0), Some(now));
    }

    /// `⠋ thinking · 4s` while the model works and nothing shows it.
    fn thinking_row(&self) -> Option<(Role, Line)> {
        let since = self.status.quiet_since?;
        let busy = self.status.streaming && self.pending.is_none();
        let tool = self.tool.as_ref().is_some_and(|row| row.running);
        (busy && !tool).then(|| {
            use crate::render::{Span, Style};
            let glyph = spinner_frame(self.spinner);
            let rest = format!(" thinking · {}", span(since.elapsed()));
            (
                Role::Thinking,
                Line::Spans(vec![
                    Span::styled(glyph, Style::default()),
                    Span::styled(rest, Style { italic: true, ..Style::default() }),
                ]),
            )
        })
    }

    /// Ids of live sessions whose agent is working.
    /// Each open session's mark, keyed by session id. A closed session has none.
    fn session_marks(&self) -> Vec<(String, char)> {
        let frame = spinner_frame(self.spinner);
        let mut out = Vec::new();
        let mut add = |worker: Option<&Worker>, waiting: bool, streaming: bool, failed: bool, replied: bool| {
            let Some(id) = worker.and_then(|w| w.2.lock().ok().map(|s| s.clone())).filter(|s| !s.is_empty()) else {
                return;
            };
            if let Some(mark) = state_mark(waiting, streaming, failed, replied, frame) {
                out.push((id, mark));
            }
        };
        let asking = |pending: &Option<Pending>| matches!(pending, Some(Pending::Confirm { .. } | Pending::Question { .. }));
        add(self.worker.as_ref(), asking(&self.pending), self.status.streaming, self.failed, self.replied);
        for p in self.panes.iter().flatten() {
            add(p.worker.as_ref(), asking(&p.pending), p.status.streaming, p.failed, p.replied);
        }
        out
    }

    /// Repaint the spinners in an open session picker, ten times a second.
    /// True when a row changed.
    fn spin_sessions(&mut self) -> bool {
        if self.last_redraw.elapsed() < Duration::from_millis(100)
            || !matches!(&self.pending, Some(Pending::Pick(p)) if p.kind == PickKind::Session)
        {
            return false;
        }
        if !self.tool.as_ref().is_some_and(|row| row.running) {
            self.spinner += 1;
        }
        let marks = self.session_marks();
        let Some(Pending::Pick(p)) = &mut self.pending else { return false };
        let before: Vec<String> = p.items.iter().map(|i| i.desc.clone()).collect();
        mark_rows(&mut p.items, &marks);
        p.items.iter().zip(&before).any(|(i, b)| &i.desc != b)
    }

    /// How long the loop may wait for a key: short while anything moves,
    /// long otherwise, an open prompt included. `quiet` is the time since
    /// the last key or event.
    fn loop_wait(&self, quiet: Duration) -> Duration {
        let wait = self.usual_wait(quiet);
        // A toast also wakes the loop when it is due to go, never later.
        match &self.status.toast {
            Some((_, at)) => wait.min(TOAST.saturating_sub(at.elapsed()).max(Duration::from_millis(50))),
            None => wait,
        }
    }

    fn usual_wait(&self, quiet: Duration) -> Duration {
        let streaming = self.status.streaming || self.panes.iter().flatten().any(|p| p.status.streaming);
        if streaming || self.tool.as_ref().is_some_and(|row| row.running) || quiet < Duration::from_secs(2) {
            Duration::from_millis(50)
        } else {
            Duration::from_secs(5)
        }
    }

    /// Advance the running row's glyph in place, with its time, or mark it
    /// as waiting on the user.
    fn tick_spinner(&mut self) {
        self.spinner += 1;
        let frame = spinner_frame(self.spinner);
        let waiting = matches!(self.pending, Some(Pending::Confirm { .. } | Pending::Question { .. }));
        if let Some(row) = &self.tool {
            if row.running {
                if let Some(entry) = self.entries.get_mut(row.index) {
                    let took = row.started.elapsed();
                    entry.text = if waiting {
                        format!("▌ {} · waiting for you", row.summary)
                    } else if took >= Duration::from_secs(1) {
                        format!("{frame} {} · {}", row.summary, span(took))
                    } else {
                        format!("{frame} {}", row.summary)
                    };
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
        SCREEN.store((cols as u32) << 16 | rows as u32 & 0xffff, Ordering::Relaxed);
        if rows < 5 || cols < 20 {
            return;
        }
        let screen = self.frame_changes(cols, rows);
        let mut out = std::io::stdout();
        out.write_all(screen.as_bytes()).ok();
        if let Some(seq) = self.osc52.take() {
            out.write_all(seq.as_bytes()).ok();
        }
        if let Some(seq) = self.mouse_seq.take() {
            out.write_all(seq.as_bytes()).ok();
        }
        out.flush().ok();
        self.bell_pending = false;
        self.dirty = false;
        self.last_redraw = Instant::now();
    }

    /// The frame's rows, top to bottom, and what draws over them.
    fn layout(&mut self, cols: usize, rows: usize) -> (Vec<String>, String) {
        let mut composer = self.composer(cols, rows.saturating_sub(1).max(1));
        // The queue sits above the draft, never above a prompt's rows, whose
        // click targets count from the rule.
        if self.pending.is_none() && !self.queued.is_empty() && rows >= 8 {
            let mut both = self.queue_rows(cols);
            both.append(&mut composer);
            composer = both;
        }
        // A rule sets the composer apart, unless the screen is too short.
        let rule = usize::from(rows >= 8);
        // So does the title bar.
        let title = rule;
        let body_rows = rows.saturating_sub(1 + rule + title + composer.len()).max(1);
        let bar = (title == 1).then(|| self.title_bar(cols));

        let markdown = self.cfg.links.supported();
        // Only entries whose text, role, width, or turn gap changed render again.
        self.drawn.truncate(self.entries.len());
        let mut spoke = false;
        for (i, entry) in self.entries.iter().enumerate() {
            // One blank row opens every turn after the first.
            let gap = entry.role == Role::User && spoke;
            spoke |= entry.role == Role::User;
            // Replies leave two columns for their copy label.
            let width = if self.cfg.mouse && entry.role == Role::Assistant { cols.saturating_sub(2) } else { cols };
            // Wide terminals draw a removed line and the added line after it as
            // one row. The first of the pair draws the row; the second draws
            // nothing. Pairing looks only at neighbours, so the cache still holds.
            let wide = side_by_side(cols);
            let pair_first = wide
                && entry.role == Role::Delete
                && self.entries.get(i + 1).is_some_and(|n| n.role == Role::Add);
            let pair_second = wide && entry.role == Role::Add && i > 0 && self.entries[i - 1].role == Role::Delete;
            let partner = if pair_first { self.entries[i + 1].text.as_str() } else { "" };
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (width, markdown, gap, entry.role as u8, &entry.text, &entry.marks, partner, pair_second).hash(&mut h);
            let key = h.finish();
            if self.drawn.get(i).map(|slot| slot.0) != Some(key) {
                let slot = (
                    key,
                    if pair_first {
                        let old = entry.text.get(3..).unwrap_or("");
                        let new = partner.get(3..).unwrap_or("");
                        vec![(Role::Delete, Line::Text(sbs_line(old, new, cols)))]
                    } else if pair_second {
                        Vec::new()
                    } else {
                        entry_lines(entry, width, markdown, gap)
                    },
                );
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
        let thinking = self.thinking_row();

        let total = self.drawn.iter().map(|slot| slot.1.len()).sum::<usize>()
            + usize::from(hint.is_some())
            + usize::from(thinking.is_some());
        let max_top = total.saturating_sub(body_rows);
        self.body_rows = body_rows;
        self.max_top = max_top;
        if self.follow {
            self.top = max_top;
        }
        self.top = self.top.min(max_top);
        self.unseen = total.saturating_sub(self.top + body_rows);
        // Each row with the copy label it carries, if any: a reply's first
        // row, and the first row of each fenced block drawn as code.
        let mut labels: Vec<Option<Target>> = Vec::new();
        for (i, slot) in self.drawn.iter().enumerate() {
            let reply = self.cfg.mouse && self.entries[i].role == Role::Assistant;
            // The latest turn's tool rows open on a click, unless an overlay
            // sits over them and the click is meant for it.
            let covered = matches!(self.pending, Some(Pending::Pick(_) | Pending::ConfirmDelete { .. }));
            let kept = self.cfg.mouse && !covered && self.done.iter().chain(&self.tool).any(|r| r.index == i);
            let mut first = true;
            let mut block = 0;
            let mut in_fence = false;
            for (_, line) in &slot.1 {
                let fence = matches!(line, Line::Spans(s) if s.iter().any(|s| s.style.fence));
                let blank = matches!(line, Line::Text(t) if t.is_empty());
                let label = if kept && !blank {
                    Some(Target::Tool(i))
                } else if !reply || blank {
                    None
                } else if fence && !in_fence {
                    block += 1;
                    Some(Target::Copy(i, Some(block - 1)))
                } else if first {
                    Some(Target::Copy(i, None))
                } else {
                    None
                };
                first &= blank;
                in_fence = fence;
                labels.push(label);
            }
        }
        labels.extend(hint.map(|_| None));
        labels.extend(thinking.as_ref().map(|_| None));
        let labels: Vec<Option<Target>> = labels.into_iter().skip(self.top).take(body_rows).collect();
        let visible: Vec<&(Role, Line)> = self
            .drawn
            .iter()
            .flat_map(|slot| slot.1.iter())
            .chain(hint)
            .chain(thinking.as_ref())
            .skip(self.top)
            .take(body_rows)
            .collect();
        self.targets.clear();
        let lead = body_rows.saturating_sub(visible.len());

        let mut screen: Vec<String> = Vec::with_capacity(rows);
        screen.extend(bar);
        for _ in 0..body_rows.saturating_sub(visible.len()) {
            screen.push(String::new());
        }
        for (n, (role, line)) in visible.into_iter().enumerate() {
            let mut row = String::new();
            match line {
                Line::Text(text) => {
                    let mut text = fit(text, cols);
                    // A diff band runs to the edge; without colour there is no band.
                    if self.palette.on && matches!(role, Role::Add | Role::Delete) {
                        let used = text.chars().count();
                        text.push_str(&" ".repeat(cols.saturating_sub(used)));
                    }
                    let painted = self.marked(*role, &text);
                    // Muted asides, reasoning first, are italic.
                    let italic = if *role == Role::Thinking { "3" } else { "" };
                    row.push_str(&self.palette.styled(italic, painted));
                }
                Line::Spans(spans) => {
                    row.push_str(&self.paint_spans(*role, spans));
                    if let Some(target) = labels[n].clone() {
                        let used: usize = spans.iter().map(|s| s.text.chars().count()).sum();
                        row.push_str(&" ".repeat(cols.saturating_sub(used + 1)));
                        row.push_str(&self.palette.paint(Role::Thinking, &COPY.to_string()));
                        self.targets.push((title + lead + n + 1, cols.saturating_sub(1), cols, target));
                    }
                }
                Line::Diff(text, marks) => row.push_str(&self.paint_diff(*role, text, marks, cols)),
                Line::Timed(text, time) => {
                    row.push_str(&self.marked(*role, text));
                    let gap = cols.saturating_sub(text.chars().count() + time.chars().count());
                    row.push_str(&" ".repeat(gap));
                    row.push_str(&self.palette.paint(Role::Thinking, time));
                }
            }
            if let Some(Target::Tool(i)) = &labels[n] {
                self.targets.push((title + lead + n + 1, 1, cols, Target::Tool(*i)));
            }
            screen.push(row);
        }

        if rule == 1 {
            // The rule takes the accent while a question waits below it.
            let asking = self.pending.is_some() && !matches!(self.pending, Some(Pending::Pick(_)));
            let tone = if asking {
                Role::Info
            } else if self.yolo() {
                Role::Error
            } else {
                Role::Border
            };
            // While a turn runs the rule says what Enter will do.
            let label = match (self.queue_at, asking || !self.status.streaming) {
                (Some(i), _) => {
                    let verb = if self.status.streaming { "steer" } else { "send" };
                    format!(
                        "editing {}/{} · Enter save · ^S {verb} · ^X del",
                        i + 1,
                        self.queued.len()
                    )
                }
                (None, true) if self.status.suggestion.is_some() => "→ accept".into(),
                // ↑ does nothing while a question or confirmation is open.
                (None, true) if !asking && !self.queued.is_empty() => "↑ edit queue".into(),
                (None, true) => String::new(),
                // The prompt glyph shows steer or queue; the rule stays quiet.
                (None, false) => String::new(),
            };
            let rule = if label.is_empty() || cols < label.chars().count() + 6 {
                "─".repeat(cols)
            } else {
                format!("── {label} {}", "─".repeat(cols - label.chars().count() - 4))
            };
            screen.push(self.palette.paint(tone, &rule));
        }
        let first = title + body_rows + rule + 1;
        for (i, row) in composer.iter().enumerate() {
            screen.push(row.clone());
            if matches!(self.pending, Some(Pending::Confirm { .. } | Pending::Question { .. })) {
                self.targets.extend(chip_targets(first + i, &unpaint(row)));
            }
        }
        screen.push(self.paint_status(cols));

        let mut tail = String::new();
        if let Some((overlay, cursor_x, cursor_y, targets)) = self.overlay(cols, rows) {
            self.targets.extend(targets);
            tail.push_str("\x1b[?7l");
            tail.push_str(&overlay);
            if matches!(self.pending, Some(Pending::Pick(_))) {
                // Only an open filter takes the cursor.
                tail.push_str(&format!("\x1b[{cursor_y};{cursor_x}H\x1b[?25h"));
            }
            tail.push_str("\x1b[?7h");
        }
        (screen, tail)
    }

    /// One frame as bytes, every row written. No terminal access, so it can
    /// be tested.
    #[cfg(test)]
    pub fn frame(&mut self, cols: usize, rows: usize) -> String {
        let (mut screen, tail) = self.layout(cols, rows);
        highlight(&mut screen, self.drag);
        self.shown.clear();
        let out = paint_rows(&screen, None, cols, &tail);
        self.shown = screen;
        self.shown_size = (cols, rows);
        self.shown_overlay = !tail.is_empty();
        out
    }

    /// One frame as bytes, writing only rows that differ from the last one.
    /// ponytail: an overlay or a resize repaints every row, since the overlay
    /// draws over rows with cursor moves; compose it into the rows if pickers flicker.
    fn frame_changes(&mut self, cols: usize, rows: usize) -> String {
        let (mut screen, tail) = self.layout(cols, rows);
        highlight(&mut screen, self.drag);
        let full = self.shown_size != (cols, rows) || self.shown_overlay || !tail.is_empty();
        let out = paint_rows(&screen, (!full).then_some(&self.shown[..]), cols, &tail);
        self.shown = screen;
        self.shown_size = (cols, rows);
        self.shown_overlay = !tail.is_empty();
        out
    }

    /// A diff row: each run of characters on the row's tint, twice as strong
    /// for changed words, to the screen edge. Plain text with colour off.
    fn paint_diff(&self, role: Role, text: &str, marks: &[u8], cols: usize) -> String {
        if !self.palette.on {
            return fit(text, cols);
        }
        let chars: Vec<char> = text.chars().collect();
        let lead = chars.iter().take_while(|c| **c == ' ').count();
        let mut out: String = chars[..lead].iter().collect();
        let mut i = lead;
        while i < chars.len() {
            let m = marks.get(i).copied().unwrap_or(0);
            let j = (i..chars.len()).find(|&k| marks.get(k).copied().unwrap_or(0) != m).unwrap_or(chars.len());
            out.push_str(&self.palette.diff_run(role, &chars[i..j].iter().collect::<String>(), m));
            i = j;
        }
        if chars.len() < cols {
            out.push_str(&self.palette.diff_run(role, &" ".repeat(cols - chars.len()), 0));
        }
        out
    }

    /// A tool row: the leading `✓` bold in the success colour, `!` in the
    /// accent, `✗` in the error colour or muted when declined, and the rest
    /// in the row's own role.
    fn marked(&self, role: Role, text: &str) -> String {
        // `!` and a declined `✗` count only at the start of a row.
        let found = text
            .find(['✓', '✗'])
            .filter(|_| matches!(role, Role::Tool | Role::Error))
            .or_else(|| (role == Role::Tool && text.starts_with("! ")).then_some(0))
            .or_else(|| (role == Role::Context && text.starts_with("✗ ")).then_some(0));
        let Some(at) = found.filter(|_| self.palette.on) else {
            return self.palette.paint(role, text);
        };
        let mark = text[at..].chars().next().unwrap_or('✓');
        let tone = match (mark, role) {
            ('✓', _) => Role::Success,
            ('!', _) => Role::Info,
            (_, Role::Context) => Role::Context,
            _ => Role::Error,
        };
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
            // Shown output brings its own colours, or none when colour is off.
            if span.style.raw {
                if self.palette.on {
                    out.push_str(&span.text);
                } else {
                    out.push_str(&crate::render::clean(&span.text));
                }
                continue;
            }
            let colour = if span.style.border {
                Role::Border
            } else if span.style.code || span.style.fence {
                Role::Code
            } else if span.link.is_some() {
                Role::Link
            } else if span.style.dim {
                Role::Thinking
            } else if matches!(span.style.heading, 1 | 2) || span.style.marker {
                Role::Info
            } else {
                role
            };
            let mut codes = Vec::new();
            if span.style.bold {
                codes.push("1");
            }
            if span.style.italic {
                codes.push("3");
            }
            if span.style.heading == 1 || span.link.is_some() {
                codes.push("4");
            }
            let painted = if span.style.code {
                self.palette.code(&span.text)
            } else {
                self.palette.paint(colour, &span.text)
            };
            let mut text = self.palette.styled(&codes.join(";"), painted);
            if let Some(url) = &span.link {
                text = crate::render::hyperlink(url, &text);
            }
            out.push_str(&text);
        }
        out
    }

    /// The confirmation row: question and key chips. Keys are bold accent,
    /// labels muted. It waits until answered.
    fn confirm_row(&self, what: &str, always: bool, cols: usize) -> String {
        let chips: &[(&str, &str)] = if always { &ALL_CHIPS } else { &ALL_CHIPS[..2] };
        let keys = chip_text(chips);
        // The command is on screen, in its tool row or above this one.
        let ask = verb(what).to_string();
        let bell = if self.bell_pending { "\u{7}" } else { "" };
        // "▌ " + question + "   " + chips.
        let used = 2 + ask.chars().count() + 3 + keys.chars().count();
        let p = &self.palette;
        if used > cols {
            // Too narrow for chips: one plain accent line, cut at the edge.
            let plain = format!("▌ {ask}   {keys}");
            return format!("{bell}{}", p.paint(Role::Info, &fit(&plain, cols)));
        }
        let mut row = format!("{bell}{}   ", p.paint(Role::Info, &format!("▌ {ask}")));
        row.push_str(&self.paint_chips(chips));
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
            if self.armed && *k == "a" {
                // Marked by a first click: reversed, waiting for the second.
                row.push_str(&p.select(&format!("[{k}] {l}")));
                continue;
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
                // The boxed form draws every field, so the composer row stays free.
                Pending::ProviderForm { .. } => return None,
                Pending::GoalReplace { text } => format!("replace the goal with \"{text}\"? y/N"),
                Pending::SettingField { key, value, error } => {
                    let now = self.setting_value(key);
                    let refused = if error.is_empty() { String::new() } else { format!("  refused: {error}") };
                    format!("{key} (now {now}): {value}{CARET}{refused}  (Enter save · Esc back)")
                }
                Pending::AllowRule { value } => {
                    format!("allow = {value}{CARET}  (Enter save · Esc cancel)")
                }
                Pending::KeyEntry { provider, var, value, status } => {
                    let stars = "*".repeat(value.chars().count());
                    let note = if status.is_empty() { String::new() } else { format!("  {status}") };
                    format!("{provider} key for {var}: {stars}{CARET}  (Enter save · Esc cancel){note}")
                }
                Pending::Question { .. } => return None,
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
    #[allow(clippy::type_complexity)]
    /// The provider form as a box. A text field shows its caret; the dialect
    /// shows arrows; the key row shows dots, never the key.
    fn provider_form_overlay(
        &self,
        name: &str,
        fields: &[(String, String)],
        at: usize,
        caret: usize,
        cols: usize,
        rows: usize,
    ) -> Option<(String, usize, usize, Vec<(usize, usize, usize, Target)>)> {
        let width = (cols * 80 / 100).clamp(40, 72).min(cols);
        if width < 4 || rows < 8 {
            return None;
        }
        let inner = width.saturating_sub(2);
        let wall = self.palette.paint(Role::Border, "│");
        let line = |t: &str| self.palette.paint(Role::Border, t);
        let edge = |l: char, r: char, text: &str| {
            let label = fit(text, inner.saturating_sub(4));
            let fill = inner.saturating_sub(label.chars().count() + 3);
            line(&format!("{l}─ {label} {}{r}", "─".repeat(fill)))
        };
        let key_env = fields.get(3).map(|f| f.1.clone()).unwrap_or_default();
        let stored = !key_env.is_empty() && crate::config::load_keys().iter().any(|(n, _)| *n == key_env);
        let mut block: Vec<String> = vec![edge('╭', '╮', &format!("provider {name}"))];
        // Where the caret is drawn, as (row in the block, column in the box).
        let mut at_caret = (1usize, 0usize);
        for (i, (label, text)) in fields.iter().enumerate() {
            let active = i == at;
            let marker = if active { "▸" } else { " " };
            let prefix = format!("{marker} {label:<8} ");
            let row = if label == "dialect" {
                select_row(&prefix, text)
            } else if label == "api_key" {
                let dots = "•".repeat(text.chars().count().min(24));
                let status = if text.is_empty() {
                    if stored { "set (type to replace)".to_string() } else { "not set".to_string() }
                } else {
                    dots
                };
                format!("{prefix}{status}")
            } else {
                let (row, col) = text_row(&prefix, text, active.then_some(caret));
                if active {
                    at_caret = (block.len(), col);
                }
                row
            };
            block.push(format!("{wall}{}{wall}", self.boxed(&row, inner, active)));
        }
        let keys = match fields.get(at).map(|f| f.0.as_str()) {
            Some("dialect") => "↑↓ field · ←→ choose · Enter next · Esc cancel",
            Some("model") => "↑↓ field · Tab models · Enter next · Esc cancel",
            _ => "↑↓ field · ←→ caret · Enter next · Esc cancel",
        };
        block.push(edge('╰', '╯', keys));
        let height = block.len();
        let body = self.body_rows.clamp(1, rows.saturating_sub(1).max(1));
        let x = (cols.saturating_sub(width)) / 2 + 1;
        let y = (body.saturating_sub(height)) / 2 + 1;
        let mut out = String::new();
        for (i, row) in block.iter().enumerate() {
            out.push_str(&format!("\x1b[{};{}H{row}", y + i, x));
        }
        let (row, col) = at_caret;
        Some((out, x + 1 + col, y + row, Vec::new()))
    }

    fn overlay(&self, cols: usize, rows: usize) -> Option<(String, usize, usize, Vec<(usize, usize, usize, Target)>)> {
        if let Some(Pending::ProviderForm { name, fields, at }) = &self.pending {
            return self.provider_form_overlay(name, fields, *at, self.form_caret, cols, rows);
        }
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
        let mut lines = picker_lines(picker, &matches, visible, inner, true);
        // The second line costs height; a short screen keeps the one-line cut.
        if lines.len() + 4 > rows.saturating_sub(1) {
            lines = picker_lines(picker, &matches, visible, inner, false);
        }
        let height = lines.len() + 4;
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
                // The title is bold; the footer's hints are not.
                self.palette.styled(if tone == Role::Assistant { "1" } else { "" }, self.palette.paint(tone, &label)),
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

        let column = desc_column(picker, inner);
        for (text, _, selected) in &lines {
            let row = match column.filter(|_| !*selected) {
                // Descriptions are muted, except on the highlighted row.
                Some(at) => {
                    let row = self.boxed(text, inner, false);
                    let cut = row.char_indices().nth(at).map_or(row.len(), |(i, _)| i);
                    format!("{}{}", &row[..cut], self.palette.paint(Role::Thinking, &row[cut..]))
                }
                None => self.boxed(text, inner, *selected),
            };
            block.push(format!("{wall}{row}{wall}"));
        }
        block.push(edge('╰', '╯', &picker.footer(), "", Role::Thinking));

        let mut out = String::new();
        for (i, line) in block.iter().enumerate() {
            out.push_str(&format!("\x1b[{};{}H{line}", y + i, x));
        }
        let cursor = (x + 1 + filter.chars().count()).min(x + width - 2);
        // Only an open picker takes clicks; a delete confirmation keeps to keys.
        let mut targets = Vec::new();
        if matches!(self.pending, Some(Pending::Pick(_))) {
            if !matches.is_empty() {
                for (row, (_, at, _)) in lines.iter().enumerate() {
                    targets.push((y + 3 + row, x + 1, x + inner, Target::Row(*at)));
                }
            }
            let footer = fit(&picker.footer(), inner.saturating_sub(4));
            targets.extend(footer_targets(y + 3 + lines.len(), x + 3, &footer));
        }
        Some((out, cursor, y + 1, targets))
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
    /// A confirmation whose tool row shows only a label: the command,
    /// wrapped and dim, above the approval row, at most a third of the screen.
    fn labelled_command(&self, cols: usize, space: usize) -> Option<Vec<String>> {
        let Some(Pending::Confirm { what, always, .. }) = &self.pending else { return None };
        let command = what.split('`').nth(1);
        let row = self.tool.as_ref()?;
        if row.summary.contains(command.unwrap_or(what)) {
            return None;
        }
        let text = command.map_or(what.clone(), |c| format!("$ {c}"));
        let cap = (space / 3).max(1);
        let mut lines = wrap(&text, cols.saturating_sub(2));
        if lines.len() > cap {
            lines.truncate(cap);
            if let Some(last) = lines.last_mut() {
                *last = last.chars().take(cols.saturating_sub(3)).chain(['…']).collect();
            }
        }
        let mut rows: Vec<String> = lines
            .iter()
            .map(|l| self.palette.paint(Role::Thinking, &format!("  {l}")))
            .collect();
        rows.push(self.confirm_row(what, *always, cols));
        Some(rows)
    }

    /// An open question as rows: the question, one per choice, then the keys,
    /// each wrapped behind the accent bar. At most `space` rows, keys last.
    fn question_rows(&self, cols: usize, space: usize) -> Option<Vec<String>> {
        let Some(Pending::Question { q, opts, mode, typed, .. }) = &self.pending else {
            return None;
        };
        // Each part: its text, the indent its rows wrap to, and whether it is
        // a muted description, which sits wholly under its option's label.
        let mut parts = vec![(q.clone(), 2, false)];
        for (i, o) in opts.iter().enumerate() {
            let key = match (self.ask_multiple, self.ask_marked.get(i).copied().unwrap_or(false)) {
                (true, true) => "[x]".to_string(),
                _ => format!("[{}]", i + 1),
            };
            parts.push((format!("{key} {o}"), 6, false));
            if let Some(about) = self.ask_descs.get(i).filter(|d| !d.is_empty()) {
                parts.push((about.clone(), 6, true));
            }
        }
        // Queued messages wait for the answer; the hint says how many.
        let queued = match self.queued.len() {
            0 => String::new(),
            n => format!(" · {n} queued"),
        };
        parts.push(match mode {
            AskMode::Choose if self.ask_multiple => (format!("type to answer   Esc stop   1-{} mark   Enter send{queued}", opts.len()), 2, false),
            AskMode::Choose => (format!("type to answer   Esc stop{queued}"), 2, false),
            AskMode::Custom => {
                let (before, after) = typed.split_at(self.ask_caret.min(typed.len()));
                (format!("> {before}{CARET}{after}{queued}"), 4, false)
            }
        });
        let mut rows: Vec<(String, bool)> = Vec::new();
        for (text, indent, muted) in parts {
            // Continuation rows sit under the text, past the key.
            for (j, line) in wrap(&text, cols.saturating_sub(indent)).into_iter().enumerate() {
                let pad = if j == 0 && !muted { 0 } else { indent - 2 };
                rows.push((format!("{}{line}", " ".repeat(pad)), muted));
            }
        }
        // ponytail: a question taller than the screen loses its middle rows; the keys stay.
        let space = space.max(2);
        if rows.len() > space {
            let keys = rows.split_off(rows.len() - 1);
            rows.truncate(space - 1);
            rows.extend(keys);
        }
        let bell = if self.bell_pending { "\u{7}" } else { "" };
        Some(
            rows.iter()
                .enumerate()
                .map(|(i, (r, muted))| {
                    let text = fit(&format!("▌ {r}"), cols);
                    let painted = if *muted {
                        let body = text.strip_prefix("▌ ").unwrap_or(&text);
                        self.palette.paint(Role::Info, "▌ ") + &self.palette.paint(Role::Thinking, body)
                    } else {
                        self.palette.paint(Role::Info, &text)
                    };
                    let row = painted.replace(CARET, "\x1b[7m \x1b[27m");
                    if i == 0 { format!("{bell}{row}") } else { row }
                })
                .collect(),
        )
    }

    /// Queued messages, dim above the composer: three at most, then a count.
    /// The one open for editing is highlighted.
    fn queue_rows(&self, cols: usize) -> Vec<String> {
        let mut rows = Vec::new();
        for (i, text) in self.queued.iter().enumerate().take(3) {
            let first = text.lines().next().unwrap_or("");
            let row = fit(&format!("⋯ {first}"), cols);
            rows.push(if self.queue_at == Some(i) { self.palette.select(&row) } else { self.palette.paint(Role::Thinking, &row) });
        }
        if self.queued.len() > 3 {
            rows.push(self.palette.paint(Role::Thinking, &format!("  +{} more", self.queued.len() - 3)));
        }
        rows
    }

    fn composer(&self, cols: usize, space: usize) -> Vec<String> {
        if let Some(rows) = self.question_rows(cols, space.saturating_sub(2)) {
            return rows;
        }
        if let (Some(s), true, None) = (&self.status.suggestion, self.input.is_empty(), &self.pending) {
            let tone = if self.yolo() { Role::Error } else { Role::User };
            let hint = fit(s, cols.saturating_sub(self.prompt_width() + 1));
            let agent = self.prompt_agent().map_or(String::new(), |a| a.to_string());
            return vec![format!(
                "{}\x1b[7m \x1b[0m{}",
                self.palette.paint(tone, &format!("{agent}> ")),
                self.palette.paint(Role::Thinking, &hint)
            )];
        }
        if let Some(rows) = self.labelled_command(cols, space) {
            return rows;
        }
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
        // The prompt's columns, and one for a caret at the row's end.
        let lead = self.prompt_width();
        let width = cols.saturating_sub(lead + 1).max(1);
        let rows = draft_rows(text, width);
        // A caret on a wrap boundary belongs to the row it starts.
        let caret_row = rows
            .iter()
            .position(|(s, e)| cursor >= *s && cursor < *e)
            .or_else(|| rows.iter().position(|(s, e)| cursor >= *s && cursor <= *e))
            .unwrap_or(rows.len().saturating_sub(1));
        // The selection in this text's bytes, as the shell `!` shifts them.
        let off = usize::from(shell);
        let sel = self.selection().map(|(a, b)| (a.saturating_sub(off), b.saturating_sub(off)));
        let mut plain = Vec::new();
        for (i, &(from, to)) in rows.iter().enumerate() {
            let line = &text[from..to];
            // Marks at a byte: the selection's end, the caret, then its start,
            // so the caret always sits outside the selection. A selection that
            // begins or ends on a row boundary belongs to the row it enters.
            let (s, e) = match sel {
                Some((a, b)) => (
                    (a >= from && a < to).then(|| a - from),
                    (b > from && b <= to).then(|| b - from),
                ),
                None => (None, None),
            };
            let caret = (i == caret_row).then(|| cursor - from);
            let mut body = String::new();
            let marks = |body: &mut String, pos: usize| {
                if e == Some(pos) {
                    body.push(SEL_END);
                }
                if caret == Some(pos) {
                    body.push(CARET);
                }
                if s == Some(pos) {
                    body.push(SEL_START);
                }
            };
            for (j, ch) in line.char_indices() {
                marks(&mut body, j);
                body.push(ch);
            }
            marks(&mut body, line.len());
            let glyph = if shell {
                "$ "
            } else if self.status.streaming {
                "» "
            } else {
                "> "
            };
            let prefix = match (i, self.prompt_agent()) {
                (0, Some(agent)) => format!("{agent}{glyph}"),
                (0, None) => glyph.to_string(),
                _ => " ".repeat(lead),
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

    /// Paint one composer row. Rows are already cut to the width.
    fn paint_composer_row(&self, row: &str, _cols: usize) -> String {
        // The prompt, agent and glyph with their spaces; `»` is two bytes.
        let cut = row.char_indices().nth(self.prompt_width()).map_or(row.len(), |(i, _)| i);
        let (prefix, fitted) = row.split_at(cut);
        let fitted = fitted.to_string();
        // A selection that runs on from the row before, or was cut off the
        // front, starts reversed.
        let open = fitted.contains(SEL_END) && fitted.find(SEL_START).is_none_or(|s| fitted.find(SEL_END) < Some(s));
        let fitted = format!("{}{fitted}", if open { "\x1b[7m" } else { "" })
            .replace(CARET, "\x1b[7m \x1b[0m")
            .replace(SEL_START, "\x1b[7m")
            .replace(SEL_END, "\x1b[27m");
        // A selection that runs on to the next row ends with this one.
        let fitted = if fitted.matches("\x1b[7m").count() > fitted.matches("\x1b[27m").count() + fitted.matches("\x1b[0m").count() {
            fitted + "\x1b[27m"
        } else {
            fitted
        };
        let tone = if self.yolo() {
            Role::Error
        } else if prefix.contains('»') {
            // Queue mode: the glyph is muted.
            Role::Thinking
        } else {
            Role::User
        };
        self.palette.paint(tone, prefix) + &fitted
    }

    /// The agent the next message goes to, shown before the prompt glyph
    /// while there is a choice of agents.
    fn prompt_agent(&self) -> Option<&str> {
        (self.agents.len() > 1).then(|| self.agent_next.as_deref().unwrap_or(&self.agent))
    }

    /// Columns the prompt takes: the agent, then the glyph and a space.
    fn prompt_width(&self) -> usize {
        2 + self.prompt_agent().map_or(0, |a| a.chars().count())
    }

    /// Yolo is shown by colour alone: the prompt glyph and the rule.
    fn yolo(&self) -> bool {
        self.cfg.live_permissions() == crate::config::Permissions::Yolo
    }

    /// Attach an image from the system clipboard, if one is there. Says once
    /// per run when there is no clipboard to read.
    fn paste_image(&mut self, tx: &Sender<Cmd>, dir: &std::path::Path) -> bool {
        use crate::images::Clip;
        let clip = self.paste_cmd.as_deref().map_or(Clip::Unsupported, crate::images::read_clipboard);
        match clip {
            Clip::Image(bytes) => {
                match crate::images::store(bytes, "pasted.png", dir) {
                    Ok(image) => {
                        self.push(Role::Thinking, crate::images::marker(&image));
                        let _ = tx.send(Cmd::Attach(vec![image]));
                    }
                    Err(e) => self.push(Role::Error, format!("pasted image: {e}")),
                }
                self.follow_tail();
                true
            }
            Clip::Unsupported => {
                if !std::mem::replace(&mut self.paste_noted, true) {
                    self.push(Role::Info, "Ctrl+V cannot paste images here: no clipboard tool, or over SSH");
                }
                false
            }
            Clip::Nothing => false,
        }
    }

    /// The selected bytes of the draft, start first, when any are selected.
    fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?.min(self.input.len());
        let (from, to) = (anchor.min(self.cursor), anchor.max(self.cursor));
        (from < to && self.input.is_char_boundary(from) && self.input.is_char_boundary(to)).then_some((from, to))
    }

    /// Move the caret within the draft. Up and Down change rows only; the
    /// plain keys recall history themselves.
    fn move_caret(&mut self, m: Move) {
        let rows = self.draft_rows_now();
        let (row, col) = self.drawn_caret(&rows);
        self.cursor = match m {
            Move::Left => previous_boundary(&self.input, self.cursor),
            Move::Right => next_boundary(&self.input, self.cursor),
            Move::Home => 0,
            Move::End => self.input.len(),
            Move::Up if row > 0 => self.drawn_at(&rows, row - 1, col, true),
            Move::Down if row + 1 < rows.len() => self.drawn_at(&rows, row + 1, col, false),
            Move::Up | Move::Down => self.cursor,
        };
        self.dirty = true;
    }

    /// The draft without the shell `!`, which is drawn as the prompt.
    fn draft_text(&self) -> &str {
        if self.input.starts_with('!') {
            &self.input[1..]
        } else {
            &self.input
        }
    }

    /// The draft's drawn rows at the terminal's width.
    fn draft_rows_now(&self) -> Vec<(usize, usize)> {
        let cols = self.cols.load(Ordering::Relaxed) as usize;
        let lead = self.prompt_width() + 1;
        let width = if cols >= lead + 2 { cols - lead } else { 77 };
        draft_rows(self.draft_text(), width)
    }

    /// The caret's drawn row and its column in characters.
    fn drawn_caret(&self, rows: &[(usize, usize)]) -> (usize, usize) {
        let text = self.draft_text();
        let off = usize::from(self.input.starts_with('!'));
        let probe = self.cursor.saturating_sub(off).min(text.len());
        let row = rows
            .iter()
            .position(|(s, e)| probe >= *s && probe < *e)
            .or_else(|| rows.iter().position(|(s, e)| probe >= *s && probe <= *e))
            .unwrap_or(rows.len().saturating_sub(1));
        let (from, _) = rows[row];
        (row, text[from..probe].chars().count())
    }

    /// The byte offset on a drawn row at a character column. Moving up onto a
    /// shorter row sits on its last character, so the caret stays on that row.
    fn drawn_at(&self, rows: &[(usize, usize)], row: usize, col: usize, back: bool) -> usize {
        let Some(&(from, to)) = rows.get(row) else {
            return self.input.len();
        };
        let text = self.draft_text();
        // The end of a wrapped row is the next row's start, so moving up onto
        // it sits on the last character; the end of a newline row is its own.
        let wrapped = to < text.len() && text.as_bytes()[to] != b'\n';
        let at = match text[from..to].char_indices().nth(col).map(|(i, _)| from + i) {
            Some(at) => at,
            None if back && wrapped => text[from..to]
                .char_indices()
                .last()
                .map(|(i, _)| from + i)
                .unwrap_or(from),
            None => to,
        };
        at + usize::from(self.input.starts_with('!'))
    }

    /// The status line, muted, with the unseen and waiting counts in accent.
    fn paint_status(&self, cols: usize) -> String {
        let status = self.status_line(cols);
        let mut painted = self.palette.paint(Role::Thinking, &status);
        // What `paint` put ahead of the text: the muted colour, to resume after a count.
        let muted = painted.strip_suffix(&format!("{status}\x1b[0m")).unwrap_or("").to_string();
        let counts = [(self.unseen, format!("↑ {}", self.unseen)), (self.waiting(), format!("{} waiting", self.waiting()))];
        for (n, count) in counts {
            if self.palette.on && n > 0 {
                painted = painted.replacen(&count, &format!("{}{muted}", self.palette.paint(Role::Info, &count)), 1);
            }
        }
        painted
    }

    fn status_line(&self, cols: usize) -> String {
        if self.released {
            return fit(" mouse released · any key to take it back", cols);
        }
        if let (Some((secs, attempt, of)), true) = (self.status.retry, self.status.streaming) {
            return fit(&format!(" retrying in {secs}s · {attempt}/{of} · Esc stop"), cols);
        }
        if let Some((text, _)) = self.status.toast.as_ref().filter(|(_, at)| at.elapsed() < TOAST) {
            return fit(&format!(" {text}"), cols);
        }
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
            .map(|c| if c > 0.0 && c < 0.1 { format!(" · ${c:.3}") } else { format!(" · ${c:.2}") })
            .unwrap_or_default()
        } else {
            String::new()
        };
        let model = if self.status.served.is_empty() || self.status.served == self.cfg.model {
            self.cfg.model.clone()
        } else {
            format!("{} ({})", self.cfg.model, self.status.served)
        };
        // A configured provider is named after its model, so the session's source shows.
        let model = if self.cfg.named.iter().any(|n| n.name == self.cfg.active_name) {
            format!("{model} ({})", self.cfg.active_name)
        } else {
            model
        };
        // With several live sessions, which one this is comes first.
        // Which session this is lives in the title bar.
        let place = "";
        let waiting = match self.waiting() {
            0 => String::new(),
            n => format!(" · {n} waiting"),
        };
        let paused = if self.queue_at.is_some() && self.status.streaming { " · paused · editing queue" } else { "" };
        let goal = match &self.status.goal {
            Some((_, round, rounds)) if self.status.goal_paused => format!(" · goal {round}/{rounds} · paused"),
            Some((_, round, rounds)) => format!(" · goal {round}/{rounds}"),
            None => String::new(),
        };
        // The agent is named before the prompt glyph instead.
        let agent = "";
        let mode = match self.cfg.live_permissions() {
            crate::config::Permissions::Normal => String::new(),
            other => format!("{} · ", other.as_str()),
        };
        // The share of the window in use, shown only when the window is known.
        let window = crate::context::window(&self.cfg.model, self.cfg.context_window);
        let percent = crate::context::percent(self.status.tokens, window)
            .map(|p| format!(" ({p}%)"))
            .unwrap_or_default();
        let left = format!(
            "{place}{model} · {agent}{mode}{} tok{percent}{money}{goal}{paused}{unseen}{thinking}{waiting}",
            compact_tokens(self.status.tokens),
        );
        // The directory sits at the right edge, and goes first when narrow.
        // A running task's meter takes its place.
        let right = match self.status.task_start.filter(|_| self.status.streaming) {
            Some(start) => {
                let s = &self.status;
                let out = s.task_usage.2.max(s.chars as u64 / 4);
                format!("{} · {} · ↓{}", span(start.elapsed()), steps(s.steps), compact_tokens(out))
            }
            // The directory shortens to what is left after two spaces.
            None => fit_path(&tilde(&self.root), cols.saturating_sub(left.chars().count() + 2)).unwrap_or_default(),
        };
        let gap = cols.saturating_sub(left.chars().count() + right.chars().count());
        if gap >= 2 && !right.is_empty() {
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
        .map(|e| crate::store::cut_title(&e.text))
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

/// Where the draft's selection starts and ends, until the painter swaps
/// them for reverse video.
const SEL_START: char = '\u{e000}';
const SEL_END: char = '\u{e001}';

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

/// The session directory in at most `room` columns, shortened in steps: the
/// whole path, parents cut to their first character (a dot folder to two),
/// the parents as `…`, then the last folder cut from the front when 8 or more
/// columns are left. `None` when even that does not fit.
fn fit_path(path: &str, room: usize) -> Option<String> {
    let width = |s: &str| s.chars().count();
    if width(path) <= room {
        return Some(path.to_string());
    }
    let (lead, rest) = match path.strip_prefix("~/") {
        Some(rest) => ("~/", rest),
        None => path.strip_prefix('/').map_or(("", path), |rest| ("/", rest)),
    };
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    let last = *parts.last()?;
    if parts.len() > 1 {
        // The two folders nearest the session stay whole; earlier ones give way.
        let keep = parts.len().saturating_sub(2);
        let parents: Vec<String> = parts[..keep]
            .iter()
            .map(|p| p.chars().take(if p.starts_with('.') { 2 } else { 1 }).collect())
            .collect();
        let kept = parts[keep..].join("/");
        let short = if parents.is_empty() {
            format!("{lead}{kept}")
        } else {
            format!("{lead}{}/{kept}", parents.join("/"))
        };
        for short in [short, format!("…/{last}")] {
            if width(&short) <= room {
                return Some(short);
            }
        }
    }
    (room >= 8).then(|| tail(last, room))
}

/// The draft as drawn rows: byte ranges in `text`, broken at the last space
/// that fits in `width` columns, or mid-word when no space does.
fn draft_rows(text: &str, width: usize) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut base = 0;
    for line in text.split('\n') {
        let chars: Vec<(usize, char)> = line.char_indices().collect();
        let mut start = 0;
        while chars.len() - start > width {
            // A space stays on the row it ends, so no character is hidden.
            let cut = chars[start..start + width]
                .iter()
                .rposition(|(_, c)| *c == ' ')
                .map(|i| start + i + 1)
                .unwrap_or(start + width);
            let from = chars[start].0;
            let to = chars[cut].0;
            rows.push((base + from, base + to));
            start = cut;
        }
        let from = chars.get(start).map(|(i, _)| *i).unwrap_or(line.len());
        rows.push((base + from, base + line.len()));
        base += line.len() + 1;
    }
    rows
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
/// `1 step`, `3 steps`.
fn steps(n: usize) -> String {
    if n == 1 { "1 step".into() } else { format!("{n} steps") }
}

/// A finished task in one line: `42s · 5 steps · 18K in (92% cached) · 1.4K out · $0.004`.
/// Parts that are unknown are left out.
pub fn task_line(took: Duration, step_count: usize, hit: u64, miss: u64, out: u64, cost: Option<f64>) -> String {
    let mut parts = vec![span(took)];
    if step_count > 0 {
        parts.push(steps(step_count));
    }
    let input = hit + miss;
    if input > 0 {
        parts.push(format!("{} in ({}% cached)", compact_tokens(input), hit * 100 / input));
    }
    if out > 0 {
        parts.push(format!("{} out", compact_tokens(out)));
    }
    if let Some(c) = cost.filter(|c| *c > 0.0) {
        parts.push(if c < 0.1 { format!("${c:.3}") } else { format!("${c:.2}") });
    }
    parts.join(" · ")
}

/// A duration as `4s` or `1m12s`.
fn span(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 { format!("{s}s") } else { format!("{}m{:02}s", s / 60, s % 60) }
}

fn verb(what: &str) -> &'static str {
    match what.split_whitespace().next().unwrap_or("") {
        "run" => "run it?",
        "write" => "write it?",
        "save" => "save it?",
        "delete" => "delete it?",
        _ => "approve?",
    }
}

/// The byte where the word after (`forward`) or before the caret at `i`
/// ends: spaces and punctuation are skipped first, as readline does. A word
/// is letters, digits, and `_`.
/// One edit to a line of text at `caret`: typing, deleting by character,
/// word, or row, caret moves, and pasting `clip` on Ctrl+V. The composer and
/// the answer row share it. Returns false for a key it does not edit with.
fn edit_line(text: &mut String, caret: &mut usize, key: &Key, clip: &str) -> bool {
    let c = (*caret).min(text.len());
    let start = text[..c].rfind('\n').map_or(0, |i| i + 1);
    let end = text[c..].find('\n').map_or(text.len(), |i| c + i);
    let (cut, at) = match key {
        Key::Char(ch) => {
            text.insert(c, *ch);
            (None, c + ch.len_utf8())
        }
        Key::Ctrl('v') => {
            text.insert_str(c, clip);
            (None, c + clip.len())
        }
        Key::Backspace => (Some(previous_boundary(text, c)..c), previous_boundary(text, c)),
        Key::Delete => (Some(c..next_boundary(text, c)), c),
        Key::Left => (None, previous_boundary(text, c)),
        Key::Right => (None, next_boundary(text, c)),
        Key::Home | Key::Ctrl('a') => (None, start),
        Key::End => (None, end),
        Key::Word(m) => (None, word_edge(text, c, *m == Move::Right)),
        Key::DeleteWord(true) => (Some(c..word_edge(text, c, true)), c),
        Key::DeleteWord(false) | Key::Ctrl('w') => (Some(word_edge(text, c, false)..c), word_edge(text, c, false)),
        Key::Ctrl('u') => (Some(start..c), start),
        Key::Ctrl('k') => (Some(c..end), c),
        _ => return false,
    };
    if let Some(range) = cut {
        text.replace_range(range, "");
    }
    *caret = at;
    true
}

/// The local time as `HH:MM`.
fn clock() -> String {
    // ponytail: the leading fields of `struct tm` match on macOS and glibc;
    // the rest is padding we never read.
    #[repr(C)]
    struct Tm {
        sec: i32,
        min: i32,
        hour: i32,
        rest: [i64; 8],
    }
    extern "C" {
        fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
        fn tzset();
    }
    // `localtime_r` need not read `TZ` itself; `tzset` makes it.
    static ZONE: std::sync::Once = std::sync::Once::new();
    // SAFETY: no arguments; sets libc's zone globals once.
    ZONE.call_once(|| unsafe { tzset() });
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let mut tm = Tm { sec: 0, min: 0, hour: 0, rest: [0; 8] };
    // SAFETY: both pointers are valid for the call; `tm` is large enough.
    if unsafe { localtime_r(&now, &mut tm) }.is_null() {
        return String::new();
    }
    format!("{:02}:{:02}", tm.hour, tm.min)
}

/// A drawn row without its escape sequences.
fn plain(row: &str) -> String {
    let mut out = String::new();
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // CSI runs to a final byte; OSC to BEL or ESC backslash.
            Some('[') => while chars.next().is_some_and(|c| !('@'..='~').contains(&c)) {},
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' || (c == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// The cells a drag covers on each row it touches, in text order: from the
/// press to the pointer. `ponytail:` one column per character.
fn drag_spans(a: (u16, u16), b: (u16, u16)) -> Vec<(usize, usize, usize)> {
    let ((x0, y0), (x1, y1)) = if (a.1, a.0) <= (b.1, b.0) { (a, b) } else { (b, a) };
    (y0..=y1)
        .map(|y| {
            let from = if y == y0 { x0 as usize - 1 } else { 0 };
            let to = if y == y1 { x1 as usize } else { usize::MAX };
            (y as usize - 1, from, to)
        })
        .collect()
}

/// The text a drag covers, one line per row, trailing spaces dropped.
fn selected_text(rows: &[String], a: (u16, u16), b: (u16, u16)) -> String {
    drag_spans(a, b)
        .into_iter()
        .filter_map(|(row, from, to)| {
            let text: Vec<char> = plain(rows.get(row)?).chars().collect();
            let to = to.min(text.len());
            Some(text.get(from.min(to)..to)?.iter().collect::<String>().trim_end().to_string())
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Draw a live drag reversed. Rows it covers lose their colour meanwhile.
fn highlight(screen: &mut [String], drag: Option<(u16, u16, u16, u16)>) {
    let Some((ax, ay, x, y)) = drag else { return };
    if (ax, ay) == (x, y) {
        return;
    }
    for (row, from, to) in drag_spans((ax, ay), (x, y)) {
        let Some(line) = screen.get_mut(row) else { continue };
        let text: Vec<char> = plain(line).chars().collect();
        let to = to.min(text.len());
        let from = from.min(to);
        let part = |r: std::ops::Range<usize>| text[r].iter().collect::<String>();
        *line = format!("{}\x1b[7m{}\x1b[27m{}", part(0..from), part(from..to), part(to..text.len()));
    }
}

fn word_edge(s: &str, i: usize, forward: bool) -> usize {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    if forward {
        let rest = &s[i..];
        let start = rest.char_indices().find(|(_, c)| word(*c)).map_or(rest.len(), |(k, _)| k);
        let end = rest[start..].char_indices().find(|(_, c)| !word(*c)).map_or(rest.len() - start, |(k, _)| k);
        i + start + end
    } else {
        let before: Vec<(usize, char)> = s[..i].char_indices().collect();
        let mut k = before.len();
        while k > 0 && !word(before[k - 1].1) {
            k -= 1;
        }
        while k > 0 && word(before[k - 1].1) {
            k -= 1;
        }
        before.get(k).map_or(i, |(at, _)| *at)
    }
}

/// The byte after the character at `i`, or the end.
fn next_boundary(s: &str, i: usize) -> usize {
    s[i.min(s.len())..].chars().next().map_or(s.len(), |c| i + c.len_utf8())
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
    // Tests neither read nor write the user's real history.
    if cfg!(test) {
        return Vec::new();
    }
    std::fs::read_to_string(history_path())
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn save_history(history: &[String]) {
    if cfg!(test) {
        return;
    }
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
fn replay(history: &[crate::api::Message], root: &std::path::Path) -> Vec<(Role, String)> {
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
                    let summary = crate::agent::relative(crate::agent::describe(&call.name, &args), root);
                    rows.push((Role::Tool, format!("▸ {summary}")));
                }
            }
            _ => {}
        }
    }
    rows
}

/// Draw a restored session and its context size.
fn send_replay(tx: &Sender<Ev>, history: &[crate::api::Message], root: &std::path::Path) {
    let _ = tx.send(Ev::Replay(replay(history, root)));
    let _ = tx.send(Ev::Usage {
        tokens: Some(crate::api::estimate_tokens(history)),
        hit: 0,
        miss: 0,
        output: 0,
        model: None,
    });
}

/// The context estimate at a turn's end. Tokens were counted per request.
fn context_usage(session: &Session) -> Ev {
    Ev::Usage {
        tokens: Some(crate::api::estimate_tokens(&session.history)),
        hit: 0,
        miss: 0,
        output: 0,
        model: session.last_model.clone(),
    }
}

/// Run the interface until the user quits. Returns the final session id.
/// A worker thread for one session: its command sender, its event receiver,
/// and the id of the session it is logging to.
type Worker = (Sender<Cmd>, Receiver<Ev>, std::sync::Arc<std::sync::Mutex<String>>);

/// One line naming what woke the session: a check-in or a job's end.
fn wake_summary(note: &str) -> String {
    let jobs: Vec<String> = note
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|w| w[0].eq_ignore_ascii_case("job") && w[1].trim_end_matches(['.', ':']).parse::<u64>().is_ok())
        .map(|w| format!("job {}", w[1].trim_end_matches(['.', ':'])))
        .fold(Vec::new(), |mut acc, j| {
            if !acc.contains(&j) {
                acc.push(j);
            }
            acc
        });
    let kind = match (note.contains("ended with exit"), note.contains("check-in you asked for")) {
        (true, true) => "job ended, check-in",
        (true, false) => "job ended",
        _ => "check-in",
    };
    if jobs.is_empty() { kind.to_string() } else { format!("{kind}: {}", jobs.join(", ")) }
}

fn spawn_worker(session: Session, sessions_dir: std::path::PathBuf) -> Worker {
    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (ev_tx, ev_rx) = channel::<Ev>();
    let mut worker_session = session;
    // This worker can wake the session for check-ins.
    worker_session.wakes = true;
    if worker_session
        .history
        .iter()
        .any(|m| m.role != crate::api::Role::System)
    {
        send_replay(&ev_tx, &worker_session.history, &worker_session.tools.root);
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
        move || {
        // Commands that arrived during a goal round, run after it.
        let mut queued: std::collections::VecDeque<Cmd> = Default::default();
        loop {
            // A second's wait, so a due check-in can start a turn while idle.
            let next = if let Some(cmd) = queued.pop_front() { Ok(cmd) } else { match cmd_rx.recv_timeout(Duration::from_secs(1)) {
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(note) = worker_session.due() {
                        let what = wake_summary(&note);
                        let _ = ev_tx.send(Ev::Woke(what));
                        let mut hooks = WorkerHooks {
                            tx: ev_tx.clone(),
                            waited: Duration::ZERO,
                        };
                        let result = worker_session.turn(&note, &mut hooks);
                        let _ = ev_tx.send(context_usage(&worker_session));
                        let _ = ev_tx.send(Ev::Done(result.err()));
                    }
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(std::sync::mpsc::RecvError),
                Ok(cmd) => Ok(cmd),
            } };
            match next {
                // An empty text resumes the saved goal; with no goal there is nothing to resume.
                Ok(Cmd::Goal(text)) if text.is_empty() && worker_session.goal.is_none() => {
                    let _ = ev_tx.send(Ev::Info("no goal to resume".into()));
                }
                Ok(Cmd::Goal(text)) => {
                    let mut message = if text.is_empty() {
                        worker_session.resume_goal().unwrap_or_default()
                    } else {
                        worker_session.start_goal(&text)
                    };
                    loop {
                        let Some(goal) = worker_session.goal.clone() else { break };
                        let _ = ev_tx.send(Ev::GoalRound { text: goal.text, round: goal.round, rounds: goal.rounds });
                        let mut hooks = WorkerHooks { tx: ev_tx.clone(), waited: Duration::ZERO };
                        let result = worker_session.turn(&message, &mut hooks);
                        let _ = ev_tx.send(context_usage(&worker_session));
                        let failed = result.is_err() || worker_session.cancel.load(Ordering::Relaxed);
                        let _ = ev_tx.send(Ev::Done(result.err()));
                        // What the user did during the round decides whether another starts.
                        let mut stop = failed.then(|| "goal stopped".to_string());
                        let mut paused_now = false;
                        while let Ok(cmd) = cmd_rx.try_recv() {
                            match cmd {
                                // Paused after this round: the goal is kept.
                                Cmd::GoalPause => paused_now = true,
                                Cmd::GoalOff => {
                                    stop.get_or_insert_with(|| "goal cleared".into());
                                }
                                Cmd::Prompt(_) | Cmd::Quit | Cmd::Goal(_) => {
                                    stop.get_or_insert_with(|| "goal stopped: you took over".into());
                                    queued.push_back(cmd);
                                }
                                other => queued.push_back(other),
                            }
                        }
                        if let Some(why) = stop {
                            worker_session.goal = None;
                            let _ = ev_tx.send(Ev::GoalEnd(why));
                            break;
                        }
                        if paused_now {
                            worker_session.pause_goal();
                            let _ = ev_tx.send(Ev::GoalPaused);
                            break;
                        }
                        match worker_session.goal_next() {
                            Ok(next) => message = next,
                            Err(why) => {
                                let _ = ev_tx.send(Ev::GoalEnd(why));
                                break;
                            }
                        }
                    }
                }
                Ok(Cmd::GoalPause) => {
                    if worker_session.goal.is_some() {
                        worker_session.pause_goal();
                        let _ = ev_tx.send(Ev::GoalPaused);
                    } else {
                        let _ = ev_tx.send(Ev::Info("no goal to pause".into()));
                    }
                }
                Ok(Cmd::GoalEdit(text)) => match worker_session.edit_goal(&text) {
                    Ok(()) => {
                        let _ = ev_tx.send(Ev::Info("goal edited; the round stays".into()));
                    }
                    Err(why) => {
                        let _ = ev_tx.send(Ev::Info(why));
                    }
                },
                Ok(Cmd::GoalOff) => {
                    if worker_session.goal.is_some() {
                        worker_session.clear_goal();
                        let _ = ev_tx.send(Ev::GoalEnd("goal cleared".into()));
                    }
                }
                Ok(Cmd::Attach(images)) => {
                    for image in images {
                        if let Err(e) = worker_session.attach_image(image) {
                            let _ = ev_tx.send(Ev::Error(e));
                        }
                    }
                }
                Ok(Cmd::Diff(args)) => {
                    let ev = match args.strip_prefix("git") {
                        Some(rest) if rest.is_empty() || rest.starts_with(' ') => match worker_session.git_changes(rest) {
                            Ok(files) => Ev::Diff(files, true),
                            Err(why) => Ev::Error(why),
                        },
                        _ => Ev::Diff(worker_session.session_changes(), false),
                    };
                    let _ = ev_tx.send(ev);
                }
                Ok(Cmd::Detach) => {
                    if let Some(image) = worker_session.attachments.pop() {
                        let _ = ev_tx.send(Ev::Info(format!("removed {}", crate::images::marker(&image))));
                    }
                }
                Ok(cmd @ (Cmd::Prompt(_) | Cmd::Retry(_))) => {
                    let text = match cmd {
                        Cmd::Retry(text) if !worker_session.retry(&text) => {
                            let _ = ev_tx.send(Ev::Info("nothing to retry".into()));
                            let _ = ev_tx.send(Ev::Done(None));
                            continue;
                        }
                        Cmd::Prompt(text) | Cmd::Retry(text) => text,
                        _ => unreachable!(),
                    };
                    let mut hooks = WorkerHooks {
                        tx: ev_tx.clone(),
                        waited: Duration::ZERO,
                    };
                    let result = worker_session.turn(&text, &mut hooks);
                    let _ = ev_tx.send(context_usage(&worker_session));
                    let ok = result.is_ok();
                    let _ = ev_tx.send(Ev::Done(result.err()));
                    // After Done, so the chat never waits for it.
                    let cancelled = worker_session.cancel.load(Ordering::Relaxed);
                    if ok && !cancelled && worker_session.wants_suggestion() {
                        if let Some((text, (hit, miss, output))) = worker_session.suggest() {
                            let _ = ev_tx.send(Ev::Usage { tokens: None, hit, miss, output, model: None });
                            let _ = ev_tx.send(Ev::Suggest(text));
                        }
                    }
                    // A summary title after the first turn, but never while the user
                    // has sent something: that message is kept and runs first.
                    if ok && !cancelled {
                        match cmd_rx.try_recv() {
                            Ok(cmd) => queued.push_back(cmd),
                            Err(_) if queued.is_empty() => {
                                if let Some((title, (hit, miss, output))) = worker_session.title_summary() {
                                    let _ = ev_tx.send(Ev::Usage { tokens: None, hit, miss, output, model: None });
                                    if let Some(log) = &worker_session.log {
                                        let _ = log.retitle(&title);
                                    }
                                    let _ = ev_tx.send(Ev::Title(title));
                                }
                            }
                            Err(_) => {}
                        }
                    }
                }
                Ok(Cmd::SetThinking(on)) => worker_session.cfg.thinking = on,
                Ok(Cmd::SetEffort(level)) => worker_session.cfg.effort = level,
                Ok(Cmd::SetAgent(name)) => {
                    let name = crate::store::resolve_agent(&name).to_string();
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
                        worker_session.switch_agent(agent);
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
                            send_replay(&ev_tx, &worker_session.history, &worker_session.tools.root);
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
                Ok(Cmd::SetConfig(key, value)) => {
                    let _ = worker_session.cfg.set(&key, &value);
                }
                Ok(Cmd::SetPermissions(mode)) => {
                    worker_session.set_permissions(mode);
                }
                Ok(Cmd::RevokeAllow { shape, always }) => {
                    if let Err(e) = worker_session.revoke_allow(&shape, always) {
                        let _ = ev_tx.send(Ev::Error(e));
                    }
                }
                Ok(Cmd::AllowAlways(shape)) => match worker_session.cfg.allow_always(&shape) {
                    Ok(()) => {
                        let _ = ev_tx.send(Ev::Allow { shape: shape.clone(), always: true });
                        let _ = ev_tx.send(Ev::Info(format!("allow = {shape}")));
                    }
                    Err(e) => {
                        let _ = ev_tx.send(Ev::Error(e));
                    }
                },
                Ok(Cmd::Reload) => {
                    // The config first: only keys the file changed apply.
                    let path = worker_session.cfg.file.clone().unwrap_or_else(Config::config_path);
                    match Config::load(&path) {
                        Ok(new) => {
                            let (changed, removed) = worker_session.cfg.reload_from(&new);
                            crate::render::set_renderers(&worker_session.cfg.render);
                            for line in changed {
                                let _ = ev_tx.send(Ev::Info(line));
                            }
                            if !removed.is_empty() {
                                let _ = ev_tx.send(Ev::Info(format!("restart to reset {}", removed.join(", "))));
                            }
                            if let Some(note) = crate::config::unknown_note(&new.unknown) {
                                let _ = ev_tx.send(Ev::Info(note));
                            }
                            let _ = ev_tx.send(Ev::Config(Box::new(new)));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(format!("config not reloaded: {e}")));
                        }
                    }
                    match worker_session.reload() {
                        Ok(()) => {
                            let _ = ev_tx.send(Ev::Info(
                                "reloaded the config, instructions and skills; prompt cache reset".to_string(),
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
                Ok(Cmd::Edit(argv)) => {
                    let message = match attach_argv(&argv) {
                        Some(0) | None => "editor closed".to_string(),
                        Some(code) => format!("editor exited {code}"),
                    };
                    let _ = ev_tx.send(Ev::Info(message));
                }
                Ok(Cmd::Open(argv)) => {
                    let what = argv.last().cloned().unwrap_or_default();
                    let result = match argv.split_first() {
                        Some((program, args)) => std::process::Command::new(program).args(args).spawn().map(|_| ()),
                        None => Err(std::io::Error::other("no opener")),
                    };
                    match result {
                        Ok(()) => {
                            let _ = ev_tx.send(Ev::Info(format!("opened {what}")));
                        }
                        Err(e) => {
                            let _ = ev_tx.send(Ev::Error(e.to_string()));
                        }
                    }
                }
                Ok(Cmd::Shell) => {
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
                    let mut hooks = WorkerHooks {
                        tx: ev_tx.clone(),
                        waited: Duration::ZERO,
                    };
                    // Start in the session directory, not where imp was launched.
                    let code = hooks.run_attached(&crate::agent::in_dir(&worker_session.tools.root, &format!("exec {shell}")));
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
                            .resolve_key_env(crate::config::key_lookup())
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
        }
    });
    (cmd_tx, ev_rx, session_id)
}

pub fn run(session: Session, cfg: Config, fresh: Fresh, missing_key: Option<String>) -> Result<Option<String>, String> {
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
    let mut session = session;
    session.steer = app.steer.clone();
    session.hold = app.hold.clone();
    app.worker = Some(spawn_worker(session, sessions_dir));
    app.fresh = Some(fresh);
    if let Some(note) = crate::config::unknown_note(&app.cfg.unknown) {
        app.push(Role::Info, note);
    }
    if let Some(var) = missing_key {
        app.begin_onboard(var);
    }

    // Key reader.
    let (key_tx, key_rx) = channel::<Vec<u8>>();
    let key_tx_resize = key_tx.clone();
    std::thread::spawn(move || read_keys(0, key_tx, &ATTACHED, &PARKED));

    wake_on_resize(key_tx_resize);

    let mut pending_bytes: Vec<u8> = Vec::new();
    let mut busy = Instant::now();

    app.render();
    // A command held the terminal: every row is repainted when it returns.
    let mut away = false;
    while !app.quit {
        let mut redraw = false;
        // While a command holds the terminal, wake often to see it return.
        let wait = if ATTACHED.load(Ordering::SeqCst) { Duration::from_millis(50) } else { app.loop_wait(busy.elapsed()) };
        match key_rx.recv_timeout(wait) {
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
                if app.expire_toast() {
                    redraw = true;
                }
                if app.clock_due() {
                    redraw = true;
                }
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

        // A resize has to be noticed without repainting on a timer.
        let (cols, rows) = size();
        if app.resized(cols, rows) {
            app.dirty = true;
        }

        // Nothing is drawn over a command that holds the terminal.
        if ATTACHED.load(Ordering::SeqCst) {
            away = true;
            continue;
        }
        if std::mem::take(&mut away) {
            app.shown_size = (0, 0);
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
mod tests;
