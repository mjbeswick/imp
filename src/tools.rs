//! The file and shell tools. Every one has a hard cap.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_int;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const INLINE_CAP: usize = 4 * 1024;
const MEM_CAP: usize = 16 * 1024 * 1024;
const SPILL_CAP: usize = 64 * 1024 * 1024;
const CAPTURE_CAP: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_WRITE: u64 = 5 * 1024 * 1024;
const READ_LINES: usize = 2000;
const READ_BYTES: usize = 200 * 1024;
const DIR_ENTRIES: usize = 200;
const LIST_BYTES: usize = 3500;
pub(crate) const IGNORE: &[&str] = &[".git", ".jj", "target", "node_modules", "dist", "build", ".venv"];

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
    /// The whole output, before the cap, for showing to the user.
    pub full: Option<String>,
}

impl Outcome {
    fn error(text: impl Into<String>) -> Self {
        Self {
            ok: false,
            text: text.into(),
            code: None,
            full: None,
        }
    }
}

enum Buffer {
    Memory(String),
    File { file: std::fs::File, len: usize },
}

impl Buffer {
    fn len(&self) -> usize {
        match self {
            Buffer::Memory(text) => text.len(),
            Buffer::File { len, .. } => *len,
        }
    }
}

/// Full tool output, memory first. When the memory budget is full the least
/// recently used buffer moves to a temporary file that is unlinked while still
/// open, so the id keeps working and nothing appears on disk.
struct Buffers {
    map: HashMap<u64, Buffer>,
    order: VecDeque<u64>,
    mem: usize,
    spill: usize,
    /// Bytes held by every session sharing this budget, this one included.
    totals: std::sync::Arc<Totals>,
    next: u64,
    mem_cap: usize,
    spill_cap: usize,
}

/// The process-wide byte counts behind one shared buffer budget.
#[derive(Default)]
struct Totals {
    mem: std::sync::atomic::AtomicUsize,
    spill: std::sync::atomic::AtomicUsize,
}

/// Every live session's buffers count against the same caps.
fn shared_totals() -> std::sync::Arc<Totals> {
    static SHARED: std::sync::OnceLock<std::sync::Arc<Totals>> = std::sync::OnceLock::new();
    SHARED.get_or_init(Default::default).clone()
}

impl Buffers {
    fn new() -> Self {
        Self::with_caps(MEM_CAP, SPILL_CAP, shared_totals())
    }

