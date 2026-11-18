//! Terminal handling is only real on a terminal.
//!
//! This test puts the TUI on a pty, quits it, and checks that the input mode
//! afterwards matches the mode before it started.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct Termios {
    iflag: u64,
    oflag: u64,
    cflag: u64,
    lflag: u64,
    cc: [u8; 20],
    ispeed: u64,
    ospeed: u64,
}

impl std::fmt::Debug for Termios {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "iflag={:#x} oflag={:#x} cflag={:#x} lflag={:#x}",
            self.iflag, self.oflag, self.cflag, self.lflag
        )
    }
}

#[repr(C)]
struct Winsize {
    rows: u16,
    cols: u16,
    x: u16,
    y: u16,
}

#[cfg_attr(target_os = "linux", link(name = "util"))]
extern "C" {
    fn openpty(
        master: *mut i32,
        slave: *mut i32,
        name: *mut u8,
        termp: *const Termios,
        winp: *const Winsize,
    ) -> i32;
    fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, action: i32, termios: *const Termios) -> i32;
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
}

const TCSANOW: i32 = 0;
const ICRNL: u64 = 0x100;

#[cfg(target_os = "macos")]
const O_NONBLOCK: i32 = 0x0004;
#[cfg(not(target_os = "macos"))]
const O_NONBLOCK: i32 = 0x0800;

#[cfg(target_os = "macos")]
const TIOCSWINSZ: u64 = 0x8008_7467;
#[cfg(not(target_os = "macos"))]
const TIOCSWINSZ: u64 = 0x5414;

fn open_pty() -> (File, OwnedFd) {
    let mut master = -1;
    let mut slave = -1;
    let size = Winsize {
        rows: 24,
        cols: 80,
        x: 0,
        y: 0,
    };
    let ok = unsafe {
        openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
        )
    };
    assert_eq!(ok, 0, "openpty failed");
    unsafe {
        let _ = ioctl(slave, TIOCSWINSZ, &size as *const Winsize);
        let slave = OwnedFd::from_raw_fd(slave);
        let master = File::from_raw_fd(master);
        (master, slave)
    }
}

fn mode(fd: &OwnedFd) -> Termios {
    let mut t = Termios {
        iflag: 0,
        oflag: 0,
        cflag: 0,
        lflag: 0,
        cc: [0; 20],
        ispeed: 0,
        ospeed: 0,
    };
    let ok = unsafe { tcgetattr(std::os::fd::AsRawFd::as_raw_fd(fd), &mut t) };
    assert_eq!(ok, 0, "tcgetattr failed");
    t
}

fn set_mode(fd: &OwnedFd, t: &Termios) {
    let ok = unsafe { tcsetattr(std::os::fd::AsRawFd::as_raw_fd(fd), TCSANOW, t) };
    assert_eq!(ok, 0, "tcsetattr failed");
}

/// A terminal that lost its CR-to-NL mapping must still submit on Enter.
#[test]
fn line_mode_submits_on_a_bare_carriage_return() {
    let (mut master, slave) = open_pty();
    let mut broken = mode(&slave);
    broken.iflag &= !ICRNL;
    set_mode(&slave, &broken);
    let before = mode(&slave);

    let dir = std::env::temp_dir().join(format!("imp-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut child = Command::new(env!("CARGO_BIN_EXE_imp"))
        .arg("--cli")
        .env("DEEPSEEK_API_KEY", "test-key")
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .stdin(Stdio::from(slave.try_clone().expect("clone slave")))
        .stdout(Stdio::from(slave.try_clone().expect("clone slave")))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn imp");

    let ready = read_until(&mut master, b"> ", Duration::from_secs(5));
    master.write_all(b"/quit\r").expect("send quit");
    master.flush().ok();

    let start = Instant::now();
    let mut exited = false;
    while start.elapsed() < Duration::from_secs(5) {
        if child.try_wait().expect("wait").is_some() {
            exited = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();

    let after = mode(&slave);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(ready, "line mode never showed a prompt");
    assert!(exited, "a bare carriage return did not submit the command");
    assert_eq!(
        stable(before),
        stable(after),
        "line mode left the terminal in a different mode"
    );
}

/// How the TUI is asked to stop.
#[derive(Clone, Copy)]
enum Stop {
    Quit,
    Panic,
    Terminate,
}

fn probe(stop: Stop) -> (Termios, Termios, Termios) {
    let (mut master, slave) = open_pty();
    let before = mode(&slave);

    let dir = std::env::temp_dir().join(format!(
        "imp-tty-{}-{}",
        std::process::id(),
        match stop {
            Stop::Quit => "quit",
            Stop::Panic => "panic",
            Stop::Terminate => "term",
        }
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut command = Command::new(env!("CARGO_BIN_EXE_imp"));
    command
        .env("DEEPSEEK_API_KEY", "test-key")
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env_remove("IMP_FORCE_PANIC")
        .stdin(Stdio::from(slave.try_clone().expect("clone slave")))
        .stdout(Stdio::from(slave.try_clone().expect("clone slave")))
        .stderr(Stdio::null());
    if matches!(stop, Stop::Panic) {
        command.env("IMP_FORCE_PANIC", "1");
    }
    let mut child = command.spawn().expect("spawn imp");

    let seen = read_until(&mut master, b"\x1b[?1049h", Duration::from_secs(5));
    let during = mode(&slave);
    match stop {
        Stop::Quit | Stop::Panic => {
            master.write_all(b"\x04").expect("send quit");
            master.flush().ok();
        }
        Stop::Terminate => {
            let _ = Command::new("kill")
                .args(["-TERM", &child.id().to_string()])
                .status();
        }
    }

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if child.try_wait().expect("wait").is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();

    let after = mode(&slave);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        seen,
        "the TUI never took the screen, so it did not run: {during:?}"
    );
    assert_ne!(before, during, "the TUI did not change the terminal mode");
    (before, during, after)
}

/// `PENDIN` is kernel bookkeeping for pending input, not a mode imp sets.
const PENDIN: u64 = 0x2000_0000;

fn stable(mut t: Termios) -> Termios {
    t.lflag &= !PENDIN;
    t
}

fn assert_restored(stop: Stop) {
    let (before, _, after) = probe(stop);
    assert_eq!(
        stable(before),
        stable(after),
        "the TUI left the terminal in a different mode"
    );
}

/// The terminal mode the TUI leaves behind must match the one it found.
#[test]
fn tui_restores_the_terminal_on_exit() {
    assert_restored(Stop::Quit);
}

#[test]
fn tui_restores_the_terminal_on_panic() {
    assert_restored(Stop::Panic);
}

#[test]
fn tui_restores_the_terminal_on_sigterm() {
    assert_restored(Stop::Terminate);
}

fn read_until(master: &mut File, needle: &[u8], limit: Duration) -> bool {
    let fd = std::os::fd::AsRawFd::as_raw_fd(master);
    unsafe {
        let flags = fcntl(fd, 3);
        fcntl(fd, 4, flags | O_NONBLOCK);
    }
    let mut seen = Vec::new();
    let start = Instant::now();
    let mut buf = [0u8; 4096];
    while start.elapsed() < limit {
        match master.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                seen.extend_from_slice(&buf[..n]);
                if seen.windows(needle.len()).any(|w| w == needle) {
                    return true;
                }
            }
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    false
}
