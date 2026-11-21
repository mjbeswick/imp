//! Background jobs: one process group each, output to a file, polled.

use std::collections::BTreeMap;
use std::ffi::c_int;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

extern "C" {
    fn setpgid(pid: c_int, pgid: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

const SIGTERM: c_int = 15;
const SIGKILL: c_int = 9;

pub struct Job {
    pub id: u64,
    pub command: String,
    pub output: PathBuf,
    pub cursor: u64,
    pub started: Instant,
    /// When the command itself begins; later than `started` for a delayed job.
    pub begins: Instant,
    child: Option<Child>,
    code: Option<i32>,
}

impl Job {
    #[allow(dead_code)]
    pub fn running(&self) -> bool {
        self.code.is_none()
    }

    pub fn status_line(&self) -> String {
        let now = Instant::now();
        let waiting = self.code.is_none() && now < self.begins;
        let state = match self.code {
            _ if waiting => format!("starts in {}", span(self.begins - now)),
            Some(c) => format!("exit {c}"),
            None => format!("running {}", span(now - self.begins)),
        };
        let cmd: String = self.command.chars().take(60).collect();
        let mut line = format!("job {}  {state}  {cmd}", self.id);
        if !waiting {
            let size = std::fs::metadata(&self.output).map_or(0, |m| m.len());
            line.push_str(&format!("\n  output {}", bytes(size)));
            if let Some(last) = last_line(&self.output) {
                line.push_str(&format!(" · last: {last}"));
            }
        }
        line
    }
}

/// A duration as `45s`, `12m 5s`, or `1h 59m`.
pub fn span(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 => format!("{}m {}s", s / 60, s % 60),
        _ => format!("{}h {}m", s / 3600, s % 3600 / 60),
    }
}

fn bytes(n: u64) -> String {
    match n {
        0..=1023 => format!("{n} B"),
        1024..=1_048_575 => format!("{:.1} KB", n as f64 / 1024.0),
        _ => format!("{:.1} MB", n as f64 / 1_048_576.0),
    }
}

/// The last non-empty line of a log. Progress meters redraw with `\r`, so
/// both `\r` and `\n` end a line.
fn last_line(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(1024))).ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    let text = String::from_utf8_lossy(&tail);
    let line = text.split(['\r', '\n']).map(str::trim).rfind(|l| !l.is_empty())?;
    Some(line.chars().take(100).collect())
}

/// Job ids are unique across the process, so sessions running side by side
/// never share a `job-<id>.log`.
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}

#[derive(Default)]
pub struct Jobs {
    map: BTreeMap<u64, Job>,
}

impl Jobs {
    pub fn start(&mut self, command: &str, dir: &Path) -> Result<u64, String> {
        self.start_after(command, dir, Duration::ZERO)
    }

    /// Start a job whose command waits `delay` first; killing it before then
    /// means the command never runs.
    pub fn start_after(&mut self, command: &str, dir: &Path, delay: Duration) -> Result<u64, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let id = next_id();
        let output = dir.join(format!("job-{id}.log"));
        let file = std::fs::File::create(&output).map_err(|e| e.to_string())?;

