//! The file and shell tools. Every one has a hard cap.

use std::collections::HashSet;
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const INLINE_CAP: usize = 4 * 1024;
const CAPTURE_CAP: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_WRITE: u64 = 5 * 1024 * 1024;
const READ_LINES: usize = 2000;
const READ_BYTES: usize = 200 * 1024;

extern "C" {
    fn setpgid(pid: c_int, pgid: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

const SIGTERM: c_int = 15;
const SIGKILL: c_int = 9;

#[derive(Debug, Clone)]
pub struct Outcome {
    pub ok: bool,
    pub text: String,
    pub code: Option<i32>,
}

impl Outcome {
    fn error(text: impl Into<String>) -> Self {
        Self {
            ok: false,
            text: text.into(),
            code: None,
        }
    }
}

pub struct Tools {
    pub root: PathBuf,
    pub out_dir: PathBuf,
    pub jail: bool,
    pub timeout: Duration,
    read_paths: HashSet<PathBuf>,
    seq: u64,
}

impl Tools {
    pub fn new(root: PathBuf, out_dir: PathBuf) -> Self {
        Self {
            root,
            out_dir,
            jail: true,
            timeout: TIMEOUT,
            read_paths: HashSet::new(),
            seq: 0,
        }
    }

    /// Resolve a path and refuse anything that leaves the root.
    pub fn resolve(&self, raw: &str) -> Result<PathBuf, String> {
        use std::path::Component;

        let given = PathBuf::from(raw);
        let joined = if given.is_absolute() {
            given
        } else {
            self.root.join(given)
        };

        // Lexical normalisation first, so `..` cannot hide behind a missing dir.
        let mut norm = PathBuf::new();
        for c in joined.components() {
            match c {
                Component::ParentDir => {
                    norm.pop();
                }
                Component::CurDir => {}
                other => norm.push(other.as_os_str()),
            }
        }

        // Canonicalise the deepest existing ancestor, then re-append the rest.
        let mut probe = norm.clone();
        let mut suffix: Vec<std::ffi::OsString> = Vec::new();
        while !probe.exists() {
            let Some(name) = probe.file_name().map(|n| n.to_os_string()) else {
                break;
            };
            suffix.push(name);
            if !probe.pop() {
                break;
            }
        }
        let real = probe
            .canonicalize()
            .map_err(|e| format!("{}: {e}", probe.display()))?;
        let mut full = real;
        for part in suffix.iter().rev() {
            full.push(part);
        }

        if !self.confined(&full) {
            return Err(format!("{raw}: outside the session directory"));
        }
        Ok(full)
    }

    /// The jail allows the session directory and imp's own configuration
    /// directory, where skills and agents live. The config file itself is
    /// still guarded by the self-config rule.
    fn confined(&self, full: &std::path::Path) -> bool {
        if !self.jail {
            return true;
        }
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        if full.starts_with(&root) {
            return true;
        }
        let config = crate::config::dirs_config().join("imp");
        let config = config.canonicalize().unwrap_or(config);
        full.starts_with(&config)
    }

    fn spill(&mut self, full: &str) -> Option<PathBuf> {
        self.seq += 1;
        let _ = std::fs::create_dir_all(&self.out_dir);
        let path = self.out_dir.join(format!("out-{}-{}", std::process::id(), self.seq));
        std::fs::write(&path, full).ok().map(|_| path)
    }

    /// Cap a result, spilling the rest to disk and pointing at it.
    pub fn cap(&mut self, text: &str) -> String {
        if text.len() <= INLINE_CAP {
            return text.to_string();
        }
        let head: String = text.chars().take(INLINE_CAP / 2).collect();
        let tail: String = text
            .chars()
            .rev()
            .take(INLINE_CAP / 4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let path = self.spill(text);
        match path {
            Some(p) => format!(
                "{head}\n... [{} bytes total, elided; full output: {}]\n{tail}",
                text.len(),
                p.display()
            ),
            None => format!(
                "{head}\n... [{} bytes total, elided; spill failed]\n{tail}",
                text.len()
            ),
        }
    }

    pub fn bash(&mut self, command: &str, cancel: &AtomicBool) -> Outcome {
        let start = Instant::now();
        if cancel.load(Ordering::Relaxed) {
            return Outcome::error("cancelled");
        }

        let script = format!("exec 2>&1\n{command}");
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                setpgid(0, 0);
                Ok(())
            });
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return Outcome::error(format!("could not start: {e}")),
        };
        let pid = child.id() as c_int;

        let mut stdout = child.stdout.take().expect("piped");
        let reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 8192];
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if buf.len() < CAPTURE_CAP {
                            buf.extend_from_slice(&chunk[..n]);
                        }
                    }
                }
            }
            buf
        });

        let mut killed = false;
        let status = 'outer: loop {
            match child.try_wait() {
                Ok(Some(s)) => break Some(s),
                Ok(None) => {}
                Err(e) => return Outcome::error(format!("wait failed: {e}")),
            }
            if cancel.load(Ordering::Relaxed) || start.elapsed() > self.timeout {
                killed = true;
                unsafe {
                    kill(-pid, SIGTERM);
                }
                let grace = Instant::now();
                loop {
                    if let Ok(Some(s)) = child.try_wait() {
                        break 'outer Some(s);
                    }
                    if grace.elapsed() > Duration::from_secs(2) {
                        unsafe {
                            kill(-pid, SIGKILL);
                        }
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                break child.try_wait().ok().flatten();
            }
            std::thread::sleep(Duration::from_millis(10));
        };

        let raw = reader.join().unwrap_or_default();
        let text = String::from_utf8_lossy(&raw).to_string();

        if killed {
            let why = if cancel.load(Ordering::Relaxed) {
                "cancelled".to_string()
            } else {
                format!("timed out after {}s", self.timeout.as_secs())
            };
            return Outcome {
                ok: false,
                text: format!("{why}\n{}", self.cap(&text)),
                code: None,
            };
        }

        let code = status.and_then(|s| s.code());
        Outcome {
            ok: code == Some(0),
            text: self.cap(&text),
            code,
        }
    }

    pub fn read(&mut self, paths: &[String], offset: Option<usize>, limit: Option<usize>) -> Outcome {
        let limit = limit.unwrap_or(READ_LINES).min(READ_LINES);
        let offset = offset.unwrap_or(0);
        let mut out = String::new();
        let mut ok = true;

        for raw in paths {
            let path = match self.resolve(raw) {
                Ok(p) => p,
                Err(e) => {
                    ok = false;
                    out.push_str(&format!("== {raw} ==\n{e}\n"));
                    continue;
                }
            };
            if path.is_dir() {
                ok = false;
                out.push_str(&format!("== {raw} ==\nis a directory\n"));
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    ok = false;
                    out.push_str(&format!("== {raw} ==\n{e}\n"));
                    continue;
                }
            };
            let text = match String::from_utf8(bytes) {
                Ok(t) => t,
                Err(_) => {
                    ok = false;
                    out.push_str(&format!("== {raw} ==\nnot valid UTF-8\n"));
                    continue;
                }
            };
            self.read_paths.insert(path.clone());
            out.push_str(&format!("== {raw} ==\n"));
            let mut used = 0usize;
            let mut shown = 0usize;
            for (n, line) in text.lines().enumerate().skip(offset) {
                if shown >= limit || used > READ_BYTES {
                    out.push_str(&format!(
                        "... [stopped at line {}; {} lines total]\n",
                        n,
                        text.lines().count()
                    ));
                    break;
                }
                let numbered = format!("{:>6}: {line}\n", n + 1);
                used += numbered.len();
                shown += 1;
                out.push_str(&numbered);
            }
        }

        Outcome {
            ok,
            text: self.cap(&out),
            code: None,
        }
    }

    pub fn write(&mut self, raw: &str, content: &str) -> Outcome {
        if content.len() as u64 > MAX_WRITE {
            return Outcome::error(format!("refused: {} bytes is over the 5 MB limit", content.len()));
        }
        let path = match self.resolve(raw) {
            Ok(p) => p,
            Err(e) => return Outcome::error(e),
        };
        let existed = std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false);
        if existed && !self.read_paths.contains(&path) {
            return Outcome::error(format!("refused: read {raw} before overwriting it"));
        }
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return Outcome::error(format!("{}: {e}", parent.display()));
            }
        }
        match std::fs::write(&path, content) {
            Ok(()) => {
                self.read_paths.insert(path);
                Outcome {
                    ok: true,
                    text: format!("wrote {raw} ({} bytes)", content.len()),
                    code: None,
                }
            }
            Err(e) => Outcome::error(format!("{raw}: {e}")),
        }
    }

    pub fn edit(&mut self, raw: &str, edits: &[(String, String)]) -> Outcome {
        let path = match self.resolve(raw) {
            Ok(p) => p,
            Err(e) => return Outcome::error(e),
        };
        let original = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => return Outcome::error(format!("{raw}: {e}")),
        };
        let mut text = original.clone();

        for (i, (old, new)) in edits.iter().enumerate() {
            if old == new {
                return Outcome::error(format!("edit {}: old and new are identical", i + 1));
            }
            if old.is_empty() {
                return Outcome::error(format!("edit {}: old string is empty", i + 1));
            }
            let count = text.matches(old.as_str()).count();
            if count != 1 {
                return Outcome::error(format!(
                    "edit {}: found {count} matches, need exactly 1 — include more context",
                    i + 1
                ));
            }
            text = text.replacen(old.as_str(), new, 1);
        }

        if let Err(e) = std::fs::write(&path, &text) {
            return Outcome::error(format!("{raw}: {e}"));
        }
        self.read_paths.insert(path);
        Outcome {
            ok: true,
            text: format!("edited {raw} ({} changes)", edits.len()),
            code: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_jail_allows_the_config_directory() {
        let (t, dir) = tools();
        let real = dir.canonicalize().expect("canonical session dir");
        assert!(t.confined(&real.join("inside.txt")), "the session directory");
        let skill = crate::config::dirs_config()
            .join("imp")
            .join("skills/x/SKILL.md");
        assert!(t.confined(&skill), "a skill file should pass the jail");
        assert!(
            !t.confined(std::path::Path::new("/tmp/imp-not-here.txt")),
            "the rest of the disk stays closed"
        );
    }

    fn tools() -> (Tools, PathBuf) {
        use std::sync::atomic::AtomicU64;
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("imp-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut t = Tools::new(dir.clone(), dir.join(".out"));
        t.timeout = Duration::from_millis(300);
        (t, dir)
    }

    #[test]
    fn bash_captures_and_reports_exit() {
        let (mut t, _d) = tools();
        let o = t.bash("printf hello", &AtomicBool::new(false));
        assert!(o.ok);
        assert_eq!(o.text, "hello");
        let o = t.bash("exit 3", &AtomicBool::new(false));
        assert_eq!(o.code, Some(3));
    }

    #[test]
    fn bash_stderr_is_included() {
        let (mut t, _d) = tools();
        let o = t.bash("echo out; echo err 1>&2", &AtomicBool::new(false));
        assert!(o.text.contains("out") && o.text.contains("err"), "{}", o.text);
    }

    #[test]
    fn big_output_spills() {
        let (mut t, _d) = tools();
        let o = t.bash("yes x | head -c 20000", &AtomicBool::new(false));
        assert!(o.text.len() < 6000, "len {}", o.text.len());
        assert!(o.text.contains("full output:"), "{}", o.text);
    }

    #[test]
    fn cancel_stops_a_sleep() {
        let (mut t, _d) = tools();
        let cancel = AtomicBool::new(true);
        let o = t.bash("sleep 5", &cancel);
        assert!(!o.ok);
        assert!(o.text.contains("cancelled"), "{}", o.text);
    }

    #[test]
    fn timeout_is_bounded() {
        let (mut t, _d) = tools();
        let o = t.bash("sleep 40", &AtomicBool::new(false));
        assert!(!o.ok);
        assert!(o.text.contains("timed out"), "{}", o.text);
    }

    #[test]
    fn read_numbers_lines_and_batches() {
        let (mut t, d) = tools();
        std::fs::write(d.join("a.txt"), "one\ntwo\n").unwrap();
        std::fs::write(d.join("b.txt"), "three\n").unwrap();
        let o = t.read(&["a.txt".into(), "b.txt".into()], None, None);
        assert!(o.ok);
        assert!(o.text.contains("1: one"), "{}", o.text);
        assert!(o.text.contains("three"), "{}", o.text);
    }

    #[test]
    fn read_rejects_escape_and_binary() {
        let (mut t, d) = tools();
        std::fs::write(d.join("bin"), [0xff, 0xfe, 0x00]).unwrap();
        let o = t.read(&["bin".into()], None, None);
        assert!(!o.ok);
        assert!(o.text.contains("not valid UTF-8"));
        let o = t.read(&["../../etc/passwd".into()], None, None);
        assert!(!o.ok);
        assert!(o.text.contains("outside"), "{}", o.text);
    }

    #[test]
    fn jail_can_be_turned_off() {
        let (mut t, _d) = tools();
        assert!(t.resolve("/tmp/imp-outside").is_err());
        t.jail = false;
        assert!(t.resolve("/tmp/imp-outside").is_ok());
    }

    #[test]
    fn write_requires_a_prior_read() {
        let (mut t, d) = tools();
        std::fs::write(d.join("f.txt"), "old").unwrap();
        let o = t.write("f.txt", "new");
        assert!(!o.ok);
        assert!(o.text.contains("read f.txt"));
        let _ = t.read(&["f.txt".into()], None, None);
        assert!(t.write("f.txt", "new").ok);
        assert_eq!(std::fs::read_to_string(d.join("f.txt")).unwrap(), "new");
    }

    #[test]
    fn write_creates_directories() {
        let (mut t, d) = tools();
        assert!(t.write("x/y/z.txt", "hi").ok);
        assert_eq!(std::fs::read_to_string(d.join("x/y/z.txt")).unwrap(), "hi");
    }

    #[test]
    fn edit_is_atomic_and_counts_matches() {
        let (mut t, d) = tools();
        std::fs::write(d.join("e.txt"), "a a a").unwrap();
        let o = t.edit("e.txt", &[("a".into(), "b".into())]);
        assert!(!o.ok);
        assert!(o.text.contains("3 matches"), "{}", o.text);
        assert_eq!(std::fs::read_to_string(d.join("e.txt")).unwrap(), "a a a");
    }

    #[test]
    fn edit_applies_multiple_hunks() {
        let (mut t, d) = tools();
        std::fs::write(d.join("e.txt"), "one two three").unwrap();
        let o = t.edit(
            "e.txt",
            &[("one".into(), "1".into()), ("three".into(), "3".into())],
        );
        assert!(o.ok, "{}", o.text);
        assert_eq!(std::fs::read_to_string(d.join("e.txt")).unwrap(), "1 two 3");
    }

    #[test]
    fn edit_rejects_identical_and_empty() {
        let (mut t, d) = tools();
        std::fs::write(d.join("e.txt"), "x").unwrap();
        assert!(!t.edit("e.txt", &[("x".into(), "x".into())]).ok);
        assert!(!t.edit("e.txt", &[("".into(), "y".into())]).ok);
    }
}
