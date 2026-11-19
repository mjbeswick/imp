//! Line mode end to end, against a provider that cannot be reached.

use std::io::Write;
use std::process::{Command, Stdio};

fn run(input: &str) -> (String, String) {
    let home = std::env::temp_dir().join(format!("imp-line-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_imp"))
        .args(["--cli", "--provider", "custom"])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("IMP_API_KEY", "unused")
        .env("NO_COLOR", "1")
        // Port 9 is discard: any request fails at once.
        .env("IMP_BASE_URL", "http://127.0.0.1:9")
        .current_dir(&home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_bang_line_runs_locally_and_sends_nothing() {
    let (out, err) = run("!echo imp-local-ok\n/quit\n");
    assert!(out.contains("$ echo imp-local-ok"), "{out}");
    assert!(out.contains("  imp-local-ok"), "{out}");
    assert!(!err.contains("127.0.0.1"), "a request was attempted: {err}");
    assert!(!out.contains("▸"), "the model ran it as a tool: {out}");
}

#[test]
fn an_unknown_option_points_to_help() {
    let out = Command::new(env!("CARGO_BIN_EXE_imp"))
        .arg("--bogus")
        .env("HOME", std::env::temp_dir())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown option --bogus; try imp --help"), "{err}");
    assert!(!out.status.success());
}

#[test]
fn a_bang_line_on_the_deny_list_is_refused() {
    // `mkfs` is deny-listed; `echo` keeps the probe harmless if it ran.
    let (out, _) = run("!echo mkfs-probe\n/quit\n");
    assert!(out.contains("refused"), "{out}");
    assert!(!out.contains("  mkfs-probe"), "it ran: {out}");
}