    fn with_caps(mem_cap: usize, spill_cap: usize, totals: std::sync::Arc<Totals>) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            mem: 0,
            spill: 0,
            totals,
            next: 0,
            mem_cap,
            spill_cap,
        }
    }

    fn store(&mut self, text: String) -> u64 {
        let id = self.next;
        self.next += 1;
        if text.len() <= self.mem_cap {
            while self.used().0 + text.len() > self.mem_cap {
                if !self.demote() {
                    break;
                }
            }
        }
        if text.len() <= self.mem_cap && self.used().0 + text.len() <= self.mem_cap {
            self.grow(text.len() as isize, 0);
            self.map.insert(id, Buffer::Memory(text));
            self.order.push_back(id);
        } else {
            self.put_file(id, text);
        }
        id
    }

    fn demote(&mut self) -> bool {
        let id = self
            .order
            .iter()
            .copied()
            .find(|id| matches!(self.map.get(id), Some(Buffer::Memory(_))));
        let Some(id) = id else {
            return false;
        };
        let Some(Buffer::Memory(text)) = self.map.remove(&id) else {
            return false;
        };
        self.grow(-(text.len() as isize), 0);
        let Some(file) = temp_file(id, &text) else {
            return true;
        };
        self.grow(0, text.len() as isize);
        self.map.insert(
            id,
            Buffer::File {
                file,
                len: text.len(),
            },
        );
        while self.used().1 > self.spill_cap {
            if !self.drop_oldest_file() {
                break;
            }
        }
        true
    }

    fn put_file(&mut self, id: u64, text: String) {
        let Some(file) = temp_file(id, &text) else {
            return;
        };
        self.grow(0, text.len() as isize);
        self.map.insert(
            id,
            Buffer::File {
                file,
                len: text.len(),
            },
        );
        self.order.push_back(id);
        while self.used().1 > self.spill_cap {
            if !self.drop_oldest_file() {
                break;
            }
        }
    }

    fn drop_oldest_file(&mut self) -> bool {
        let id = self
            .order
            .iter()
            .copied()
            .find(|id| matches!(self.map.get(id), Some(Buffer::File { .. })));
        let Some(id) = id else {
            return false;
        };
        if let Some(pos) = self.order.iter().position(|x| *x == id) {
            self.order.remove(pos);
        }
        if let Some(buffer) = self.map.remove(&id) {
            self.grow(0, -(buffer.len().min(self.spill) as isize));
            true
        } else {
            false
        }
    }

    fn read(&mut self, id: u64, offset: usize, limit: usize) -> Option<String> {
        if !self.map.contains_key(&id) {
            return None;
        }
        if let Some(pos) = self.order.iter().position(|x| *x == id) {
            self.order.remove(pos);
            self.order.push_back(id);
        }
        match self.map.get(&id)? {
            Buffer::Memory(text) => Some(slice_text(text, offset, limit)),
            Buffer::File { file, .. } => Some(slice_file(file, offset, limit)),
        }
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.grow(-(self.mem as isize), -(self.spill as isize));
    }

    /// Memory and spill bytes across every session sharing the budget.
    fn used(&self) -> (usize, usize) {
        use std::sync::atomic::Ordering::Relaxed;
        (self.totals.mem.load(Relaxed), self.totals.spill.load(Relaxed))
    }

    /// Change this session's counts and the shared ones together.
    fn grow(&mut self, mem: isize, spill: isize) {
        use std::sync::atomic::Ordering::Relaxed;
        self.mem = self.mem.saturating_add_signed(mem);
        self.spill = self.spill.saturating_add_signed(spill);
        for (total, delta) in [(&self.totals.mem, mem), (&self.totals.spill, spill)] {
            if delta >= 0 {
                total.fetch_add(delta as usize, Relaxed);
            } else {
                total.fetch_sub(delta.unsigned_abs(), Relaxed);
            }
        }
    }
}

/// A closed session's bytes leave the shared budget.
impl Drop for Buffers {
    fn drop(&mut self) {
        self.clear();
    }
}

