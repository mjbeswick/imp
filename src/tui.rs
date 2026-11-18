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

static SAVED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
const RESTORE: &str = "\x1b[?25h\x1b[?1049l";

extern "C" {
    fn write(fd: std::ffi::c_int, buf: *const u8, len: usize) -> isize;
    fn _exit(code: std::ffi::c_int) -> !;
    fn signal(sig: std::ffi::c_int, handler: usize) -> usize;
}

extern "C" fn on_signal(_: std::ffi::c_int) {
    unsafe {
        write(1, RESTORE.as_ptr(), RESTORE.len());
        _exit(1);
    }
}

/// Restore on panic and on SIGTERM/SIGINT, not only on a clean drop.
fn install_guards() {
    std::panic::set_hook(Box::new(|info| {
        unsafe {
            write(1, RESTORE.as_ptr(), RESTORE.len());
        }
        if let Ok(saved) = SAVED.lock() {
            if let Some(s) = &*saved {
                let _ = std::process::Command::new("stty").arg(s).status();
            }
        }
        eprintln!("{info}");
    }));
    unsafe {
        signal(15, on_signal as *const () as usize);
        signal(2, on_signal as *const () as usize);
    }
}

/// Raw mode through `stty`, which is more portable than a hand-declared termios.
struct RawMode {
    saved: String,
}

impl RawMode {
    fn enter() -> Self {
        let saved = std::process::Command::new("stty")
            .arg("-g")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let _ = std::process::Command::new("stty")
            .args(["raw", "-echo"])
            .status();
        if let Ok(mut slot) = SAVED.lock() {
            *slot = Some(saved.clone());
        }
        print!("{ALT_ON}");
        std::io::stdout().flush().ok();
        Self { saved }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if !self.saved.is_empty() {
            let _ = std::process::Command::new("stty").arg(&self.saved).status();
        }
        print!("{ALT_OFF}");
        std::io::stdout().flush().ok();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
            if buf.len() >= 3 && buf[1] == b'[' {
                let k = match buf[2] {
                    b'A' => Key::Up,
                    b'B' => Key::Down,
                    b'C' => Key::Right,
                    b'D' => Key::Left,
                    b'H' => Key::Home,
                    b'F' => Key::End,
                    b'Z' => Key::BackTab,
                    _ => Key::Unknown,
                };
                buf.drain(..3);
                keys.push(k);
                continue;
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
    Cancel,
    SetThinking(bool),
    SetAgent(String),
    Compact,
    Resume(String),
    Reload,
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
    Usage { tokens: u64, hit: u64, miss: u64 },
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
}

enum Pending {
    Choose { title: String, options: Vec<String>, index: usize, action: ChooseKind },
    Confirm { what: String, reply: Sender<bool> },
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
    streaming: bool,
}

const COMMANDS: &[(&str, &str)] = crate::help::COMMANDS;

impl App {
    pub fn new(cfg: Config) -> Self {
        let palette = Palette::detect(&cfg);
        let history = load_history();
        let root = std::env::current_dir().unwrap_or_default();
        let agents = crate::store::load_agents(&cfg, &root)
            .into_iter()
            .map(|a| a.name)
            .collect();
        let sessions_dir = crate::config::dirs_state().join("mote").join("sessions");
        Self {
            cfg,
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
                streaming: false,
            },
            agents,
            sessions_dir,
            last_redraw: Instant::now() - Duration::from_secs(1),
            dirty: true,
            quit: false,
            spinner: 0,
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
            Ev::ToolStart { name, args } => self.push(Role::Tool, format!("{name}  {args}")),
            Ev::ToolEnd { ok, preview } => {
                self.push(Role::Tool, format!("  {} {preview}", if ok { "✓" } else { "✗" }))
            }
            Ev::Info(t) => self.push(Role::Info, t),
            Ev::Error(t) => self.push(Role::Error, t),
            Ev::Usage { tokens, hit, miss } => {
                self.status.tokens = tokens;
                self.status.hit = hit;
                self.status.miss = miss;
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
                    let _ = tx.send(Cmd::Cancel);
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
                    let _ = tx.send(Cmd::Cancel);
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
            crate::config::dirs_state().join("mote").join("out"),
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
                let mut options: Vec<String> = crate::store::Log::list(&self.sessions_dir)
                    .into_iter()
                    .map(|m| {
                        let title = if m.title.is_empty() { "(untitled)" } else { &m.title };
                        format!("{}  {title}  {} msgs", m.id, m.messages)
                    })
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
            screen.push_str("\x1b[2K\n");
        }
        for (role, text) in visible.iter().take(body_rows) {
            screen.push_str("\x1b[2K");
            screen.push_str(&self.palette.paint(*role, &fit(text, cols)));
            screen.push('\n');
        }

        screen.push_str("\x1b[2K");
        screen.push_str(&self.input_line(cols));
        screen.push('\n');

        for (i, (name, desc)) in menu.iter().take(6).enumerate() {
            let line = fit(&format!(" {name:<10} {desc}"), cols);
            let painted = if i == self.menu_index {
                self.palette.paint(Role::User, &line)
            } else {
                line
            };
            screen.push_str("\x1b[2K");
            screen.push_str(&painted);
            screen.push('\n');
        }

        screen.push_str("\x1b[2K");
        screen.push_str(&self.palette.paint(Role::Thinking, &self.status_line(cols)));
        screen.push_str("\x1b[J\x1b[?7h");

        let mut out = std::io::stdout();
        out.write_all(screen.as_bytes()).ok();
        out.flush().ok();
        self.dirty = false;
        self.last_redraw = Instant::now();
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
        fit(
            &format!(
                " {spin} {} · {} tok · cache {hit_rate} · {}{thinking}",
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
    crate::config::dirs_state().join("mote").join("history")
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
    if std::env::var_os("MOTE_FORCE_PANIC").is_some() {
        panic!("forced panic for the terminal-guard test");
    }
    let mut app = App::new(cfg);

    let (cmd_tx, cmd_rx) = channel::<Cmd>();
    let (ev_tx, ev_rx) = channel::<Ev>();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let sessions_dir = crate::config::dirs_state().join("mote").join("sessions");
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
                        hit: 0,
                        miss: 0,
                    });
                    let _ = ev_tx.send(Ev::Done(result.err()));
                }
                Ok(Cmd::Cancel) => cancel.store(true, Ordering::Relaxed),
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
    fn wrap_respects_width() {
        let lines = wrap("alpha beta gamma delta", 10);
        assert!(lines.iter().all(|l| l.chars().count() <= 10), "{lines:?}");
        assert!(lines.len() > 1);
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
