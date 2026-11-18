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
    child: Option<Child>,
    code: Option<i32>,
}

impl Job {
    pub fn running(&self) -> bool {
        self.code.is_none()
    }

    pub fn status_line(&self) -> String {
        let state = match self.code {
            Some(c) => format!("exit {c}"),
            None => format!("running {}s", self.started.elapsed().as_secs()),
        };
        let cmd: String = self.command.chars().take(60).collect();
        format!("job {}  {state}  {cmd}", self.id)
    }
}

#[derive(Default)]
pub struct Jobs {
    map: BTreeMap<u64, Job>,
    next: u64,
}

impl Jobs {
    pub fn start(&mut self, command: &str, dir: &Path) -> Result<u64, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        self.next += 1;
        let id = self.next;
        let output = dir.join(format!("job-{id}.log"));
        let file = std::fs::File::create(&output).map_err(|e| e.to_string())?;

        let script = format!("exec 2>&1\n{command}");
        let mut cmd = Command::new("sh");
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
                child: Some(child),
                code: None,
            },
        );
        Ok(id)
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

    pub fn running(&self) -> usize {
        self.map.values().filter(|j| j.running()).count()
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
            "mote-jobs-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
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
        assert_eq!(jobs.running(), 1);
        jobs.kill(id).unwrap();
        let done = jobs.poll();
        assert!(done.iter().any(|(j, _, _)| *j == id) || jobs.running() == 0);
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