fn temp_file(id: u64, text: &str) -> Option<std::fs::File> {
    let dir = std::env::temp_dir();
    for attempt in 0..8 {
        let path = dir.join(format!(
            "imp-buf-{}-{id}-{attempt}",
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
        {
            Ok(mut file) => {
                if file.write_all(text.as_bytes()).is_err() || file.seek(SeekFrom::Start(0)).is_err()
                {
                    let _ = std::fs::remove_file(&path);
                    return None;
                }
                let _ = std::fs::remove_file(&path);
                return Some(file);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    None
}

fn slice_text(text: &str, offset: usize, limit: usize) -> String {
    let mut out = String::new();
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
    out
}

fn slice_file(file: &std::fs::File, offset: usize, limit: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    let mut shown = 0usize;
    for (n, line) in BufReader::new(file).lines().enumerate().skip(offset) {
        let Ok(line) = line else {
            break;
        };
        if shown >= limit || used > READ_BYTES {
            out.push_str(&format!("... [stopped at line {n}]\n"));
            break;
        }
        let numbered = format!("{:>6}: {line}\n", n + 1);
        used += numbered.len();
        shown += 1;
        out.push_str(&numbered);
    }
    out
}

fn head_tail(text: &str) -> (String, String) {
    let head: String = text.chars().take(INLINE_CAP / 2).collect();
    let tail: String = text
        .chars()
        .rev()
        .take(INLINE_CAP / 4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    (head, tail)
}

/// Delete job logs older than `max_age` under the output directory.
pub fn prune_out(dir: &std::path::Path, max_age: Duration) {
    prune_out_with(dir, max_age, 64 * 1024 * 1024);
}

fn prune_out_with(dir: &std::path::Path, max_age: Duration, max_bytes: u64) {
    let now = std::time::SystemTime::now();

    // Legacy spill files from the disk-spill era. Nothing reads these names now.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let legacy = path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with("out-"))
                .unwrap_or(false);
            if legacy && path.is_file() {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    let jobs = dir.join("jobs");
    let mut logs: Vec<(std::time::SystemTime, u64, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&jobs) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            let stale = now
                .duration_since(modified)
                .map(|age| age > max_age)
                .unwrap_or(false);
            if stale {
                let _ = std::fs::remove_file(&path);
            } else {
                logs.push((modified, meta.len(), path));
            }
        }
    }

    let mut total: u64 = logs.iter().map(|(_, len, _)| *len).sum();
    logs.sort_by_key(|(modified, _, _)| *modified);
    for (_, len, path) in logs {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

/// A model command that outlived the timeout, left running for the job table.
pub struct Handover {
    pub command: String,
    pub child: std::process::Child,
    pub log: PathBuf,
}

pub struct Tools {
    pub root: PathBuf,
    pub out_dir: PathBuf,
    pub jail: bool,
    pub timeout: Duration,
    /// Set by `bash_handover` when a command is still running at the timeout.
    pub handover: Option<Handover>,
    read_paths: HashSet<PathBuf>,
    buffers: Buffers,
}

impl Tools {
    pub fn new(root: PathBuf, out_dir: PathBuf) -> Self {
        Self {
            root,
            out_dir,
            jail: true,
            timeout: TIMEOUT,
            handover: None,
            read_paths: HashSet::new(),
            buffers: Buffers::new(),
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
        // A dangling symlink counts as present: it cannot be resolved, so
        // canonicalising it fails and the path is refused.
        while probe.symlink_metadata().is_err() {
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
        if full.starts_with(&config) {
            return true;
        }
        let out = self
            .out_dir
            .canonicalize()
            .unwrap_or_else(|_| self.out_dir.clone());
        full.starts_with(&out)
    }

    /// Cap a result, storing the rest in memory and pointing at its buffer.
    pub fn cap(&mut self, text: &str) -> String {
        if text.len() <= INLINE_CAP {
            return text.to_string();
        }
        let (head, tail) = head_tail(text);
        let id = self.buffers.store(text.to_string());
        format!(
            "{head}\n... [{} bytes total; the rest is imp://out/{id}, open it with the read tool]\n{tail}",
            text.len()
        )
    }

    /// Cap a read result without storing it: the source file can be reopened
    /// with offset and limit.
    pub fn trim(&self, text: &str) -> String {
        if text.len() <= INLINE_CAP {
            return text.to_string();
        }
        let (head, tail) = head_tail(text);
        format!(
            "{head}\n... [{} bytes total; use offset and limit to read more]\n{tail}",
            text.len()
        )
    }

    pub fn clear_buffers(&mut self) {
        self.buffers.clear();
    }

    /// Run a command, terminating it at the timeout.
    pub fn bash(&mut self, command: &str, cancel: &AtomicBool) -> Outcome {
        // Typed commands keep the environment, but their output is redacted.
        let mut o = self.shell(command, cancel, false);
        o.text = crate::secrets::redact(&o.text);
        o
    }

    /// Run a command; at the timeout leave it running in `self.handover`.
    pub fn bash_handover(&mut self, command: &str, cancel: &AtomicBool) -> Outcome {
        self.shell(command, cancel, true)
    }

    fn shell(&mut self, command: &str, cancel: &AtomicBool, hand_over: bool) -> Outcome {
        let start = Instant::now();
        if cancel.load(Ordering::Relaxed) {
            return Outcome::error("cancelled");
        }

        // Output goes to a log, so a command can outlive this call as a job.
        let dir = self.out_dir.join("jobs");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let log = dir.join(format!("run-{}-{nanos}.log", std::process::id()));
        let file = match std::fs::create_dir_all(&dir).and_then(|_| std::fs::File::create(&log)) {
            Ok(f) => f,
            Err(e) => return Outcome::error(format!("could not start: {e}")),
        };

        let script = format!("exec 2>&1\n{command}");
        let mut cmd = Command::new("sh");
        // Only the agent's commands hand over; what the user types keeps the
        // full environment.
        if hand_over {
            crate::secrets::scrub(&mut cmd);
        }
        cmd.arg("-c")
            .arg(script)
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
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

        let mut killed = false;
        let status = 'outer: loop {
            match child.try_wait() {
                Ok(Some(s)) => break Some(s),
                Ok(None) => {}
                Err(e) => return Outcome::error(format!("wait failed: {e}")),
            }
            let late = start.elapsed() > self.timeout;
            if late && hand_over && !cancel.load(Ordering::Relaxed) {
                self.handover = Some(Handover {
                    command: command.to_string(),
                    child,
                    log,
                });
                return Outcome {
                    ok: true,
                    text: String::new(),
                    code: None,
                    full: None,
                };
            }
            if cancel.load(Ordering::Relaxed) || late {
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

        let mut raw = Vec::new();
        if let Ok(f) = std::fs::File::open(&log) {
            let _ = f.take(CAPTURE_CAP as u64).read_to_end(&mut raw);
        }
        let _ = std::fs::remove_file(&log);
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
                full: None,
            };
        }

        let code = status.and_then(|s| s.code());
        Outcome {
            ok: code == Some(0),
            text: self.cap(&text),
            code,
            full: Some(text),
        }
    }

    pub fn read(
        &mut self,
        paths: &[String],
        offset: Option<usize>,
        limit: Option<usize>,
        depth: Option<usize>,
    ) -> Outcome {
        let limit = limit.unwrap_or(READ_LINES).min(READ_LINES);
        let offset = offset.unwrap_or(0);
        let depth = depth.unwrap_or(1).clamp(1, 3);
        let mut budget = LIST_BYTES;
        let mut out = String::new();
        let mut ok = true;

        for raw in paths {
            if let Some(rest) = raw.strip_prefix("imp://out/") {
                if let Ok(id) = rest.parse::<u64>() {
                    match self.buffers.read(id, offset, limit) {
                        Some(text) => {
                            out.push_str(&format!("== {raw} ==\n{text}"));
                        }
                        None => {
                            ok = false;
                            out.push_str(&format!("== {raw} ==\nbuffer {raw} expired\n"));
                        }
                    }
                    continue;
                }
            }
            let path = match self.resolve(raw) {
                Ok(p) => p,
                Err(e) => {
                    ok = false;
                    out.push_str(&format!("== {raw} ==\n{e}\n"));
                    continue;
                }
            };
            if path.is_dir() {
                out.push_str(&format!("== {raw} ==\n"));
                walk_dir(&path, 0, depth, &mut out, &mut budget);
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
            text: self.trim(&out),
            code: None,
            full: None,
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
                    full: None,
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
            full: None,
        }
    }
}

/// Sorted, depth-limited directory listing with the shared ignore list and a
/// byte budget that keeps it under the spill threshold.
fn walk_dir(
    dir: &std::path::Path,
    level: usize,
    depth: usize,
    out: &mut String,
    budget: &mut usize,
) {
    if *budget == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        out.push_str(&format!("{}... [unreadable]\n", "  ".repeat(level)));
        return;
    };
    let mut items: Vec<_> = entries.flatten().collect();
    items.sort_by_key(|e| e.file_name());
    let mut shown = 0usize;
    for e in items {
        let name = e.file_name().to_string_lossy().to_string();
        if IGNORE.contains(&name.as_str()) {
            continue;
        }
        if shown >= DIR_ENTRIES {
            let line = format!("{}... [more entries cut]\n", "  ".repeat(level));
            if line.len() <= *budget {
                *budget -= line.len();
                out.push_str(&line);
            }
            return;
        }
        let path = e.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let file_type = meta.file_type();
        let (suffix, is_dir) = if file_type.is_symlink() {
            ("@", false)
        } else if file_type.is_dir() {
            ("/", true)
        } else {
            ("", false)
        };
        let line = format!("{}{}{}\n", "  ".repeat(level), name, suffix);
        if line.len() > *budget {
            *budget = 0;
            out.push_str("... [listing cut]\n");
            return;
        }
        *budget -= line.len();
        out.push_str(&line);
        shown += 1;
        if is_dir && level + 1 < depth {
            walk_dir(&path, level + 1, depth, out, budget);
        }
    }
}

/// Only `http` and `https` URLs may be fetched.
pub fn fetchable(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
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
    fn bash_runs_in_its_root() {
        let (mut t, dir) = tools();
        let o = t.bash("pwd", &AtomicBool::new(false));
        assert!(o.ok, "{}", o.text);
        let expected = dir.canonicalize().unwrap();
        assert_eq!(o.text.trim(), expected.display().to_string());
    }

    #[test]
    fn big_output_spills() {
        let (mut t, _d) = tools();
        let o = t.bash("yes x | head -c 20000", &AtomicBool::new(false));
        assert!(o.text.len() < 6000, "len {}", o.text.len());
        assert!(o.text.contains("imp://out/"), "{}", o.text);
    }

    #[test]
    fn buffers_demote_and_expire() {
        let mut b = Buffers::with_caps(8, 8, Default::default());
        let a = b.store("aaaa".into());
        let second = b.store("bbbb".into());
        let _c = b.store("cccc".into());
        assert!(b.read(a, 0, 10).unwrap().contains("aaaa"));
        let _d = b.store("dddd".into());
        let f = b.store("eeee".into());
        assert!(b.read(second, 0, 10).is_none(), "spill cap kept the oldest");
        assert!(b.read(a, 0, 10).is_some(), "recently read buffer was dropped");
        assert!(b.read(f, 0, 10).unwrap().contains("eeee"));
    }

    #[test]
    fn sessions_share_one_buffer_budget() {
        let totals: std::sync::Arc<Totals> = Default::default();
        let mut one = Buffers::with_caps(10, 100, totals.clone());
        let mut two = Buffers::with_caps(10, 100, totals.clone());
        let first = one.store("x".repeat(8));
        let second = two.store("y".repeat(8));
        let (mem, spill) = one.used();
        assert!(mem <= 10, "memory {mem} passed the shared cap");
        assert_eq!(mem + spill, 16);
        assert!(one.read(first, 0, 20).unwrap().contains("xxxx"));
        assert!(two.read(second, 0, 20).unwrap().contains("yyyy"));
        assert!(two.read(second + 1, 0, 20).is_none(), "addresses are per session");
        drop(two);
        assert_eq!(one.used(), (8, 0), "a closed session kept its bytes");
    }

    #[test]
    fn reads_a_buffer_by_line_range() {
        let mut t = tools().0;
        let big = "line\n".repeat(2000);
        let capped = t.cap(&big);
        let address = capped
            .split("the rest is ")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap()
            .to_string();
        assert!(address.starts_with("imp://out/"), "{address}");
        let o = t.read(&[address], Some(100), Some(2), None);
        assert!(o.ok, "{}", o.text);
        assert!(o.text.contains("101: line"), "{}", o.text);
        assert!(o.text.contains("102: line"), "{}", o.text);
        let missing = t.read(&["imp://out/9999".into()], None, None, None);
        assert!(!missing.ok);
        assert!(missing.text.contains("expired"), "{}", missing.text);
    }

    #[test]
    fn reads_are_not_buffered() {
        let (mut t, d) = tools();
        let text = "x".repeat(10_000);
        std::fs::write(d.join("big.txt"), &text).unwrap();
        let o = t.read(&["big.txt".into()], None, None, None);
        assert!(o.ok);
        assert!(o.text.contains("use offset and limit"), "{}", o.text);
        assert!(!o.text.contains("imp://out/"), "{}", o.text);
    }

    #[test]
    fn job_logs_are_readable() {
        let (mut t, _d) = tools();
        std::fs::create_dir_all(t.out_dir.join("jobs")).unwrap();
        let log = t.out_dir.join("jobs/job-1.log");
        std::fs::write(&log, "job output\n").unwrap();
        let o = t.read(&[log.display().to_string()], None, None, None);
        assert!(o.ok, "{}", o.text);
        assert!(o.text.contains("job output"), "{}", o.text);
    }

    #[test]
    fn prune_removes_only_old_output() {
        let (_t, d) = tools();
        let out = d.join("out");
        std::fs::create_dir_all(out.join("jobs")).unwrap();
        let old = out.join("jobs/old.log");
        let fresh = out.join("jobs/fresh.log");
        std::fs::write(&old, "old").unwrap();
        std::fs::write(&fresh, "fresh").unwrap();
        let file = std::fs::File::options().write(true).open(&old).unwrap();
        let when = std::time::SystemTime::now() - Duration::from_secs(8 * 86_400);
        file.set_times(std::fs::FileTimes::new().set_modified(when))
            .unwrap();
        prune_out(&out, Duration::from_secs(7 * 86_400));
        assert!(!old.exists(), "old log kept");
        assert!(fresh.exists(), "fresh log removed");
    }

    #[test]
    fn prune_removes_legacy_spill_files() {
        let (_t, d) = tools();
        let out = d.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let legacy = out.join("out-1234-1");
        std::fs::write(&legacy, "x").unwrap();
        prune_out(&out, Duration::from_secs(7 * 86_400));
        assert!(!legacy.exists(), "legacy spill kept");
    }

    #[test]
    fn prune_caps_the_job_log_size() {
        let (_t, d) = tools();
        let out = d.join("out");
        std::fs::create_dir_all(out.join("jobs")).unwrap();
        let old = out.join("jobs/a.log");
        let new = out.join("jobs/b.log");
        std::fs::write(&old, "12345678").unwrap();
        std::fs::write(&new, "abcdefgh").unwrap();
        let file = std::fs::File::options().write(true).open(&old).unwrap();
        let when = std::time::SystemTime::now() - Duration::from_secs(3_600);
        file.set_times(std::fs::FileTimes::new().set_modified(when))
            .unwrap();
        prune_out_with(&out, Duration::from_secs(7 * 86_400), 8);
        assert!(!old.exists(), "oldest log survived the cap");
        assert!(new.exists(), "newest log was dropped");
    }

    #[test]
    fn clearing_buffers_expires_addresses() {
        let mut t = tools().0;
        let capped = t.cap(&"y".repeat(10_000));
        let address = capped
            .split("the rest is ")
            .nth(1)
            .unwrap()
            .split(',')
            .next()
            .unwrap()
            .to_string();
        t.clear_buffers();
        let o = t.read(&[address], None, None, None);
        assert!(!o.ok);
        assert!(o.text.contains("expired"), "{}", o.text);
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
    fn only_web_urls_are_fetchable() {
        assert!(fetchable("https://wttr.in/?format=3"));
        assert!(fetchable("HTTP://192.168.0.61:11080/"));
        assert!(!fetchable("file:///etc/passwd"));
        assert!(!fetchable("ftp://x"));
        assert!(!fetchable("gopher://x"));
    }

    #[test]
    fn timeout_is_bounded() {
        let (mut t, _d) = tools();
        let o = t.bash("sleep 40", &AtomicBool::new(false));
        assert!(!o.ok);
        assert!(o.text.contains("timed out"), "{}", o.text);
    }

    #[test]
    fn a_slow_model_command_is_handed_over_and_finishes_as_a_job() {
        let (mut t, _d) = tools();
        let o = t.bash_handover("echo early; sleep 1; echo late", &AtomicBool::new(false));
        assert!(o.ok && o.code.is_none(), "{o:?}");
        let h = t.handover.take().expect("no handover");
        let mut jobs = crate::jobs::Jobs::default();
        let id = jobs.adopt(&h.command, h.child, h.log.clone());
        let text = jobs.wait(id, Duration::from_secs(10)).unwrap();
        assert!(text.contains("exit 0"), "{text}");
        assert!(text.contains("early") && text.contains("late"), "{text}");
    }

    #[test]
    fn a_fast_command_leaves_no_log_behind() {
        let (mut t, _d) = tools();
        let o = t.bash_handover("echo quick", &AtomicBool::new(false));
        assert!(o.ok && o.text.contains("quick"), "{o:?}");
        assert!(t.handover.is_none());
        let logs = std::fs::read_dir(t.out_dir.join("jobs")).map(|r| r.count()).unwrap_or(0);
        assert_eq!(logs, 0, "the run log stayed");
    }

    #[test]
    fn read_numbers_lines_and_batches() {
        let (mut t, d) = tools();
        std::fs::write(d.join("a.txt"), "one\ntwo\n").unwrap();
        std::fs::write(d.join("b.txt"), "three\n").unwrap();
        let o = t.read(&["a.txt".into(), "b.txt".into()], None, None, None);
        assert!(o.ok);
        assert!(o.text.contains("1: one"), "{}", o.text);
        assert!(o.text.contains("three"), "{}", o.text);
    }

    #[test]
    fn read_rejects_escape_and_binary() {
        let (mut t, d) = tools();
        std::fs::write(d.join("bin"), [0xff, 0xfe, 0x00]).unwrap();
        let o = t.read(&["bin".into()], None, None, None);
        assert!(!o.ok);
        assert!(o.text.contains("not valid UTF-8"));
        let o = t.read(&["../../etc/passwd".into()], None, None, None);
        assert!(!o.ok);
        assert!(o.text.contains("outside"), "{}", o.text);
    }

    #[test]
    fn read_lists_a_directory_tree() {
        let (mut t, d) = tools();
        std::fs::create_dir_all(d.join("src/a")).unwrap();
        std::fs::create_dir_all(d.join(".git")).unwrap();
        std::fs::create_dir_all(d.join("target")).unwrap();
        std::fs::write(d.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(d.join("src/a/mod.rs"), "mod a;\n").unwrap();
        std::fs::write(d.join("root.txt"), "root\n").unwrap();
        let _ = std::os::unix::fs::symlink(d.join("src"), d.join("src/loop"));

        let one = t.read(&["src".into()], None, None, None);
        assert!(one.ok, "{}", one.text);
        assert!(one.text.contains("main.rs"), "{}", one.text);
        assert!(!one.text.contains("mod.rs"), "depth 1 went deeper");
        assert!(!one.text.contains(".git") && !one.text.contains("target"));

        let two = t.read(&["src".into()], None, None, Some(2));
        assert!(two.text.contains("a/"), "{}", two.text);
        assert!(two.text.contains("  mod.rs"), "{}", two.text);
        assert!(two.text.contains("loop@"), "symlink not marked: {}", two.text);
    }

    #[test]
    fn read_caps_a_long_directory() {
        let (mut t, d) = tools();
        std::fs::create_dir_all(d.join("many")).unwrap();
        for i in 0..500 {
            std::fs::write(d.join("many").join(format!("f{i:03}")), "x").unwrap();
        }
        let o = t.read(&["many".into()], None, None, None);
        assert!(o.ok);
        assert!(o.text.contains("more entries cut"), "{}", o.text);
        let shown = o.text.lines().filter(|l| l.starts_with('f')).count();
        assert_eq!(shown, 200, "{}", shown);
    }

    #[test]
    fn read_mixes_a_directory_and_a_file() {
        let (mut t, d) = tools();
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("root.txt"), "root\n").unwrap();
        let o = t.read(&["src".into(), "root.txt".into()], None, None, None);
        assert!(o.ok);
        assert!(o.text.contains("== src =="), "{}", o.text);
        assert!(o.text.contains("1: root"), "{}", o.text);
    }

    #[test]
    fn the_agent_runs_without_secrets_and_typed_output_is_redacted() {
        use crate::secrets::{TEST_NAME, TEST_VALUE};
        std::env::set_var(TEST_NAME, TEST_VALUE);
        let (mut t, _d) = tools();
        let probe = format!("printf '[%s]' \"${TEST_NAME}\"");
        let agent = t.bash_handover(&probe, &AtomicBool::new(false));
        assert_eq!(agent.text.trim(), "[]", "the agent saw the secret");
        let typed = t.bash(&probe, &AtomicBool::new(false));
        assert_eq!(typed.text.trim(), format!("[[redacted {TEST_NAME}]]"));
    }

    #[test]
    fn a_dangling_link_cannot_leave_the_jail() {
        let (mut t, d) = tools();
        let outside = std::env::temp_dir().join(format!("imp-dangling-{}", std::process::id()));
        let _ = std::fs::remove_file(&outside);
        std::os::unix::fs::symlink(&outside, d.join("out")).unwrap();
        assert!(t.resolve("out").is_err(), "a dangling link resolved");
        assert!(t.resolve("out/deeper").is_err());
        assert!(!t.write("out", "x").ok);
        assert!(!outside.exists(), "the write left the jail");
        // A missing file under a real directory still resolves.
        assert!(t.resolve("new/file.txt").is_ok());
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
        let _ = t.read(&["f.txt".into()], None, None, None);
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