        let wait = if delay.is_zero() { String::new() } else { format!("sleep {}\n", delay.as_secs()) };
        let script = format!("{wait}exec 2>&1\n{command}");
        let mut cmd = Command::new("sh");
        // Jobs are the agent's: no secrets unless passed through.
        crate::secrets::scrub(&mut cmd);
        cmd.arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                setpgid(0, 0);
                Ok(())
            });
        }
        let child = cmd.spawn().map_err(|e| format!("could not start: {e}"))?;
        self.map.insert(
            id,
            Job {
                id,
                command: command.to_string(),
                output,
                cursor: 0,
                started: Instant::now(),
                begins: Instant::now() + delay,
                child: Some(child),
                code: None,
            },
        );
        Ok(id)
    }

    /// Take over a command that is already running, logging to `output`.
    pub fn adopt(&mut self, command: &str, child: Child, output: PathBuf) -> u64 {
        let id = next_id();
        self.map.insert(
            id,
            Job {
                id,
                command: command.to_string(),
                output,
                cursor: 0,
                started: Instant::now(),
                begins: Instant::now(),
                child: Some(child),
                code: None,
            },
        );
        id
    }

    /// Poll every job; return ids that finished since the last poll.
    pub fn poll(&mut self) -> Vec<(u64, i32, PathBuf)> {
        let mut done = Vec::new();
        for job in self.map.values_mut() {
            if job.code.is_some() {
                continue;
            }
            if let Some(child) = job.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let code = status.code().unwrap_or(-1);
                        job.code = Some(code);
                        done.push((job.id, code, job.output.clone()));
                    }
                    Ok(None) => {}
                    Err(_) => {
                        job.code = Some(-1);
                        done.push((job.id, -1, job.output.clone()));
                    }
                }
            }
        }
        done
    }

    pub fn wait(&mut self, id: u64, timeout: Duration) -> Result<String, String> {
        let start = Instant::now();
        loop {
            let Some(job) = self.map.get_mut(&id) else {
                return Err(format!("no job {id}"));
            };
            if let Some(code) = job.code {
                let text = read_from(&job.output, job.cursor, 3800);
                job.cursor += text.len() as u64;
                return Ok(format!("job {id} exit {code}\n{text}"));
            }
            if let Some(child) = job.child.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    let code = status.code().unwrap_or(-1);
                    job.code = Some(code);
                    let text = read_from(&job.output, job.cursor, 3800);
                    job.cursor += text.len() as u64;
                    return Ok(format!("job {id} exit {code}\n{text}"));
                }
            }
            if start.elapsed() > timeout {
                let text = read_from(&job.output, job.cursor, 3800);
                job.cursor += text.len() as u64;
                return Ok(format!(
                    "job {id} still running after {}s\n{text}",
                    timeout.as_secs()
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn kill(&mut self, id: u64) -> Result<String, String> {
        let Some(job) = self.map.get_mut(&id) else {
            return Err(format!("no job {id}"));
        };
        if let Some(pid) = job.child.as_ref().map(|c| c.id() as c_int) {
            unsafe {
                kill(-pid, SIGTERM);
            }
            std::thread::sleep(Duration::from_millis(300));
            unsafe {
                kill(-pid, SIGKILL);
            }
        }
        if let Some(child) = job.child.as_mut() {
            let _ = child.try_wait();
        }
        if job.code.is_none() {
            job.code = Some(-1);
        }
        Ok(format!("job {id} killed"))
    }

    pub fn list(&self) -> String {
        if self.map.is_empty() {
            return "no jobs".into();
        }
        self.map
            .values()
            .map(Job::status_line)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(dead_code)]
    pub fn running(&self) -> usize {
        self.map.values().filter(|j| j.running()).count()
    }
}

/// Jobs end with their session: SIGTERM to every running process group. No
/// wait, so closing a session never blocks the interface.
impl Drop for Jobs {
    fn drop(&mut self) {
        for job in self.map.values_mut().filter(|j| j.code.is_none()) {
            if let Some(child) = job.child.as_mut() {
                if matches!(child.try_wait(), Ok(None)) {
                    unsafe {
                        kill(-(child.id() as c_int), SIGTERM);
                    }
                }
            }
        }
    }
}

fn read_from(path: &Path, cursor: u64, cap: usize) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let start = (cursor as usize).min(bytes.len());
    let slice = &bytes[start..];
    let text = String::from_utf8_lossy(slice).to_string();
    if text.len() > cap {
        format!(
            "{}\n… [{} bytes more, full output: {}]",
            &text[..cap],
            text.len() - cap,
            path.display()
        )
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "imp-jobs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn dropping_jobs_stops_running_ones() {
        let d = dir();
        let mut jobs = Jobs::default();
        jobs.start(&format!("sleep 0.4; touch {}", d.join("late").display()), &d).unwrap();
        jobs.start("true", &d).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        jobs.poll();
        drop(jobs);
        std::thread::sleep(Duration::from_millis(700));
        assert!(!d.join("late").exists(), "the job outlived its session");
    }

    #[test]
    fn start_wait_and_read_output() {
        let d = dir();
        let mut jobs = Jobs::default();
        let id = jobs.start("printf job-ok", &d).unwrap();
        let out = jobs.wait(id, Duration::from_secs(5)).unwrap();
        assert!(out.contains("exit 0"), "{out}");
        assert!(out.contains("job-ok"), "{out}");
    }

    #[test]
    fn wait_times_out_but_reports_progress() {
        let d = dir();
        let mut jobs = Jobs::default();
        let id = jobs.start("echo first; sleep 3", &d).unwrap();
        let out = jobs.wait(id, Duration::from_millis(400)).unwrap();
        assert!(out.contains("still running"), "{out}");
        assert!(out.contains("first"), "{out}");
        let _ = jobs.kill(id);
    }

    #[test]
    fn kill_stops_the_group() {
        let d = dir();
        let mut jobs = Jobs::default();
        let id = jobs.start("sleep 30", &d).unwrap();
        
        jobs.kill(id).unwrap();
        let listing = jobs.list();
        assert!(listing.contains(&format!("job {id}")), "{listing}");
        assert!(!listing.contains("running"), "{listing}");
    }

    #[test]
    fn a_delayed_job_waits_and_can_be_cancelled_first() {
        let d = dir();
        let mut jobs = Jobs::default();
        let marker = d.join("ran");
        let id = jobs.start_after(&format!("touch {}", marker.display()), &d, Duration::from_secs(2)).unwrap();
        assert!(jobs.list().contains("starts in 1s") || jobs.list().contains("starts in 2s"), "{}", jobs.list());
        jobs.kill(id).unwrap();
        std::thread::sleep(Duration::from_millis(2500));
        assert!(!marker.exists(), "a cancelled delayed job ran");

        let id = jobs.start_after("echo late", &d, Duration::from_secs(1)).unwrap();
        let out = jobs.wait(id, Duration::from_secs(5)).unwrap();
        assert!(out.contains("exit 0") && out.contains("late"), "{out}");
    }

    #[test]
    fn status_shows_size_and_the_last_progress_line() {
        let d = dir();
        let mut jobs = Jobs::default();
        let id = jobs.start("printf 'copying\\n  10%%\\r  45%% 12MB/s\\r'; sleep 3", &d).unwrap();
        std::thread::sleep(Duration::from_millis(400));
        let text = jobs.list();
        assert!(text.contains("output 27 B · last: 45% 12MB/s"), "{text}");
        let _ = jobs.kill(id);
        assert_eq!(span(Duration::from_secs(7_140)), "1h 59m");
        assert_eq!(span(Duration::from_secs(725)), "12m 5s");
    }

    #[test]
    fn list_reports_running_and_done() {
        let d = dir();
        let mut jobs = Jobs::default();
        let a = jobs.start("true", &d).unwrap();
        let b = jobs.start("sleep 5", &d).unwrap();
        jobs.wait(a, Duration::from_secs(5)).unwrap();
        let text = jobs.list();
        assert!(text.contains(&format!("job {a}")));
        assert!(text.contains(&format!("job {b}")));
        assert!(text.contains("running"), "{text}");
        let _ = jobs.kill(b);
    }
}
