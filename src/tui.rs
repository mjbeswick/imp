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
const RESTORE: &str = "\x1b[?1006l\x1b[?1000l\x1b[<u\x1b[?25h\x1b[?1049l";

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
    Ctrl(char),
    WheelUp,
    WheelDown,
    /// A left-button press at a 1-based column and row.
    Click(u16, u16),
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
                return Some((i + 1, mouse(sgr, b == b'M')));
            }
            // `1;2` is Shift; other modifiers keep the plain key.
            let shift = params == b"1;2";
            let key = match b {
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
    /// Drop the last unsent attachment.
    Detach,
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
    /// Work across turns until the goal is met.
    Goal(String),
    GoalOff,
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
        reply: Sender<Option<String>>,
    },
    Stop(String),
    Done(Option<String>),
    /// The user's likely next message, drawn greyed out in an empty input.
    Suggest(String),
    /// A goal round begins.
    GoalRound { text: String, round: u32, rounds: u32 },
    /// The goal ended, and why.
    GoalEnd(String),
}

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
            Ui::Show(t) => Ev::Show(t),
            Ui::Usage { hit, miss, output, model } => Ev::Usage { tokens: None, hit, miss, output, model },
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
        // Stop the key reader before the command takes the terminal.
        ATTACHED.store(true, std::sync::atomic::Ordering::SeqCst);
        let asked = Instant::now();
        while !PARKED.load(std::sync::atomic::Ordering::SeqCst) && asked.elapsed() < Duration::from_millis(500) {
            std::thread::sleep(Duration::from_millis(5));
        }
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
        ATTACHED.store(false, std::sync::atomic::Ordering::SeqCst);
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
fn spin_rows(items: &mut [PickItem], working: &[String], frame: char) {
    for item in items {
        let count = item.desc.trim_start_matches(|c| spinner_frames().contains(c) || c == ' ');
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
    /// Enter during a turn queues rather than steers.
    queue_mode: bool,
    /// Messages waiting for the running turn to end, oldest first.
    queued: Vec<String>,
    /// The queued message open in the composer, while one is.
    queue_at: Option<usize>,
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
    spinner: usize,
    /// Rendered rows per entry, keyed by a hash of what shapes them.
    drawn: Vec<(u64, Vec<(Role, Line)>)>,
    /// What a click on the last frame lands on: row, first and last column
    /// (1-based, inclusive), and the target.
    targets: Vec<(usize, usize, usize, Target)>,
    /// `[a] always` was clicked once; a second click on it applies.
    armed: bool,
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
    /// Enter during a turn queues rather than steers.
    queue_mode: bool,
    /// Messages waiting for the running turn to end, oldest first.
    queued: Vec<String>,
    /// The queued message open in the composer, while one is.
    queue_at: Option<usize>,
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
}

impl Pane {
    fn new(session: &Session, worker: Option<Worker>) -> Self {
        let (session_steer, session_hold) = (session.steer.clone(), session.hold.clone());
        Self {
            cfg: session.cfg.clone(),
            cancel: session.cancel.clone(),
            entries: Vec::new(),
            input: String::new(),
            cursor: 0,
            anchor: None,
            queue_mode: false,
            queued: Vec::new(),
            queue_at: None,
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
    if picker.kind != PickKind::Command {
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
    vec![format!("{marker} {label:<left$}  {desc}")]
}

/// The column a picker's descriptions start at, or `None` when it has none.
fn desc_column(picker: &Picker, inner: usize) -> Option<usize> {
    if !picker.items.iter().any(|item| !item.desc.is_empty()) {
        return None;
    }
    let mut left = picker.items.iter().map(|item| item.label.chars().count()).max().unwrap_or(0).max(10) + 2;
    if picker.kind != PickKind::Command {
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
        for line in picker_row(picker, at, index, inner, wrap && selected) {
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

/// Commands whose argument is typed rather than picked.
const FREE_TEXT: &[&str] = &["queue", "rename"];

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
        SPIN.store(cfg.spinner, Ordering::Relaxed);
        let history = load_history();
        let agents = crate::store::load_agents(&cfg, &root)
            .into_iter()
            .map(|a| (a.name, a.description))
            .collect();
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
            queue_mode: false,
            queued: Vec::new(),
            queue_at: None,
            queue_due: false,
            steer: session_steer.clone(),
            hold: session_hold.clone(),
            history,
            hist_pos: None,
            follow: true,
            top: 0,
            body_rows: 1,
            max_top: 0,
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
            spinner: 0,
            drawn: Vec::new(),
            targets: Vec::new(),
            armed: false,
            onboard: false,
            clip: String::new(),
            // Tests never read the real clipboard.
            paste_cmd: if cfg!(test) { None } else { crate::images::clipboard_command() },
            paste_noted: false,
            osc52: None,
            released: false,
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
        swap(&mut self.queue_mode, &mut p.queue_mode);
        swap(&mut self.queued, &mut p.queued);
        swap(&mut self.queue_at, &mut p.queue_at);
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

    /// One diff row, indented under its tool row like a failure's reason.
    fn push_diff(&mut self, line: String) {
        let role = match line.chars().next() {
            Some('+') => Role::Add,
            Some('-') => Role::Delete,
            _ => Role::Context,
        };
        self.push(role, format!("  {line}"));
    }

    /// Start another live session, or reset this one without a factory.
    fn new_session(&mut self, tx: &Sender<Cmd>) {
        let Some(fresh) = &self.fresh else {
            let _ = tx.send(Cmd::Reset);
            self.clear_entries();
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
        self.clear_entries();
        self.push(Role::Info, "new conversation");
    }

    /// Resume a stored session in another live pane, or in this one.
    fn resume(&mut self, id: String, tx: &Sender<Cmd>) {
        let Some(fresh) = &self.fresh else {
            self.clear_entries();
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

    /// Empty the transcript, and the tool rows that point into it.
    fn clear_entries(&mut self) {
        self.entries.clear();
        self.tool = None;
        self.done.clear();
    }

    /// Keep `text` for Ctrl+V and say how much was copied.
    fn copy(&mut self, text: String) {
        let n = text.chars().count();
        self.osc52 = Some(format!("\x1b]52;c;{}\x07", crate::api::base64(text.as_bytes())));
        self.clip = text;
        self.push(Role::Info, format!("copied {n} characters"));
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
                self.dirty = true;
            }
            Ev::GoalEnd(why) => {
                self.push(Role::Info, format!("◎ {why}"));
                self.status.goal = None;
                self.dirty = true;
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
                            for line in preview {
                                self.push_diff(line);
                            }
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
            Ev::Question { q, opts, reply } => {
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
            Ev::Stop(reason) => {
                self.pending = None;
                self.status.streaming = false;
                self.cancel.store(true, Ordering::Relaxed);
                self.push(Role::Error, reason);
                self.dirty = true;
            }
            Ev::Done(err) => {
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
                if let Some(e) = err {
                    self.push(Role::Error, e);
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
                    | Some(Pending::KeyEntry { .. })
            );
            if prompt || self.input.is_empty() {
                self.quit = true;
            } else {
                self.input.clear();
                self.cursor = 0;
                self.hist_pos = None;
                self.pending = None;
                self.follow_tail();
                self.dirty = true;
            }
            return;
        }
        if let Some(s) = self.status.suggestion.take() {
            if matches!(key, Key::Right) && self.input.is_empty() && self.pending.is_none() {
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
        if !matches!(key, Key::Select(_)) {
            if let Some((from, to)) = self.selection() {
                let paste = matches!(key, Key::Ctrl('v')) && !self.clip.is_empty();
                let removes = matches!(key, Key::Backspace | Key::Delete);
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
            Key::Delete => {
                if self.cursor < self.input.len() {
                    self.follow_tail();
                    let next = next_boundary(&self.input, self.cursor);
                    self.input.replace_range(self.cursor..next, "");
                    self.sync_menu();
                }
            }
            Key::Ctrl('q') => {
                self.queue_mode = !self.queue_mode;
                self.dirty = true;
            }
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
                let (row, col) = self.caret_position();
                if row > 0 {
                    self.cursor = self.cursor_at(row - 1, col);
                    self.dirty = true;
                } else if let Some(i) = self.queue_at {
                    // Older queued messages, then history.
                    let kept = self.save_queued(tx, false);
                    match kept.unwrap_or(i).checked_sub(1).filter(|&o| o < self.queued.len()) {
                        Some(o) => self.open_queued(o),
                        None if !self.history.is_empty() => {
                            let last = self.history.len() - 1;
                            self.hist_pos = Some(last);
                            self.input = self.history[last].clone();
                            self.cursor = self.input.len();
                        }
                        None => {}
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
                let (row, col) = self.caret_position();
                if row + 1 < self.input.matches('\n').count() + 1 {
                    self.cursor = self.cursor_at(row + 1, col);
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
            Key::Tab if self.input.is_empty() => self.cycle_agent(tx),
            Key::Tab => self.sync_menu(),
            Key::BackTab => {
                // Shift+Tab steps through the modes; it applies at once.
                use crate::config::Permissions::*;
                let next = match self.cfg.live_permissions() {
                    Normal => "strict",
                    Strict => "yolo",
                    Yolo => "normal",
                };
                self.set_mode(next, tx);
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
                for line in lines {
                    self.push_diff(line);
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

    fn submit(&mut self, tx: &Sender<Cmd>) {
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
        // During a turn, queue mode holds the message for the turn's end. A
        // goal ends on any message, as before.
        if self.status.streaming && self.status.goal.is_none() {
            if self.queue_mode {
                self.queued.push(text);
            } else {
                // Steer: the running turn takes it before its next step.
                self.push(Role::User, format!("{text} (steered)"));
                if let Ok(mut box_) = self.steer.lock() {
                    box_.push(text);
                }
            }
            self.dirty = true;
            return;
        }
        self.start_turn(text, tx);
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
        if self.status.streaming {
            self.push(Role::Info, format!("agent: {next} after this turn"));
            self.agent_next = Some(next);
        } else {
            self.push(Role::Info, format!("agent: {next}"));
            self.agent = next;
        }
        self.dirty = true;
    }

    /// Open queued message `i` in the composer and hold the running turn.
    fn open_queued(&mut self, i: usize) {
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
        let kept = if text.trim().is_empty() {
            self.queued.remove(i);
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
        self.push(Role::Info, format!("model: {name}"));
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
        self.onboard = true;
        self.push(
            Role::Info,
            format!("no key for {} ({var}); choose a provider", self.cfg.active_name),
        );
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
                let help = crate::help::commands(width.saturating_sub(2).max(40), true, self.palette.on);
                // Shown output keeps its codes; help's cyan becomes the accent.
                let accent = self.palette.paint(Role::Info, "").replace("\x1b[0m", "");
                self.push(Role::Info, format!("{RAW}{}", help.replace("\x1b[36m", &accent)))
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
            "goal" => {
                let text = match &self.status.goal {
                    Some((goal, round, rounds)) => format!("goal: {goal} · round {round}/{rounds}"),
                    None => "no goal; /goal <text> sets one".into(),
                };
                self.push(Role::Info, text);
            }
            "goal off" => {
                let _ = tx.send(Cmd::GoalOff);
            }
            other if other.starts_with("goal ") => {
                let text = other["goal ".len()..].trim().to_string();
                self.push(Role::User, format!("/goal {text}"));
                self.status.streaming = true;
                self.begin_task();
                let _ = tx.send(Cmd::Goal(text));
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
                        picker.index = Verbosity::ALL.iter().position(|(l, _)| *l == self.verbosity).unwrap_or(1);
                        self.pending = Some(Pending::Pick(picker));
                        self.dirty = true;
                    }
                    level => match Verbosity::parse(level) {
                        Ok(v) => {
                            self.verbosity = v;
                            self.push(Role::Info, format!("verbosity: {}", v.as_str()));
                        }
                        Err(e) => self.push(Role::Error, e),
                    },
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
                                self.verbosity = Verbosity::parse(&choice.value).unwrap_or(self.verbosity);
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
                            PickKind::Provider if self.onboard => {
                                self.choose_onboard_provider(&choice.value, tx);
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
                        self.cancel.store(true, Ordering::Relaxed);
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
            let text = format!("{} thinking · {}", spinner_frame(self.spinner), span(since.elapsed()));
            (Role::Thinking, Line::Text(text))
        })
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
    /// long otherwise, an open prompt included. `quiet` is the time since
    /// the last key or event.
    fn loop_wait(&self, quiet: Duration) -> Duration {
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
        let body_rows = rows.saturating_sub(1 + rule + composer.len()).max(1);

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
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (width, markdown, gap, entry.role as u8, &entry.text).hash(&mut h);
            let key = h.finish();
            if self.drawn.get(i).map(|slot| slot.0) != Some(key) {
                let slot = (key, entry_lines(entry, width, markdown, gap));
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
                        self.targets.push((lead + n + 1, cols.saturating_sub(1), cols, target));
                    }
                }
                Line::Timed(text, time) => {
                    row.push_str(&self.marked(*role, text));
                    let gap = cols.saturating_sub(text.chars().count() + time.chars().count());
                    row.push_str(&" ".repeat(gap));
                    row.push_str(&self.palette.paint(Role::Thinking, time));
                }
            }
            if let Some(Target::Tool(i)) = &labels[n] {
                self.targets.push((lead + n + 1, 1, cols, Target::Tool(*i)));
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
                (Some(i), _) => format!("editing queued {}/{} · Enter save · ↑↓ move", i + 1, self.queued.len()),
                (None, true) => String::new(),
                (None, false) if self.queue_mode => "queue · ^Q steer".into(),
                (None, false) => "steer · ^Q queue".into(),
            };
            let rule = if label.is_empty() || cols < label.chars().count() + 6 {
                "─".repeat(cols)
            } else {
                format!("── {label} {}", "─".repeat(cols - label.chars().count() - 4))
            };
            screen.push(self.palette.paint(tone, &rule));
        }
        let first = body_rows + rule + 1;
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
        let (screen, tail) = self.layout(cols, rows);
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
        let (screen, tail) = self.layout(cols, rows);
        let full = self.shown_size != (cols, rows) || self.shown_overlay || !tail.is_empty();
        let out = paint_rows(&screen, (!full).then_some(&self.shown[..]), cols, &tail);
        self.shown = screen;
        self.shown_size = (cols, rows);
        self.shown_overlay = !tail.is_empty();
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
                Pending::ProviderForm { name, fields, at } => {
                    let (label, value) = fields
                        .get(*at)
                        .cloned()
                        .unwrap_or_else(|| ("field".into(), String::new()));
                    format!("provider {name} · {label}: {value}{CARET}  (Enter next · Esc cancel)")
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
    fn overlay(&self, cols: usize, rows: usize) -> Option<(String, usize, usize, Vec<(usize, usize, usize, Target)>)> {
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
        let mut parts = vec![(q.clone(), 2)];
        for (i, o) in opts.iter().enumerate() {
            parts.push((format!("[{}] {o}", i + 1), 6));
        }
        parts.push(match mode {
            AskMode::Choose => ("[e] answer   Esc stop".to_string(), 2),
            AskMode::Custom => (format!("> {typed}{CARET}"), 4),
        });
        let mut rows = Vec::new();
        for (text, indent) in parts {
            // Continuation rows sit under the text, past the key.
            for (j, line) in wrap(&text, cols.saturating_sub(indent)).into_iter().enumerate() {
                let pad = if j == 0 { 0 } else { indent - 2 };
                rows.push(format!("▌ {}{line}", " ".repeat(pad)));
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
                .map(|(i, r)| {
                    let painted = self.palette.paint(Role::Info, &fit(r, cols));
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
            let hint = fit(s, cols.saturating_sub(3));
            return vec![format!(
                "{}\x1b[7m \x1b[0m{}",
                self.palette.paint(tone, "> "),
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
        let upto = &text[..cursor];
        let caret_row = upto.matches('\n').count();
        let caret_col = cursor - upto.rfind('\n').map(|i| i + 1).unwrap_or(0);
        // The selection in this text's bytes, as the shell `!` shifts them.
        let off = usize::from(shell);
        let sel = self.selection().map(|(a, b)| (a.saturating_sub(off), b.saturating_sub(off)));
        let mut plain = Vec::new();
        let mut at = 0;
        for (i, line) in text.split('\n').enumerate() {
            // Marks at a byte: the selection's end, the caret, then its start,
            // so the caret always sits outside the selection.
            let (s, e) = match sel {
                Some((a, b)) if a < at + line.len() + 1 && b > at => {
                    (Some(a.saturating_sub(at)), Some((b - at).min(line.len())).filter(|_| b <= at + line.len()))
                }
                _ => (None, None),
            };
            let caret = (i == caret_row).then_some(caret_col.min(line.len()));
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
            at += line.len() + 1;
            let prefix = if i != 0 {
                "  "
            } else if shell {
                "$ "
            } else if self.status.streaming && self.queue_mode {
                "» "
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
        // The prompt glyph and its space; `»` is two bytes.
        let cut = row.char_indices().nth(2).map_or(row.len(), |(i, _)| i);
        let (prefix, body) = row.split_at(cut);
        // ponytail: the zero-width selection marks count toward the width, so
        // a selected long row is cut two columns early.
        let fitted = tail(body, cols.saturating_sub(2));
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
        } else if prefix.starts_with('»') {
            // Queue mode: the glyph is muted.
            Role::Thinking
        } else {
            Role::User
        };
        self.palette.paint(tone, prefix) + &fitted
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
        let (row, col) = self.caret_position();
        self.cursor = match m {
            Move::Left => previous_boundary(&self.input, self.cursor),
            Move::Right => next_boundary(&self.input, self.cursor),
            Move::Home => 0,
            Move::End => self.input.len(),
            Move::Up if row > 0 => self.cursor_at(row - 1, col),
            Move::Down if row < self.input.matches('\n').count() => self.cursor_at(row + 1, col),
            Move::Up | Move::Down => self.cursor,
        };
        self.dirty = true;
    }

    /// Row and byte column of the caret within the input.
    fn caret_position(&self) -> (usize, usize) {
        let cursor = self.cursor.min(self.input.len());
        let upto = &self.input[..cursor];
        let row = upto.matches('\n').count();
        let col = upto.rfind('\n').map(|i| i + 1).unwrap_or(0);
        (row, cursor - col)
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
        let paused = if self.queue_at.is_some() && self.status.streaming { " · paused · editing queue" } else { "" };
        let goal = match &self.status.goal {
            Some((_, round, rounds)) => format!(" · goal {round}/{rounds}"),
            None => String::new(),
        };
        // The agent, when there is a choice of them, and the mode.
        let agent = match (&self.agent_next, self.agents.len() > 1) {
            (Some(next), _) => format!("{} → {next} · ", self.agent),
            (None, true) => format!("{} · ", self.agent),
            (None, false) => String::new(),
        };
        let mode = match self.cfg.live_permissions() {
            crate::config::Permissions::Normal => String::new(),
            other => format!("{} · ", other.as_str()),
        };
        let left = format!(
            "{place}{model} · {agent}{mode}{} tok{money}{goal}{paused}{unseen}{thinking}{waiting}",
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
        let parents: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|p| p.chars().take(if p.starts_with('.') { 2 } else { 1 }).collect())
            .collect();
        for short in [format!("{lead}{}/{last}", parents.join("/")), format!("…/{last}")] {
            if width(&short) <= room {
                return Some(short);
            }
        }
    }
    (room >= 8).then(|| tail(last, room))
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
                Ok(Cmd::Goal(text)) => {
                    let mut message = worker_session.start_goal(&text);
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
                        while let Ok(cmd) = cmd_rx.try_recv() {
                            match cmd {
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
                        match worker_session.goal_next() {
                            Ok(next) => message = next,
                            Err(why) => {
                                let _ = ev_tx.send(Ev::GoalEnd(why));
                                break;
                            }
                        }
                    }
                }
                Ok(Cmd::GoalOff) => {
                    if worker_session.goal.take().is_some() {
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
                Ok(Cmd::Detach) => {
                    if let Some(image) = worker_session.attachments.pop() {
                        let _ = ev_tx.send(Ev::Info(format!("removed {}", crate::images::marker(&image))));
                    }
                }
                Ok(Cmd::Prompt(text)) => {
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
    fn decodes_the_wheel_and_a_left_press() {
        let mut buf = b"\x1b[<64;10;5M\x1b[<65;10;5M\x1b[<0;3;4M\x1b[<0;3;4m\x1b[<68;1;1M".to_vec();
        let keys = decode(&mut buf);
        assert!(matches!(keys[0], Key::WheelUp), "{keys:?}");
        assert!(matches!(keys[1], Key::WheelDown), "{keys:?}");
        assert!(matches!(keys[2], Key::Click(3, 4)), "press: {keys:?}");
        assert!(matches!(keys[3], Key::Unknown), "release: {keys:?}");
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
        // Shift selects; any other modifier keeps the plain arrow.
        let mut buf = b"\x1b[1;5A".to_vec();
        let keys = decode(&mut buf);
        assert_eq!(keys.len(), 1);
        assert!(matches!(keys[0], Key::Up));
    }

    #[test]
    fn a_selection_is_extended_replaced_and_cleared() {
        let (tx, _cmds) = channel();
        let draft = |text: &str| {
            let mut a = app();
            a.input = text.into();
            a.cursor = text.len();
            a
        };
        // Replace a word by typing.
        let mut a = draft("make it red");
        for _ in 0..3 {
            a.handle_key(Key::Select(Move::Left), &tx);
        }
        assert_eq!(a.selection(), Some((8, 11)));
        for c in "blue".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        assert_eq!((a.input.as_str(), a.cursor), ("make it blue", 12));
        // Backspace removes it; the caret sits where it began.
        let mut a = draft("make it red");
        a.handle_key(Key::Select(Move::Left), &tx);
        a.handle_key(Key::Select(Move::Left), &tx);
        a.handle_key(Key::Backspace, &tx);
        assert_eq!((a.input.as_str(), a.cursor), ("make it r", 9));
        // A plain move clears it and leaves the text alone.
        let mut a = draft("make it red");
        a.handle_key(Key::Select(Move::Home), &tx);
        a.handle_key(Key::Right, &tx);
        assert_eq!((a.input.as_str(), a.selection()), ("make it red", None));
        // Ctrl+V and a newline replace it too.
        let mut a = draft("make it red");
        a.clip = "X".into();
        a.handle_key(Key::Select(Move::Left), &tx);
        a.handle_key(Key::Ctrl('v'), &tx);
        assert_eq!(a.input, "make it reX");
        a.handle_key(Key::Select(Move::Left), &tx);
        a.handle_key(Key::Newline, &tx);
        assert_eq!(a.input, "make it re\n");
        // Across rows, without recalling history.
        let mut a = draft("ab\ncd");
        a.history = vec!["old".into()];
        a.handle_key(Key::Select(Move::Up), &tx);
        assert_eq!((a.input.as_str(), a.selection()), ("ab\ncd", Some((2, 5))));
        a.handle_key(Key::Select(Move::Up), &tx);
        assert_eq!(a.input, "ab\ncd", "Shift+Up recalled history");
    }

    #[test]
    fn a_selection_is_drawn_reversed() {
        let (tx, _cmds) = channel();
        // Colour off: reverse video still shows it, as it does the caret.
        let mut a = app();
        a.input = "make it red".into();
        a.cursor = a.input.len();
        for _ in 0..3 {
            a.handle_key(Key::Select(Move::Left), &tx);
        }
        let out = a.frame(54, 10);
        assert!(out.contains("make it \x1b[7m \x1b[0m\x1b[7mred\x1b[27m"), "{out:?}");
        assert!(!out.contains('\u{e000}') && !out.contains('\u{e001}'), "marks leaked: {out:?}");
        // Across rows: the end of the first and all of the second.
        let mut b = app();
        b.input = "ab\ncd".into();
        b.cursor = 5;
        b.handle_key(Key::Select(Move::Up), &tx);
        let out = b.frame(54, 10);
        assert!(out.contains("> ab\x1b[7m \x1b[0m\x1b[7m\x1b[27m"), "{out:?}");
        assert!(out.contains("  \x1b[7mcd\x1b[27m"), "{out:?}");
        // A long selected draft still fits the width.
        let mut c = app();
        c.input = "x".repeat(120);
        c.cursor = 120;
        for _ in 0..80 {
            c.handle_key(Key::Select(Move::Left), &tx);
        }
        let row = strip(&c.frame(54, 10)).lines().find(|l| l.starts_with("> ")).unwrap().trim_end_matches('\r').to_string();
        assert!(row.chars().count() <= 54, "{row:?}");
    }

    #[test]
    fn ctrl_v_attaches_a_clipboard_image_or_pastes_the_copy() {
        let (tx, cmds) = channel();
        let dir = scratch("paste");
        let png = dir.join("shot.png");
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&[0u8; 16]);
        std::fs::write(&png, &bytes).unwrap();
        // An image on the clipboard is attached with the usual marker.
        let mut a = app();
        a.paste_cmd = Some(format!("cat '{}'", png.display()));
        a.input = "keep".into();
        a.cursor = 4;
        assert!(a.paste_image(&tx, &dir));
        assert!(a.entries.last().unwrap().text.starts_with("[image pasted.png ·"), "{:?}", a.entries.last().unwrap().text);
        assert!(matches!(cmds.try_recv(), Ok(Cmd::Attach(v)) if v.len() == 1 && v[0].name == "pasted.png"));
        assert_eq!(a.input, "keep", "the draft changed");
        // Text on the clipboard: Ctrl+V pastes imp's last copy instead.
        let mut b = app();
        b.paste_cmd = Some("printf 'some text'".into());
        b.clip = "copied".into();
        b.handle_key(Key::Ctrl('v'), &tx);
        assert_eq!(b.input, "copied");
        assert!(cmds.try_recv().is_err(), "text was attached");
        // No clipboard: one note, however often Ctrl+V is pressed.
        let mut c = app();
        c.clip = "x".into();
        c.handle_key(Key::Ctrl('v'), &tx);
        c.handle_key(Key::Ctrl('v'), &tx);
        assert_eq!(c.input, "xx");
        assert_eq!(c.entries.iter().filter(|e| e.text.contains("cannot paste images")).count(), 1);
    }

    #[test]
    fn ctrl_q_toggles_steer_and_queue_shown_in_the_rule() {
        let (tx, _cmds) = channel();
        let mut b = Vec::from(&b"\x11"[..]);
        assert_eq!(decode(&mut b), vec![Key::Ctrl('q')]);
        let mut a = app();
        // Idle: a plain rule and the usual glyph.
        let idle = strip(&a.frame(60, 10));
        assert!(!idle.contains("steer") && idle.contains("\r\n> "), "{idle:?}");
        a.status.streaming = true;
        let steer = strip(&a.frame(60, 10));
        assert!(steer.contains("── steer · ^Q queue ─"), "{steer:?}");
        a.handle_key(Key::Ctrl('q'), &tx);
        let queue = strip(&a.frame(60, 10));
        assert!(queue.contains("── queue · ^Q steer ─") && queue.contains("\r\n» "), "{queue:?}");
        let rule = queue.lines().find(|l| l.contains("queue · ^Q")).unwrap().trim_end_matches('\r');
        assert_eq!(rule.chars().count(), 60, "{rule:?}");
        // The mode stays when the turn ends; the label goes.
        a.status.streaming = false;
        let done = strip(&a.frame(60, 10));
        assert!(a.queue_mode && !done.contains("^Q"), "{done:?}");
    }

    /// An app whose worker is a pair of test channels.
    fn worker_app() -> (App, Receiver<Cmd>, Sender<Ev>) {
        let mut a = app();
        let (cmd_tx, cmd_rx) = channel();
        let (ev_tx, ev_rx) = channel();
        a.worker = Some((cmd_tx, ev_rx, Default::default()));
        (a, cmd_rx, ev_tx)
    }

    #[test]
    fn queued_messages_wait_for_the_turn_to_end() {
        let (mut a, cmds, _ev) = worker_app();
        let tx = a.worker.as_ref().unwrap().0.clone();
        a.status.streaming = true;
        a.begin_task();
        a.status.steps = 4;
        a.queue_mode = true;
        a.input = "also add tests".into();
        a.cursor = a.input.len();
        a.handle_key(Key::Enter, &tx);
        assert_eq!(a.queued, vec!["also add tests"]);
        assert!(!a.entries.iter().any(|e| e.text == "also add tests"), "queued text reached the transcript");
        assert!(cmds.try_recv().is_err(), "a queued message was sent");
        assert_eq!(a.status.steps, 4, "the meter restarted");
        assert!(strip(&a.frame(60, 12)).contains("⋯ also add tests"));
        // The turn ends: the queued message starts the next one.
        a.apply(Ev::Done(None));
        assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "also add tests"));
        assert!(a.queued.is_empty() && a.status.streaming);
        // A stopped turn leaves the queue waiting.
        a.queued.push("later".into());
        a.cancel.store(true, Ordering::Relaxed);
        a.apply(Ev::Done(None));
        assert_eq!(a.queued, vec!["later"]);
        assert!(cmds.try_recv().is_err());
        // Steer mode, until steering lands, still keeps the meter.
        let (mut b, _c, _e) = worker_app();
        let tx = b.worker.as_ref().unwrap().0.clone();
        b.status.streaming = true;
        b.begin_task();
        b.status.steps = 3;
        b.input = "more".into();
        b.cursor = 4;
        b.handle_key(Key::Enter, &tx);
        assert_eq!(b.status.steps, 3, "the meter restarted");
    }

    #[test]
    fn queued_messages_are_walked_and_edited_in_place() {
        let (mut a, cmds, _ev) = worker_app();
        let tx = a.worker.as_ref().unwrap().0.clone();
        a.status.streaming = true;
        a.history = vec!["old".into()];
        a.queued = vec!["first".into(), "second".into()];
        // Newest first, holding the turn, then older, then history.
        a.handle_key(Key::Up, &tx);
        assert_eq!((a.queue_at, a.input.as_str()), (Some(1), "second"));
        assert!(a.hold.load(Ordering::SeqCst), "the turn is not held");
        let shown = strip(&a.frame(70, 14));
        assert!(shown.contains("editing queued 2/2 · Enter save · ↑↓ move"), "{shown:?}");
        assert!(a.status_line(120).contains("paused · editing queue"));
        a.handle_key(Key::Up, &tx);
        assert_eq!((a.queue_at, a.input.as_str()), (Some(0), "first"));
        a.handle_key(Key::Up, &tx);
        assert_eq!((a.queue_at, a.input.as_str(), a.hist_pos), (None, "old", Some(0)));
        assert!(!a.hold.load(Ordering::SeqCst), "the hold outlived the edit");
        // Edit in place with Enter.
        a.input.clear();
        a.cursor = 0;
        a.hist_pos = None;
        a.handle_key(Key::Up, &tx);
        a.input = "second, edited".into();
        a.cursor = a.input.len();
        a.handle_key(Key::Enter, &tx);
        assert_eq!(a.queued, vec!["first", "second, edited"]);
        assert!(a.queue_at.is_none() && a.input.is_empty());
        assert!(cmds.try_recv().is_err(), "saving sent something");
        // Emptied, it is removed.
        a.handle_key(Key::Up, &tx);
        a.input.clear();
        a.cursor = 0;
        a.handle_key(Key::Enter, &tx);
        assert_eq!(a.queued, vec!["first"]);
        // Down past the newest returns to an empty draft.
        a.handle_key(Key::Up, &tx);
        a.handle_key(Key::Down, &tx);
        assert!(a.queue_at.is_none() && a.input.is_empty() && a.queued == vec!["first"]);
        // A turn ending mid-edit waits for the edit, then goes on.
        a.handle_key(Key::Up, &tx);
        a.apply(Ev::Done(None));
        assert!(cmds.try_recv().is_err(), "sent while being edited");
        a.handle_key(Key::Enter, &tx);
        assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "first"));
    }

    #[test]
    fn enter_steers_a_running_turn() {
        let (mut a, cmds, _ev) = worker_app();
        let tx = a.worker.as_ref().unwrap().0.clone();
        a.status.streaming = true;
        a.input = "use pnpm, not npm".into();
        a.cursor = a.input.len();
        a.handle_key(Key::Enter, &tx);
        assert_eq!(a.entries.last().unwrap().text, "use pnpm, not npm (steered)");
        assert_eq!(*a.steer.lock().unwrap(), vec!["use pnpm, not npm".to_string()]);
        assert!(cmds.try_recv().is_err(), "a steered message was sent as a turn");
        // Steered too late: the next turn carries it.
        a.apply(Ev::Done(None));
        assert!(matches!(cmds.try_recv(), Ok(Cmd::Prompt(t)) if t == "use pnpm, not npm"));
        assert!(a.status.streaming && a.steer.lock().unwrap().is_empty());
        // After a stop it waits at the front of the queue.
        a.queued = vec!["queued".into()];
        a.steer.lock().unwrap().push("late".into());
        a.cancel.store(true, Ordering::Relaxed);
        a.apply(Ev::Done(None));
        assert_eq!(a.queued, vec!["late", "queued"]);
        assert!(cmds.try_recv().is_err());
    }

    #[test]
    fn an_open_queued_message_holds_the_turn_and_esc_still_stops_it() {
        let (mut a, _cmds, _ev) = worker_app();
        let tx = a.worker.as_ref().unwrap().0.clone();
        a.status.streaming = true;
        a.queued = vec!["next".into()];
        a.handle_key(Key::Up, &tx);
        assert!(a.hold.load(Ordering::SeqCst));
        a.handle_key(Key::Esc, &tx);
        assert!(a.cancel.load(Ordering::Relaxed), "Esc did not stop the held turn");
        // Leaving the message releases the hold.
        a.handle_key(Key::Enter, &tx);
        assert!(!a.hold.load(Ordering::SeqCst));
        assert_eq!(a.queued, vec!["next"]);
    }

    #[test]
    fn shift_tab_cycles_the_mode_at_once() {
        let (tx, _cmds) = channel();
        let mut a = app();
        a.status.streaming = true;
        let mut seen = Vec::new();
        for _ in 0..3 {
            a.handle_key(Key::BackTab, &tx);
            seen.push(a.cfg.live_permissions());
        }
        use crate::config::Permissions::*;
        assert_eq!(seen, vec![Strict, Yolo, Normal]);
        assert_eq!(a.entries.len(), 3, "one line per change");
        // Yolo turns the rule red.
        a.handle_key(Key::BackTab, &tx);
        a.handle_key(Key::BackTab, &tx);
        assert!(a.yolo());
    }

    #[test]
    fn tab_cycles_agents_on_an_empty_draft() {
        let (tx, cmds) = channel();
        let mut a = app();
        a.agents = vec![("build".into(), String::new()), ("review".into(), String::new())];
        a.agent = "build".into();
        a.handle_key(Key::Tab, &tx);
        assert_eq!(a.agent, "review");
        assert!(matches!(cmds.try_recv(), Ok(Cmd::SetAgent(n)) if n == "review"));
        // Text in the draft: nothing changes.
        a.input = "fix".into();
        a.cursor = 3;
        a.handle_key(Key::Tab, &tx);
        assert_eq!(a.agent, "review");
        assert!(cmds.try_recv().is_err());
        // Mid-turn: the choice waits for the turn's end.
        a.input.clear();
        a.cursor = 0;
        a.status.streaming = true;
        a.handle_key(Key::Tab, &tx);
        assert_eq!((a.agent.as_str(), a.agent_next.as_deref()), ("review", Some("build")));
        assert!(a.entries.last().unwrap().text.ends_with("after this turn"));
        a.apply(Ev::Done(None));
        assert_eq!((a.agent.as_str(), a.agent_next.as_deref()), ("build", None));
        // One agent: Tab does nothing.
        let mut b = app();
        b.agents = vec![("build".into(), String::new())];
        b.agent = "build".into();
        b.handle_key(Key::Tab, &tx);
        assert!(b.entries.is_empty());
    }

    #[test]
    fn the_status_line_names_the_agent_and_an_unusual_mode() {
        let mut a = app();
        a.agents = vec![("build".into(), String::new())];
        a.agent = "build".into();
        assert!(a.status_line(80).starts_with("deepseek-flash · 0 tok"), "{}", a.status_line(80));
        a.agents.push(("review".into(), String::new()));
        assert!(a.status_line(80).starts_with("deepseek-flash · build · 0 tok"));
        a.agent_next = Some("review".into());
        assert!(a.status_line(80).starts_with("deepseek-flash · build → review · 0 tok"));
        a.cfg.set_permissions(crate::config::Permissions::Strict);
        assert!(a.status_line(80).starts_with("deepseek-flash · build → review · strict · 0 tok"));
        assert!(a.status_line(54).chars().count() <= 54);
    }

    #[test]
    fn delete_removes_forward() {
        let (tx, _cmds) = channel();
        let mut a = app();
        a.input = "cat".into();
        a.cursor = 1;
        a.handle_key(Key::Delete, &tx);
        assert_eq!((a.input.as_str(), a.cursor), ("ct", 1));
        a.cursor = 2;
        a.handle_key(Key::Delete, &tx);
        assert_eq!(a.input, "ct", "Delete at the end changed the draft");
        a.input = "né".into();
        a.cursor = 1;
        a.handle_key(Key::Delete, &tx);
        assert_eq!(a.input, "n", "a multi-byte character was split");
    }

    #[test]
    fn the_key_reader_never_reads_while_a_command_holds_the_terminal() {
        use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
        static ATT: AtomicBool = AtomicBool::new(false);
        static PARK: AtomicBool = AtomicBool::new(false);
        let mut fds = [0 as c_int; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
        let (tx, rx) = channel();
        let fd = fds[0];
        std::thread::spawn(move || read_keys(fd, tx, &ATT, &PARK));
        let put = |b: &[u8]| unsafe { write(fds[1], b.as_ptr().cast(), b.len()) };
        put(b"a");
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).ok(), Some(b"a".to_vec()));
        // Attached: the reader parks, and bytes stay for the command.
        ATT.store(true, SeqCst);
        let t = Instant::now();
        while !PARK.load(SeqCst) && t.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(PARK.load(SeqCst), "the reader did not park");
        put(b"for the shell");
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "it read while attached");
        let mut left = [0u8; 32];
        let n = unsafe { read(fds[0], left.as_mut_ptr().cast(), left.len()) };
        assert_eq!(&left[..n as usize], b"for the shell");
        // Back: it reads again.
        ATT.store(false, SeqCst);
        put(b"b");
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).ok(), Some(b"b".to_vec()));
    }

    #[test]
    fn shift_moves_and_delete_decode() {
        let cases: [(&[u8], Key); 13] = [
            (b"\x1b[1~", Key::Home),
            (b"\x1b[4~", Key::End),
            (b"\x1b[7~", Key::Home),
            (b"\x1b[8~", Key::End),
            (b"\x1b[1;2A", Key::Select(Move::Up)),
            (b"\x1b[1;2B", Key::Select(Move::Down)),
            (b"\x1b[1;2C", Key::Select(Move::Right)),
            (b"\x1b[1;2D", Key::Select(Move::Left)),
            (b"\x1b[1;2H", Key::Select(Move::Home)),
            (b"\x1b[1;2F", Key::Select(Move::End)),
            (b"\x1b[3~", Key::Delete),
            (b"\x1b[3;2~", Key::Delete),
            (b"\x1b[1;5D", Key::Left),
        ];
        for (bytes, want) in cases {
            let mut buf = bytes.to_vec();
            assert_eq!(decode(&mut buf), vec![want], "{bytes:?}");
        }
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
        assert!(spinner_frames().contains(first.chars().next().unwrap()), "{first}");
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
    fn backspace_on_an_empty_draft_removes_the_last_image() {
        // The chat asks only when the draft is empty.
        let (tx, cmds) = channel();
        let mut a = app();
        a.input = "x".into();
        a.cursor = 1;
        a.handle_key(Key::Backspace, &tx);
        assert!(cmds.try_recv().is_err(), "a real delete detached an image");
        a.handle_key(Key::Backspace, &tx);
        assert!(matches!(cmds.try_recv(), Ok(Cmd::Detach)));
        // The worker drops the newest attachment and says which.
        let dir = std::env::temp_dir().join(format!("imp-detach-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let session = Session::new(
            Config::default(),
            dir.clone(),
            dir.join("out"),
            String::new(),
            crate::store::Agent::builtin(),
            Vec::new(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
        );
        let (tx, rx, _) = spawn_worker(session, dir.join("sessions"));
        let image = |name: &str| crate::api::Image { file: format!("{name}.png"), name: name.into(), mime: "image/png".into(), bytes: 2048 };
        tx.send(Cmd::Attach(vec![image("one"), image("two")])).unwrap();
        tx.send(Cmd::Detach).unwrap();
        tx.send(Cmd::Detach).unwrap();
        tx.send(Cmd::Detach).unwrap();
        let mut said = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while said.len() < 2 && Instant::now() < deadline {
            if let Ok(Ev::Info(t)) = rx.recv_timeout(Duration::from_millis(100)) {
                said.push(t);
            }
        }
        assert_eq!(said, vec!["removed [image two · 2 KB]", "removed [image one · 2 KB]"]);
        assert!(!matches!(rx.recv_timeout(Duration::from_millis(300)), Ok(Ev::Info(_))), "a third removal spoke");
        let _ = tx.send(Cmd::Quit);
    }

    #[test]
    fn a_due_check_in_runs_a_turn_in_the_worker() {
        let dir = std::env::temp_dir().join(format!("imp-wake-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut cfg = Config::default();
        cfg.color = ColorMode::Never;
        let mut session = Session::new(
            cfg,
            dir.clone(),
            dir.join("out"),
            String::new(),
            crate::store::Agent::builtin(),
            Vec::new(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
        );
        session.transport = Box::new(std::sync::Arc::new(crate::transport::Fake::new(vec![
            crate::transport::Script::Ok(crate::transport::text_reply("the copy is 64% done")),
        ])));
        session.check_in_now();
        let (_tx, rx, _) = spawn_worker(session, dir.join("sessions"));
        let deadline = Instant::now() + Duration::from_secs(10);
        let (mut woke, mut text, mut done) = (None, String::new(), false);
        while !done && Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(Ev::Woke(w)) => woke = Some(w),
                Ok(Ev::Token(t)) => text.push_str(&t),
                Ok(Ev::Done(_)) => done = true,
                _ => {}
            }
        }
        assert_eq!(woke.as_deref(), Some("check-in"));
        assert!(done && text.contains("64% done"), "{text:?}");
    }

    #[test]
    fn a_goal_runs_rounds_in_the_worker_until_met() {
        let dir = std::env::temp_dir().join(format!("imp-goal-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut session = Session::new(
            Config::default(),
            dir.clone(),
            dir.join("out"),
            String::new(),
            crate::store::Agent::builtin(),
            Vec::new(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(80)),
        );
        session.transport = Box::new(std::sync::Arc::new(crate::transport::Fake::new(vec![
            crate::transport::Script::Ok(crate::transport::text_reply("fixed one; one left")),
            crate::transport::Script::Ok(crate::transport::text_reply("all green\nGOAL MET 13 tests OK")),
        ])));
        let (tx, rx, _) = spawn_worker(session, dir.join("sessions"));
        tx.send(Cmd::Goal("all tests pass".into())).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let (mut rounds, mut end, mut dones) = (Vec::new(), None, 0);
        while end.is_none() && Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(Ev::GoalRound { round, rounds: n, .. }) => rounds.push((round, n)),
                Ok(Ev::GoalEnd(why)) => end = Some(why),
                Ok(Ev::Done(_)) => dones += 1,
                _ => {}
            }
        }
        assert_eq!(rounds, vec![(1, 5), (2, 5)]);
        assert_eq!(dones, 2);
        assert_eq!(end.as_deref(), Some("goal met"));
    }

    #[test]
    fn goal_command_starts_shows_and_ends() {
        let mut a = app();
        let (tx, rx) = channel();
        a.run_command("goal all tests pass", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Goal(t)) if t == "all tests pass"));
        assert!(a.entries.iter().any(|e| e.text == "/goal all tests pass"));
        a.apply(Ev::GoalRound { text: "all tests pass".into(), round: 2, rounds: 5 });
        assert!(strip(&a.status_line(100)).contains("goal 2/5"));
        assert!(a.entries.iter().any(|e| e.text.contains("goal round 2/5")));
        a.run_command("goal", &tx);
        assert!(a.entries.last().unwrap().text.contains("all tests pass · round 2/5"));
        a.run_command("goal off", &tx);
        assert!(matches!(rx.try_recv(), Ok(Cmd::GoalOff)));
        a.apply(Ev::GoalEnd("goal cleared".into()));
        assert!(!strip(&a.status_line(100)).contains("goal "));
    }

    #[test]
    fn a_woken_session_out_of_view_joins_the_queue() {
        let mut a = multi_app("woke");
        let (tx, _rx) = channel();
        a.run_command("new", &tx);
        assert_eq!((a.at, a.queue().len()), (1, 0));
        a.apply_to(0, Ev::Woke("check-in: job 3".into()));
        assert!(a.panes[0].as_ref().unwrap().status.streaming, "the woken session is not working");
        a.apply_to(0, Ev::Token("64% done".into()));
        a.apply_to(0, Ev::Done(None));
        assert_eq!(a.queue(), vec![0], "the woken session is not waiting");
        assert!(a.panes[0].as_ref().unwrap().entries.iter().any(|e| e.text == "⏰ check-in: job 3"));
        assert_eq!(wake_summary("[Check-in from imp]\nJob 3 ended with exit 0. job 3 x"), "job ended: job 3");
        assert_eq!(
            wake_summary("Job 3 ended with exit 0.\n\nThe check-in you asked for 30s ago:\njob 3  exit 0"),
            "job ended, check-in: job 3"
        );
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
        // Brought into view, the prompt is still open.
        a.show(0);
        assert!(matches!(a.pending, Some(Pending::Confirm { .. })));
        assert!(answers.try_recv().is_err(), "a prompt was answered by switching");
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
        // One directory per call: tests sharing a tag run in parallel.
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("imp-picker-{tag}-{}-{n}", std::process::id()));
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
        assert!(frame.contains("  $ find . | wc -l\r\n"), "{frame:?}");
        assert!(frame.contains("▌ run it?   [y] yes  [n] no"), "{frame:?}");
    }

    #[test]
    fn the_confirm_row_has_no_countdown_and_enter_approves() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, answers) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        let frame = strip(&a.frame(80, 12));
        let row = frame.lines().find(|l| l.contains("[a] always")).expect("chips");
        assert_eq!(row.trim_end().trim_start_matches('\u{7}'), "▌ run it?   [y] yes  [n] no  [s] session  [a] always");
        a.handle_key(Key::Enter, &tx);
        assert_eq!(answers.try_recv().ok(), Some(Answer::Yes));
    }

    /// Click the middle of the first target that matches.
    fn click(a: &mut App, tx: &Sender<Cmd>, want: impl Fn(&Target) -> bool) {
        let t = a.targets.iter().find(|t| want(&t.3)).cloned().expect("no such target");
        a.handle_key(Key::Click(((t.1 + t.2) / 2) as u16, t.0 as u16), tx);
    }

    #[test]
    fn a_click_on_a_chip_answers_like_its_key() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        a.frame(80, 12);
        click(&mut a, &tx, |t| matches!(t, Target::Key(Key::Char('n'))));
        assert_eq!(rx.try_recv().ok(), Some(Answer::No));
    }

    #[test]
    fn a_click_on_an_option_answers_the_question() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, rx) = channel();
        a.apply(Ev::Question { q: "which?".into(), opts: vec!["red".into(), "blue".into()], reply });
        a.frame(80, 16);
        click(&mut a, &tx, |t| matches!(t, Target::Key(Key::Char('2'))));
        assert_eq!(rx.try_recv().ok(), Some(Some("blue".to_string())));
    }

    #[test]
    fn a_click_on_a_picker_row_chooses_it_and_a_hint_runs_its_key() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.run_command("mode", &tx);
        a.frame(100, 30);
        click(&mut a, &tx, |t| matches!(t, Target::Row(2)));
        assert_eq!(a.cfg.live_permissions(), crate::config::Permissions::Yolo);

        a.run_command("mode", &tx);
        a.frame(100, 30);
        click(&mut a, &tx, |t| matches!(t, Target::Key(Key::Esc)));
        assert!(a.pending.is_none(), "Esc close did not close the picker");
    }

    #[test]
    fn always_takes_two_clicks_but_one_key() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        a.frame(80, 12);
        let always = |t: &Target| matches!(t, Target::Key(Key::Char('a')));
        click(&mut a, &tx, always);
        assert!(rx.try_recv().is_err(), "one click applied always");
        assert!(a.armed);
        a.frame(80, 12);
        click(&mut a, &tx, always);
        assert_eq!(rx.try_recv().ok(), Some(Answer::Always));

        // Anything else in between clears the mark.
        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        a.frame(80, 12);
        click(&mut a, &tx, always);
        a.handle_key(Key::Click(1, 1), &tx);
        assert!(!a.armed);
        a.frame(80, 12);
        click(&mut a, &tx, always);
        assert!(rx.try_recv().is_err(), "the mark survived another click");

        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: true });
        a.handle_key(Key::Char('a'), &tx);
        assert_eq!(rx.try_recv().ok(), Some(Answer::Always));
    }

    #[test]
    fn copy_labels_copy_a_reply_or_its_code() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let reply = "Run this:\n```sh\ncargo test\n```\nthen push.";
        a.push(Role::Assistant, reply);
        let frame = strip(&a.frame(40, 12));
        for line in frame.split("\r\n") {
            assert!(line.chars().count() <= 40, "too wide: {line:?}");
        }
        assert_eq!(frame.matches(COPY).count(), 2, "{frame}");
        click(&mut a, &tx, |t| matches!(t, Target::Copy(_, Some(0))));
        assert_eq!(a.clip, "cargo test");
        a.frame(40, 12);
        click(&mut a, &tx, |t| matches!(t, Target::Copy(_, None)));
        assert_eq!(a.clip, reply);
        assert!(a.entries.iter().any(|e| e.text == format!("copied {} characters", reply.len())));
    }

    #[test]
    fn ctrl_v_pastes_the_copy_and_osc52_carries_it() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('v'), &tx);
        assert_eq!(a.input, "", "an empty buffer changed the composer");
        a.handle_key(Key::Char('>'), &tx);
        a.copy("a\nb".into());
        assert_eq!(a.osc52.as_deref(), Some("\x1b]52;c;YQpi\x07"));
        a.handle_key(Key::Ctrl('v'), &tx);
        assert_eq!(a.input, ">a\nb");
    }

    #[test]
    fn copy_command_takes_the_last_reply_or_code_block() {
        let mut a = app();
        let (tx, cmds) = channel();
        a.run_command("copy", &tx);
        assert_eq!(a.entries.last().unwrap().text, "nothing to copy");
        a.push(Role::Assistant, "first\n```\nold\n```");
        a.push(Role::Assistant, "no code here");
        a.run_command("copy", &tx);
        assert_eq!(a.clip, "no code here");
        a.run_command("copy code", &tx);
        assert_eq!(a.clip, "old", "the newest block is in an earlier reply");
        assert!(cmds.try_recv().is_err(), "a request was sent");
    }

    #[test]
    fn ctrl_g_releases_the_mouse_until_the_next_key() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.handle_key(Key::Ctrl('g'), &tx);
        assert!(a.released);
        assert_eq!(a.mouse_seq, Some(MOUSE_OFF));
        assert!(strip(&a.status_line(80)).contains("mouse released"));
        a.mouse_seq = None;
        a.handle_key(Key::Char('h'), &tx);
        assert!(!a.released);
        assert_eq!(a.mouse_seq, Some(MOUSE_ON));
        assert_eq!(a.input, "h", "the key that took the mouse back was lost");

        let mut b = app();
        b.cfg.mouse = false;
        b.handle_key(Key::Ctrl('g'), &tx);
        assert!(!b.released && b.mouse_seq.is_none());
    }

    #[test]
    fn no_copy_labels_with_the_mouse_off() {
        let mut a = app();
        a.cfg.mouse = false;
        a.push(Role::Assistant, "hello\n```\nx\n```");
        assert!(!a.frame(40, 12).contains(COPY));
        assert!(!a.targets.iter().any(|t| matches!(t.3, Target::Copy(..))));
    }

    #[test]
    fn a_click_elsewhere_does_nothing() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, rx) = channel();
        a.apply(Ev::Confirm { what: "run `npm test`".into(), reply, always: false });
        a.frame(80, 12);
        a.handle_key(Key::Click(1, 1), &tx);
        assert!(rx.try_recv().is_err());
        assert!(a.pending.is_some());
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
    fn an_unanswered_prompt_waits() {
        let mut a = app();
        let (reply, answers) = channel();
        a.status.streaming = true;
        a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
        a.frame(80, 12);
        assert!(a.pending.is_some() && a.status.streaming, "the prompt gave up");
        assert!(answers.try_recv().is_err());
        assert!(!a.cancel.load(Ordering::Relaxed), "the turn was cancelled");
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
        let narrow = a.status_line(30);
        assert!(narrow.starts_with("deepseek-flash"), "{narrow}");
    }

    #[test]
    fn the_status_line_shortens_a_long_directory() {
        let mut a = app();
        a.agents.clear();
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        a.root = home.join("Projects/imp/src/providers");
        let left = "deepseek-flash · 0 tok · $0.00";
        let at = |a: &App, cols: usize| {
            let line = a.status_line(cols);
            assert!(line.starts_with(left) && line.chars().count() <= cols, "{line}");
            line
        };
        assert!(at(&a, 100).ends_with("~/Projects/imp/src/providers"));
        assert!(at(&a, 54).ends_with("  ~/P/i/s/providers"));
        assert!(at(&a, 40).ends_with("  …oviders"));
        assert_eq!(at(&a, 38), left, "6 columns left: left out");
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
        plain.agents.clear();
        let before = plain.frame(80, 20);
        plain.cfg.set_permissions(crate::config::Permissions::Yolo);
        // Only the status line names it; no colour appears.
        let after = plain.frame(80, 20);
        assert!(!after.contains("38;"), "colour with colour off");
        let rows = |f: &str| f.split("\r\n").map(str::to_string).collect::<Vec<_>>();
        let (b, a2) = (rows(&before), rows(&after));
        assert_eq!(b[..b.len() - 1], a2[..a2.len() - 1], "more than the status line changed");
        assert!(a2.last().unwrap().contains("deepseek-flash · yolo · "), "{after:?}");
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
    fn typed_output_keeps_its_colour() {
        let mut a = colour_app();
        a.push(Role::Tool, format!("{RAW}\u{1b}[31mred\u{1b}[0m \u{1b}[2Jplain"));
        let screen = a.frame(40, 10);
        assert!(screen.contains("\u{1b}[31mred\u{1b}[0m plain"), "{screen:?}");
        assert!(!screen.contains("\u{1b}[2J"), "a clear reached the screen");
        let mut plain = app();
        plain.push(Role::Tool, format!("{RAW}\u{1b}[31mred\u{1b}[0m"));
        let screen = plain.frame(40, 10);
        assert!(screen.contains("  red") && !screen.contains("\u{1b}[31m"), "{screen:?}");
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
        assert!(!a.entries[0].text.contains("$ ls -la"), "Ctrl+O still expands");
        assert_eq!(a.entries.last().unwrap().text, "nothing waiting");
        a.handle_key(Key::Ctrl('e'), &tx);
        let text = a.entries[0].text.clone();
        assert!(text.contains("\n  $ ls -la"), "{text}");
        assert!(text.contains("total 8"), "{text}");
        a.handle_key(Key::Ctrl('e'), &tx);
        assert!(!a.entries[0].text.contains("$ ls -la"), "it did not collapse");
    }

    #[test]
    fn inline_code_sits_on_its_tint_and_is_not_bold() {
        let mut a = colour_app();
        a.push(Role::Assistant, "could not `bash` it");
        let out = a.frame(60, 12);
        let [r, g, b] = crate::theme::tint(
            a.palette.theme.rgb(crate::theme::Slot::Code),
            a.palette.theme.rgb(crate::theme::Slot::Text),
        );
        let [x, y, z] = a.palette.theme.rgb(crate::theme::Slot::Text);
        let code = format!("\x1b[48;2;{r};{g};{b}m\x1b[38;2;{x};{y};{z}mbash\x1b[0m");
        assert!(out.contains(&code), "no tint: {out:?}");
        assert!(!out.contains("\x1b[1m"), "code is bold: {out:?}");
        // Mono has no tint, so code is reversed.
        a.palette = a.palette.with_theme(crate::theme::by_name("mono").unwrap());
        a.push(Role::Assistant, "and `sh`");
        assert!(a.frame(60, 12).contains("\x1b[7msh\x1b[0m"));
    }

    #[test]
    fn markdown_structure_is_styled() {
        let mut a = colour_app();
        a.cfg.links = crate::config::Links::Always;
        a.push(Role::Assistant, "# Big one\n## Mid\n### Low\n- item\n> said\n[docs](https://example.com)");
        let out = a.frame(60, 20);
        let accent = |t: &str| a.palette.paint(Role::Info, t);
        assert!(out.contains(&format!("\x1b[1;4m{}", accent("Big"))), "h1: {out:?}");
        // The space inside the heading keeps its underline.
        assert!(out.contains(&format!("\x1b[1;4m{}", accent(" "))), "gap: {out:?}");
        assert!(out.contains(&format!("\x1b[1m{}", accent("Mid"))), "h2: {out:?}");
        assert!(out.contains(&format!("\x1b[1m{}", a.palette.paint(Role::Assistant, "Low"))), "h3: {out:?}");
        assert!(out.contains(&accent("• ")), "marker: {out:?}");
        assert!(out.contains(&format!("\x1b[3m{}", a.palette.paint(Role::Thinking, "said"))), "quote: {out:?}");
        assert!(out.contains(&format!("\x1b[4m{}", a.palette.paint(Role::Link, "docs"))), "link: {out:?}");
    }

    #[test]
    fn interface_rows_carry_emphasis() {
        let mut a = colour_app();
        let (tx, _cmds) = channel();
        a.push(Role::Thinking, "pondering");
        let out = a.frame(60, 12);
        let thinking = a.palette.paint(Role::Thinking, "~ pondering");
        assert!(out.contains(&format!("\x1b[3m{thinking}")), "thinking: {out:?}");
        let colour = a.palette.paint(Role::Thinking, "").replace("\x1b[0m", "");
        // Command picker: title bold, other rows' descriptions muted.
        a.input = "/".into();
        a.cursor = 1;
        a.sync_menu();
        let out = a.frame(80, 30);
        assert!(out.contains("\x1b[1m"), "no bold title: {out:?}");
        let Some(Pending::Pick(p)) = &a.pending else { panic!("no picker") };
        let second = &p.items[p.matches()[1]].desc;
        let head: String = second.chars().take(10).collect();
        let at = out.find(&head).expect("description not shown");
        let before = &out[..at];
        assert!(before.rfind(&colour) > before.rfind("\x1b[0m"), "description not muted: {out:?}");
        // The unseen count is accent, the rest of the status line muted.
        a.pending = None;
        a.input.clear();
        for i in 0..20 {
            a.push(Role::User, format!("line {i}"));
        }
        a.frame(60, 8);
        a.handle_key(Key::PageUp, &tx);
        a.push(Role::User, "more");
        let out = a.frame(60, 8);
        let count = format!("↑ {}", a.unseen);
        assert!(out.contains(&a.palette.paint(Role::Info, &count)), "count: {out:?}");
    }

    #[test]
    fn help_titles_are_bold_and_commands_accent() {
        let mut a = colour_app();
        let (tx, _cmds) = channel();
        a.run_command("help", &tx);
        let out = a.frame(80, 60);
        assert!(out.contains("\x1b[1mKeys\x1b[0m"), "{out:?}");
        assert!(out.contains(&a.palette.paint(Role::Info, "/compact")), "{out:?}");
        assert!(!out.contains("\x1b[36m"), "terminal cyan left in: {out:?}");
        // Without colour, /help has no escapes.
        let mut b = app();
        b.run_command("help", &tx);
        assert!(!b.frame(80, 60).contains("\x1b[1m"));
        // At Hackberry widths every help row arrives whole and flush left.
        for cols in [40, 54] {
            let mut c = colour_app();
            c.cols.store(cols as u16, Ordering::Relaxed);
            c.run_command("help", &tx);
            let want = crate::help::commands((cols - 2).max(40), true, false);
            let rows: Vec<String> = entry_lines(&c.entries[0], cols, false, false)
                .into_iter()
                .map(|(_, l)| match l {
                    Line::Spans(s) => unpaint(&s.iter().map(|s| s.text.as_str()).collect::<String>()),
                    _ => panic!("help is not raw"),
                })
                .collect();
            assert_eq!(rows.join("\n"), want.trim_end_matches('\n'), "at {cols}");
        }
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
    fn mono_keeps_bold_and_reverse_but_no_colour() {
        let mut a = colour_app();
        a.palette = a.palette.with_theme(crate::theme::by_name("mono").unwrap());
        a.push(Role::Assistant, "a **loud** word");
        let out = a.frame(60, 12);
        assert!(out.contains("\x1b[1mloud\x1b[0m"), "{out:?}");
        assert!(!out.contains("38;") && !out.contains("48;"), "colour in mono: {out:?}");
        assert_eq!(a.palette.select("row"), "\x1b[7mrow\x1b[0m");
        // NO_COLOR and `never` still mean no escapes at all.
        let mut b = app();
        b.push(Role::Assistant, "a **loud** word");
        assert!(!b.frame(60, 12).contains("\x1b[1m"));
        assert_eq!(b.palette.select("row"), "row");
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
        let row = &out[out.rfind("\r\n").expect("status row")..];
        let erase = row.find("\x1b[J").expect("nothing clears below the status");
        let text = row.find('·').expect("no status text");
        assert!(
            erase < text,
            "erase below runs after the status text: {row:?}"
        );
    }
    #[test]
    fn rows_are_written_then_erased_never_blanked_first() {
        let mut a = app();
        a.push(Role::Info, "hi");
        let out = a.frame(40, 10);
        assert!(!out.contains("\x1b[2K"), "a row is blanked before its text: {out:?}");
        assert!(out.contains("hi\x1b[K") || out.contains("hi\x1b[0m\x1b[K"), "{out:?}");
        // A full-width row is not erased: the erase would take its last cell.
        assert!(!out.contains(&format!("{}\x1b[K", "─".repeat(40))), "{out:?}");
        assert!(out.starts_with("\x1b[?2026h") && out.ends_with("\x1b[?2026l"), "{out:?}");
    }

    #[test]
    fn a_status_change_writes_one_row() {
        let mut a = app();
        a.push(Role::Assistant, "hello there");
        a.frame_changes(60, 12);
        a.status.tokens = 1234;
        let tick = a.frame_changes(60, 12);
        assert_eq!(tick.matches(";1H").count(), 1, "{tick:?}");
        assert!(!tick.contains("hello there"), "an unchanged row was written: {tick:?}");
        let same = a.frame_changes(60, 12);
        assert_eq!(same.matches(";1H").count(), 0, "{same:?}");
        let resized = a.frame_changes(61, 12);
        assert!(resized.contains("hello there") && resized.contains("\x1b[H"), "{resized:?}");
    }

    #[test]
    fn composer_draws_one_row_per_line() {
        let mut a = app();
        a.input = "line one\nline two".into();
        a.cursor = a.input.len();
        let out = a.frame(60, 12).replace("\x1b[K", "");
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
        let out = a.frame(100, 12).replace("\x1b[K", "");
        assert!(out.contains("which database?\r\n"), "{out}");
        assert!(out.contains("▌ [2] postgres"), "{out}");
        assert!(out.contains("[1] sqlite"), "{out}");
        assert!(out.contains("[e]"), "{out}");
    }

    #[test]
    fn the_active_step_is_always_visible() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.input = "go".into();
        a.cursor = 2;
        a.handle_key(Key::Enter, &tx);
        assert!(strip(&a.frame(65, 12)).contains("thinking · 0s"), "no thinking row");

        a.apply(Ev::ToolStart { name: "bash".into(), summary: "Run test suite".into(), args: String::new() });
        assert!(!strip(&a.frame(65, 12)).contains("thinking"), "thinking beside a running tool");
        let (reply, _r) = channel();
        let command = format!("python3 -m unittest {} 2>&1 | tail -5", "tests/test_cart.py ".repeat(14));
        a.apply(Ev::Confirm { what: format!("run `{command}`"), reply, always: true });
        let out = strip(&a.frame(65, 20));
        assert!(out.contains("▌ Run test suite · waiting for you"), "{out}");
        assert!(out.contains("$ python3 -m unittest"), "{out}");
        assert!(out.contains("tests/test_cart.py tests/test_cart.py"), "{out}");
        for line in out.split("\r\n") {
            assert!(line.chars().filter(|c| *c != '\u{7}').count() <= 65, "{line:?}");
        }
    }

    #[test]
    fn a_task_reports_its_meter_and_finish_line() {
        assert_eq!(
            task_line(Duration::from_secs(42), 5, 16_560, 1_440, 1_400, Some(0.004)),
            "42s · 5 steps · 18K in (92% cached) · 1.4K out · $0.004"
        );
        assert_eq!(task_line(Duration::from_secs(72), 1, 0, 0, 0, None), "1m12s · 1 step");

        let mut a = app();
        let (tx, _rx) = channel();
        a.input = "go".into();
        a.cursor = 2;
        a.handle_key(Key::Enter, &tx);
        a.apply(Ev::ToolStart { name: "bash".into(), summary: "ls".into(), args: String::new() });
        a.apply(Ev::Usage { tokens: None, hit: 900, miss: 100, output: 2100, model: None });
        let status = strip(&a.status_line(80));
        assert!(status.ends_with("0s · 1 step · ↓2.1K"), "{status}");
        a.apply(Ev::Done(None));
        let last = a.entries.last().unwrap();
        assert!(last.text.starts_with("0s · 1 step · 1K in (90% cached) · 2.1K out"), "{}", last.text);
        assert!(!strip(&a.status_line(80)).contains("step"), "the meter stayed");

        a.verbosity = Verbosity::Low;
        a.begin_task();
        a.status.streaming = true;
        let n = a.entries.len();
        a.apply(Ev::Done(None));
        assert_eq!(a.entries.len(), n, "a finish line at verbosity low");
    }

    #[test]
    fn usage_without_tokens_keeps_the_context() {
        let mut a = app();
        a.apply(Ev::Usage { tokens: Some(5000), hit: 1, miss: 2, output: 3, model: None });
        a.apply(Ev::Usage { tokens: None, hit: 10, miss: 20, output: 30, model: None });
        assert_eq!((a.status.tokens, a.status.hit, a.status.miss, a.status.output), (5000, 11, 22, 33));
    }

    #[test]
    fn a_suggestion_is_taken_with_right_and_dropped_by_typing() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.apply(Ev::Suggest("run the tests".into()));
        let shown = strip(&a.frame(60, 10));
        assert!(shown.contains(">  run the tests"), "not drawn");
        assert!(!shown.contains("run the tests  Tab"), "the key hint is back: {shown:?}");
        a.handle_key(Key::Right, &tx);
        assert_eq!(a.input, "run the tests");
        assert_eq!(a.cursor, a.input.len());

        let mut a = app();
        a.apply(Ev::Suggest("run the tests".into()));
        a.handle_key(Key::Char('n'), &tx);
        assert_eq!(a.input, "n");
        assert!(!strip(&a.frame(60, 10)).contains("run the tests"), "still drawn");

        let mut a = app();
        a.apply(Ev::Suggest("x".into()));
        a.handle_key(Key::Enter, &tx);
        assert!(_rx.try_recv().is_err(), "Enter sent the suggestion");

        let mut a = app();
        a.input = "draft".into();
        a.apply(Ev::Suggest("late".into()));
        assert!(a.status.suggestion.is_none(), "a late suggestion was kept");
    }

    #[test]
    fn long_choices_wrap_instead_of_leaving_the_screen() {
        let mut a = app();
        let (tx, _rx) = channel();
        let long = "imp's agent loop (idle → streaming → tool call → tool end → streaming again)";
        a.pending = Some(Pending::Question {
            q: "Which state machine should I chart?".into(),
            opts: vec![long.into(), "A job's lifecycle".into(), "Something else".into()],
            reply: tx,
            mode: AskMode::Choose,
            typed: String::new(),
        });
        let out = a.frame(40, 20);
        for line in out.split("\r\n") {
            assert!(strip(line).chars().count() <= 40, "{line:?}");
        }
        let text = strip(&out);
        assert!(text.contains("again)"), "the end of choice 1 was cut: {text}");
        assert!(text.contains("[3] Something else"), "{text}");
        assert!(text.contains("[e] answer"), "{text}");
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
        a.agents.clear();
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
            tokens: Some(100),
            hit: 900,
            miss: 100,
            output: 50,
            model: None,
        });
        a.apply(Ev::Usage {
            tokens: Some(200),
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
        // The model is thinking about the next step, so the thinking row spins.
        let done = a.entries[0].text.clone();
        assert!(a.spinner_due(), "the thinking row did not spin");
        a.tick_spinner();
        assert_eq!(a.entries[0].text, done, "a finished tool kept ticking");
        a.apply(Ev::Done(None));
        assert!(!a.spinner_due(), "an idle session ticked");
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
        for frame in spinner_frames().chars() {
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
    fn a_result_that_restates_the_step_is_dropped() {
        assert!(echoes("edit x.txt (1 change)", "edited x.txt (1 change)"));
        assert!(!echoes("run `ls`", "exit 0"));
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
    fn queue_adds_lists_pops_deletes_clears_and_pops_all() {
        let mut a = app();
        let (tx, _rx) = channel();
        a.run_command("queue check the tests", &tx);
        a.run_command("queue write the docs", &tx);
        a.run_command("queue", &tx);
        let listed = &a.entries.last().unwrap().text;
        assert!(listed.contains("1. check the tests"), "{listed}");
        assert!(listed.contains("2. write the docs"), "{listed}");

        a.run_command("queue pop 1", &tx);
        assert_eq!(a.input, "check the tests");
        assert_eq!(a.queued.len(), 1);

        a.run_command("queue delete 1", &tx);
        assert!(a.queued.is_empty());
        assert_eq!(a.input, "check the tests", "delete touched the composer");

        a.run_command("queue first", &tx);
        a.run_command("queue second", &tx);
        a.run_command("queue pop all", &tx);
        assert_eq!(a.input, "check the tests\n\nfirst\n\nsecond");
        assert!(a.queued.is_empty());

        a.run_command("queue pop 9", &tx);
        assert_eq!(a.entries.last().unwrap().role, Role::Error);
        a.run_command("queue one", &tx);
        a.run_command("queue clear", &tx);
        assert!(a.queued.is_empty());
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
        assert_eq!(a.verbosity, Verbosity::High, "Enter did not apply high verbosity");
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
        let short = picker_row(&picker, 0, 0, 50, false).remove(0);
        let long = picker_row(&picker, 1, 1, 50, false).remove(0);
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
            picker_row(&picker, at, at, 70, false).remove(0)
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
    fn the_highlighted_command_description_wraps() {
        let mut picker = Picker::new("commands", command_items(), PickKind::Command);
        let at = picker
            .items
            .iter()
            .position(|i| i.label == "/goal")
            .expect("/goal");
        picker.index = at;
        let lines = picker_lines(&picker, &picker.matches(), 8, 46, true);
        let row: Vec<&String> = lines
            .iter()
            .filter(|(_, i, _)| *i == at)
            .map(|(l, _, _)| l)
            .collect();
        assert_eq!(row.len(), 2, "{row:?}");
        assert!(row[1].contains("off to stop"), "description cut: {:?}", row[1]);
        assert_eq!(
            row[1].chars().take_while(|c| *c == ' ').count(),
            16,
            "no hanging indent: {:?}",
            row[1]
        );
        // Rows that are not highlighted keep one line, so only the
        // highlighted item adds a row.
        assert_eq!(lines.len(), 9, "another row wrapped: {lines:?}");
    }

    #[test]
    fn a_description_too_long_for_two_lines_ends_in_an_ellipsis() {
        let mut picker = Picker::new("commands", command_items(), PickKind::Command);
        let at = picker
            .items
            .iter()
            .position(|i| i.label == "/goal")
            .expect("/goal");
        picker.index = at;
        let row = picker_row(&picker, at, at, 38, true);
        assert_eq!(row.len(), 2, "{row:?}");
        assert!(row[1].ends_with('…'), "{:?}", row[1]);
        assert!(
            row.iter().all(|l| l.chars().count() <= 38),
            "a line is wider than the box: {row:?}"
        );
    }

    #[test]
    fn the_command_picker_shows_the_whole_description() {
        let mut a = app();
        let (tx, _cmds) = channel();
        for c in "/goal".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        let frame = strip(&a.frame(60, 20));
        assert!(frame.contains("/goal"), "no command row: {frame:?}");
        assert!(frame.contains("off to stop"), "description cut: {frame:?}");
    }

    #[test]
    fn a_short_screen_cuts_the_description_on_one_line() {
        let mut a = app();
        let mut picker = Picker::new("commands", command_items(), PickKind::Command);
        let at = picker
            .items
            .iter()
            .position(|i| i.label == "/goal")
            .expect("/goal");
        picker.index = at;
        a.pending = Some(Pending::Pick(picker));
        let frame = strip(&a.frame(60, 12));
        assert!(frame.contains("/goal"), "no command row: {frame:?}");
        assert!(!frame.contains("off to stop"), "the box grew: {frame:?}");
    }

    #[test]
    fn a_wrapped_line_carries_the_same_click_target() {
        let mut a = app();
        let (tx, _cmds) = channel();
        for c in "/goal".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        a.frame(60, 20);
        let hits: Vec<usize> = a
            .targets
            .iter()
            .filter_map(|t| match t.3 {
                Target::Row(0) => Some(t.0),
                _ => None,
            })
            .collect();
        assert_eq!(hits.len(), 2, "the second line has no target: {:?}", a.targets);
        assert_eq!(hits[1], hits[0] + 1, "{hits:?}");
    }

    #[test]
    fn a_missing_key_opens_the_provider_picker() {
        let mut a = app();
        a.begin_onboard("DEEPSEEK_API_KEY".into());
        assert!(a.onboard);
        assert_eq!(open_picker(&a).kind, PickKind::Provider);
        let frame = strip(&a.frame(80, 20));
        assert!(frame.contains("no key for deepseek (DEEPSEEK_API_KEY)"), "{frame}");
    }

    #[test]
    fn a_provider_without_a_key_starts_at_once() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.begin_onboard("DEEPSEEK_API_KEY".into());
        a.choose_onboard_provider("ollama", &tx);
        assert!(a.pending.is_none(), "a key row opened for Ollama");
        assert!(!a.onboard);
        assert_eq!(a.cfg.provider, crate::providers::Provider::Ollama);
    }

    #[test]
    fn the_first_run_key_is_masked_and_editable() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.begin_onboard("DEEPSEEK_API_KEY".into());
        a.choose_onboard_provider("deepseek", &tx);
        assert!(matches!(a.pending, Some(Pending::KeyEntry { .. })), "no key row");
        for c in "sk-ab".chars() {
            a.handle_key(Key::Char(c), &tx);
        }
        let row = |a: &mut App| {
            strip(&a.frame(80, 20))
                .lines()
                .find(|l| l.contains("key for DEEPSEEK_API_KEY"))
                .map(str::to_string)
                .expect("the key row")
        };
        let shown = row(&mut a);
        let stars = shown.matches('*').count();
        assert_eq!(stars, 5, "{shown}");
        assert!(!shown.contains("sk-ab"), "the key is drawn: {shown}");
        a.handle_key(Key::Backspace, &tx);
        assert_eq!(row(&mut a).matches('*').count(), 4, "Backspace did not remove");
    }

    #[test]
    fn esc_leaves_the_first_run() {
        let mut a = app();
        let (tx, _cmds) = channel();
        a.begin_onboard("DEEPSEEK_API_KEY".into());
        a.choose_onboard_provider("deepseek", &tx);
        a.handle_key(Key::Esc, &tx);
        assert!(a.quit, "Esc kept the first run open");
        assert!(a.pending.is_none());
    }

    #[test]
    fn a_refused_key_retries_and_a_dead_network_saves() {
        assert_eq!(first_run_outcome(Ok(())), KeyOutcome::Save(None));
        assert_eq!(
            first_run_outcome(Err("models: HTTP 401".into())),
            KeyOutcome::Retry("models: HTTP 401".into())
        );
        match first_run_outcome(Err("models: connection refused".into())) {
            KeyOutcome::Save(Some(warning)) => {
                assert!(warning.contains("check did not run"), "{warning}")
            }
            other => panic!("offline did not save: {other:?}"),
        }
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
        assert_eq!(a.verbosity, Verbosity::Medium, "default is not medium");
        let row = |a: &mut App| {
            a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `ls`".into(), args: "{}".into() });
            a.apply(Ev::ToolEnd {
                ok: true,
                preview: "exit 0".into(),
                output: "exit 0\nfoo".into(),
                detail: String::new(),
                ms: 4,
            });
            a.entries.last().unwrap().text.clone()
        };
        let medium = row(&mut a);
        assert!(medium.starts_with("✓ run `ls` → exit 0"), "{medium:?}");
        a.run_command("verbosity low", &tx);
        let low = row(&mut a);
        assert!(low.starts_with("✓ run `ls`") && !low.contains('→'), "{low:?}");
        a.entries.clear();
        a.run_command("verbosity high", &tx);
        assert_eq!(a.verbosity, Verbosity::High);

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
        assert!(text.contains("\n  $ ls"), "{text:?}");
        assert!(text.contains("foo"), "{text:?}");

        a.run_command("verbosity low", &tx);
        assert_eq!(a.verbosity, Verbosity::Low);
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
        assert_eq!(a.verbosity, Verbosity::Medium, "choosing medium did nothing");
        assert!(a.pending.is_none(), "the picker stayed open");
        a.run_command("verbosity loud", &tx);
        assert!(a.entries.last().unwrap().text.contains("use low, medium, or high"));
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
        // A command that ran and exited 1 is a result, not a breakage.
        assert_eq!(a.entries[0].role, Role::Tool);
        assert!(a.entries[0].text.starts_with("! "), "{}", a.entries[0].text);
        assert!(a.entries[0].text.contains("run `false`"));
        assert!(a.entries[0].text.contains("ms"));
    }

    #[test]
    fn ctrl_e_toggles_every_row_of_the_latest_turn() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let step = |a: &mut App, cmd: &str, ok: bool, output: &str| {
            a.apply(Ev::ToolStart { name: "bash".into(), summary: format!("run `{cmd}`"), args: format!("{{\"command\":\"{cmd}\"}}") });
            a.apply(Ev::ToolEnd { ok, preview: String::new(), output: output.into(), detail: String::new(), ms: 1 });
        };
        a.begin_task();
        step(&mut a, "false", false, "refused: outside the session");
        step(&mut a, "true", true, "exit 0");
        a.handle_key(Key::Ctrl('e'), &tx);
        let expanded = a.entries.iter().filter(|e| e.text.contains("\n  $ ")).count();
        assert_eq!(expanded, 2, "both rows should expand");
        a.handle_key(Key::Ctrl('e'), &tx);
        assert!(!a.entries.iter().any(|e| e.text.contains("\n  $ ")), "they should collapse");
        // A new turn leaves the old rows behind.
        a.begin_task();
        step(&mut a, "ls", true, "exit 0");
        a.handle_key(Key::Ctrl('e'), &tx);
        assert_eq!(a.entries.iter().filter(|e| e.text.contains("\n  $ ls")).count(), 1);
        // Clearing the screen drops the rows, so Ctrl+E edits nothing stale.
        a.handle_key(Key::Ctrl('l'), &tx);
        a.push(Role::Assistant, "fresh");
        a.handle_key(Key::Ctrl('e'), &tx);
        assert_eq!(a.entries[0].text, "fresh");
    }

    #[test]
    fn expanded_arguments_read_as_a_person_would() {
        assert_eq!(readable_args(r#"{"command":"python3 calc.py","description":"Run calc.py"}"#), vec!["$ python3 calc.py"]);
        assert_eq!(
            readable_args(r#"{"path":"calc.py","old":"a - b","new":"a + b","all":false}"#),
            vec!["path: calc.py", "all: false", "new: a + b", "old: a - b"]
        );
        assert_eq!(readable_args("not json"), vec!["not json"]);
        let mut a = app();
        let (tx, _cmds) = channel();
        a.apply(Ev::ToolStart { name: "bash".into(), summary: "Run calc.py".into(), args: r#"{"command":"python3 calc.py"}"#.into() });
        a.apply(Ev::ToolEnd { ok: true, preview: String::new(), output: "exit 0\n5".into(), detail: String::new(), ms: 1 });
        a.handle_key(Key::Ctrl('e'), &tx);
        let out = strip(&a.frame(60, 10));
        assert!(out.contains("    $ python3 calc.py") && !out.contains("{\""), "{out:?}");
        // A long argument is cut like long output.
        let body: String = (1..=50).map(|i| format!("line {i}\n")).collect();
        let args = serde_json::json!({"path": "big.txt", "content": body}).to_string();
        a.apply(Ev::ToolStart { name: "write".into(), summary: "write big.txt".into(), args });
        a.apply(Ev::ToolEnd { ok: true, preview: String::new(), output: "wrote".into(), detail: String::new(), ms: 1 });
        a.handle_key(Key::Ctrl('e'), &tx);
        let text = &a.entries.last().unwrap().text;
        assert!(text.contains("line 19") && !text.contains("line 20\n") && text.contains("… +31 lines"), "{text}");
    }

    #[test]
    fn a_long_directory_shortens_in_steps() {
        let p = "~/Projects/imp/src/providers";
        assert_eq!(fit_path(p, 40).as_deref(), Some(p));
        assert_eq!(fit_path(p, 20).as_deref(), Some("~/P/i/s/providers"));
        assert_eq!(fit_path(p, 12).as_deref(), Some("…/providers"));
        assert_eq!(fit_path(p, 9).as_deref(), Some("providers"));
        assert_eq!(fit_path(p, 8).as_deref(), Some("…oviders"));
        assert_eq!(fit_path(p, 7), None, "below the floor it is left out");
        assert_eq!(fit_path("~/.config/imp/agents", 16).as_deref(), Some("~/.c/i/agents"));
        assert_eq!(fit_path("/private/tmp/claude/demo", 16).as_deref(), Some("/p/t/c/demo"));
        assert_eq!(fit_path("~/averyveryverylongname", 10).as_deref(), Some("…ylongname"));
    }

    #[test]
    fn diff_bands_reach_the_edge() {
        let mut a = colour_app();
        a.push_diff("+short line".into());
        let out = a.frame(54, 8);
        let row = out.split("\r\n").find(|r| r.contains("+short line")).expect("diff row");
        let band = row.split_once("+short line").unwrap().1;
        let pad = band.split('\x1b').next().unwrap();
        assert_eq!(2 + "+short line".len() + pad.len(), 54, "{row:?}");
        assert!(row.starts_with("  \x1b[48;2;"), "the indent must stay clear: {row:?}");
        // Without colour the row keeps its sign and no padding.
        let mut b = app();
        b.push_diff("+short line".into());
        let plain = b.frame(54, 8);
        assert!(plain.contains("  +short line\x1b[K") || plain.contains("  +short line\r"), "{plain:?}");
    }

    #[test]
    fn esc_on_a_prompt_stops_the_turn_but_n_only_declines() {
        let (tx, _cmds) = channel();
        let mut a = app();
        let (reply, answers) = channel();
        a.status.streaming = true;
        a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
        a.handle_key(Key::Char('n'), &tx);
        assert_eq!(answers.try_recv().ok(), Some(Answer::No));
        assert!(!a.cancel.load(Ordering::Relaxed), "n stopped the turn");
        let (reply, answers) = channel();
        a.apply(Ev::Confirm { what: "run `make`".into(), reply, always: false });
        a.handle_key(Key::Esc, &tx);
        assert_eq!(answers.try_recv().ok(), Some(Answer::No));
        assert!(a.cancel.load(Ordering::Relaxed), "Esc did not stop the turn");
        assert!(a.pending.is_none() && !a.quit);
        // A question stops the same way.
        let mut b = app();
        let (reply, answers) = channel();
        b.apply(Ev::Question { q: "which?".into(), opts: vec!["a".into(), "b".into(), "c".into()], reply });
        b.handle_key(Key::Esc, &tx);
        assert_eq!(answers.try_recv().ok(), Some(None));
        assert!(b.cancel.load(Ordering::Relaxed), "Esc on a question did not stop the turn");
    }

    #[test]
    fn a_decline_shows_once_on_its_row() {
        let mut a = app();
        let (tx, _cmds) = channel();
        let (reply, answers) = channel();
        a.apply(Ev::ToolStart { name: "bash".into(), summary: "Delete README.md".into(), args: "{}".into() });
        a.apply(Ev::Confirm { what: "run `rm README.md`".into(), reply, always: false });
        a.handle_key(Key::Char('n'), &tx);
        assert_eq!(answers.try_recv().ok(), Some(Answer::No));
        let out = "refused: the user declined to run this command";
        a.apply(Ev::ToolEnd { ok: false, preview: out.into(), output: out.into(), detail: String::new(), ms: 1 });
        let screen = strip(&a.frame(60, 10));
        assert_eq!(screen.matches("declined").count(), 1, "{screen:?}");
        assert!(screen.contains("✗ Delete README.md") && screen.contains("  declined"), "{screen:?}");
        assert!(!screen.contains("refused:"), "{screen:?}");
        assert_eq!(a.entries.len(), 1);
    }

    #[test]
    fn a_click_toggles_one_tool_row() {
        let mut a = app();
        a.cfg.mouse = true;
        let (tx, _cmds) = channel();
        a.begin_task();
        for cmd in ["false", "true"] {
            let args = format!("{{\"command\":\"{cmd}\"}}");
            a.apply(Ev::ToolStart { name: "bash".into(), summary: format!("run `{cmd}`"), args });
            a.apply(Ev::ToolEnd { ok: false, preview: String::new(), output: "exit 1".into(), detail: String::new(), ms: 1 });
        }
        a.frame(80, 14);
        click(&mut a, &tx, |t| matches!(t, Target::Tool(0)));
        assert!(a.entries[0].text.contains("$ false"), "the clicked row did not open");
        assert!(!a.entries[1].text.contains("$ true"), "the other row opened too");
        a.frame(80, 14);
        click(&mut a, &tx, |t| matches!(t, Target::Tool(1)));
        assert!(a.entries[1].text.contains("$ true"), "the newest row did not open");
        a.frame(80, 20);
        click(&mut a, &tx, |t| matches!(t, Target::Tool(0)));
        assert!(!a.entries[0].text.contains("$ false"), "a second click did not close it");
        // A picker over the rows takes the click; no row opens beneath it.
        a.handle_key(Key::Ctrl('e'), &tx);
        a.handle_key(Key::Ctrl('e'), &tx);
        assert!(!a.entries.iter().any(|e| e.text.contains("\n  $ ")));
        // Enough rows that the picker sits over them.
        for i in 0..12 {
            let args = format!("{{\"command\":\"step{i}\"}}");
            a.apply(Ev::ToolStart { name: "bash".into(), summary: format!("step {i}"), args });
            a.apply(Ev::ToolEnd { ok: true, preview: String::new(), output: "exit 0".into(), detail: String::new(), ms: 1 });
        }
        open_many(&mut a, 3);
        a.frame(80, 16);
        click(&mut a, &tx, |t| matches!(t, Target::Row(1)));
        assert!(!a.entries.iter().any(|e| e.text.contains("\n  $ ")), "a tool row opened under the picker");
        assert!(!matches!(a.pending, Some(Pending::Pick(_))), "the picker did not take the click");
        // With the mouse off there is nothing to click.
        a.cfg.mouse = false;
        a.frame(80, 20);
        assert!(!a.targets.iter().any(|t| matches!(t.3, Target::Tool(_))));
    }

    #[test]
    fn an_exit_shows_the_tail_of_its_output() {
        let mut a = app();
        let end = |a: &mut App, output: &str| {
            a.entries.clear();
            a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `make`".into(), args: "{}".into() });
            a.apply(Ev::ToolEnd { ok: false, preview: String::new(), output: output.into(), detail: String::new(), ms: 1 });
            strip(&a.frame(54, 14))
        };
        let short = end(&mut a, "exit 2\nmake: *** No rule\nto make target");
        assert!(short.contains("  exit 2\r\n  make: *** No rule\r\n  to make target"), "{short:?}");
        assert!(!short.contains("more above"), "{short:?}");
        let ten: String = (1..=10).map(|i| format!("\nline {i}")).collect();
        let long = end(&mut a, &format!("exit 1{ten}"));
        assert!(long.contains("  exit 1 · 7 more above (Ctrl+E)\r\n  line 8\r\n  line 9\r\n  line 10"), "{long:?}");
        assert!(!long.contains("line 7"), "{long:?}");
        assert!(long.lines().all(|l| l.trim_end_matches('\r').chars().count() <= 54));
    }

    #[test]
    fn each_finish_has_its_mark_and_colour() {
        assert_eq!(Finish::of(true, "exit 0\nhi"), Finish::Done);
        assert_eq!(Finish::of(false, "exit 1\nno tests"), Finish::Exited);
        assert_eq!(Finish::of(false, "exit -1\ntimed out after 30s"), Finish::Failed);
        assert_eq!(Finish::of(false, "refused: outside the session"), Finish::Failed);
        assert_eq!(Finish::of(false, "refused: the user declined to run this command"), Finish::Declined);
        let mut a = colour_app();
        for (ok, output, mark, tone) in [
            (false, "exit 1", "!", Role::Info),
            (false, "refused: the user declined to run this command", "✗", Role::Context),
            (false, "refused: outside the session", "✗", Role::Error),
            (true, "exit 0", "✓", Role::Success),
        ] {
            a.entries.clear();
            a.apply(Ev::ToolStart { name: "bash".into(), summary: "run `x`".into(), args: "{}".into() });
            a.apply(Ev::ToolEnd { ok, preview: String::new(), output: output.into(), detail: String::new(), ms: 1 });
            let out = a.frame(60, 10);
            let want = format!("\x1b[1m{}", a.palette.paint(tone, mark));
            assert!(out.contains(&want), "{output}: {out:?}");
        }
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
        send_replay(&tx, &history, std::path::Path::new("/"));
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
        assert!(last.text.contains("low, medium, or high"), "{}", last.text);
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
        let (_, x, y, _) = a.overlay(100, 30).expect("overlay");
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
        let before = a.frame(60, 8).replace("\x1b[K", "");
        a.push(Role::User, "brand new line");
        let after = a.frame(60, 8).replace("\x1b[K", "");
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
        // An open prompt moves nothing, so the loop sleeps as when idle.
        let (reply, _answers) = channel();
        a.apply(Ev::Confirm { what: "run `ls`".into(), reply, always: false });
        assert_eq!(a.loop_wait(quiet), Duration::from_secs(5));
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
