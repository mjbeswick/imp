//! Agents are markdown profiles: instructions in the body, knobs in frontmatter.

use crate::config::{dirs_config, Config};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub thinking: Option<bool>,
    pub source: Option<PathBuf>,
}

impl Agent {
    /// `build`: all tools, no extra instructions, the agent a session starts in.
    pub fn builtin() -> Self {
        builtin_agents().swap_remove(0)
    }

    /// Tools this agent may use; `None` means all of them.
    pub fn allows(&self, tool: &str) -> bool {
        match &self.tools {
            None => true,
            Some(list) => list.iter().any(|t| t.trim() == tool),
        }
    }

    pub fn from_file(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let name = path.file_stem()?.to_string_lossy().to_string();
        Some(Self::from_text(name, &text, Some(path.to_path_buf())))
    }

    /// Parse an agent file's text: optional frontmatter, then instructions.
    pub fn from_text(name: String, text: &str, source: Option<PathBuf>) -> Self {
        let mut tools = None;
        let mut model = None;
        let mut thinking = None;
        let mut description = String::new();
        let mut body = String::new();
        let mut in_fm = false;
        let mut fm_done = false;

        for (i, line) in text.lines().enumerate() {
            if i == 0 && line.trim() == "---" {
                in_fm = true;
                continue;
            }
            if in_fm && !fm_done {
                if line.trim() == "---" {
                    in_fm = false;
                    fm_done = true;
                    continue;
                }
                if let Some((k, v)) = line.split_once(':') {
                    let v = v.trim().trim_matches('"');
                    match k.trim() {
                        "tools" => {
                            tools = Some(
                                v.split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect(),
                            )
                        }
                        "model" => model = Some(v.to_string()),
                        "thinking" => thinking = Some(matches!(v, "true" | "yes" | "on")),
                        "description" => description = v.to_string(),
                        _ => {}
                    }
                }
                continue;
            }
            body.push_str(line);
            body.push('\n');
        }

        if description.is_empty() {
            description = body
                .lines()
                .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .unwrap_or("")
                .chars()
                .take(80)
                .collect();
        }
        Self {
            name,
            description,
            instructions: body.trim().to_string(),
            tools,
            model,
            thinking,
            source,
        }
    }
}

/// Agents compiled in, in the order Tab cycles them; a file replaces one.
pub fn builtin_agents() -> Vec<Agent> {
    [
        ("build", include_str!("../assets/agents/build.md")),
        ("plan", include_str!("../assets/agents/plan.md")),
        ("review", include_str!("../assets/agents/review.md")),
    ]
    .into_iter()
    .map(|(name, text)| Agent::from_text(name.into(), text, None))
    .collect()
}

/// An agent name as typed or saved; the old `default` is now `build`.
pub fn resolve_agent(name: &str) -> &str {
    if name == "default" { "build" } else { name }
}

/// Skills compiled in, available without any files on disk.
pub fn builtin_skills() -> Vec<(String, String, String)> {
    [
        (
            "config",
            "Change imp's own configuration, skills, agents, or memory. Use when asked to add a skill, create an agent, remember a preference, or explain where config lives. Cannot edit the main config file.",
            include_str!("../assets/skills/config/SKILL.md"),
        ),
        (
            "rg",
            "Search file contents with ripgrep. Use for finding text, symbols, TODOs, or which files mention something. Faster and safer than grep.",
            include_str!("../assets/skills/rg/SKILL.md"),
        ),
        (
            "jq",
            "Query and reshape JSON with jq. Use for jq filters, extracting fields, formatting JSON output, or checking API responses.",
            include_str!("../assets/skills/jq/SKILL.md"),
        ),
        (
            "gh",
            "Use the GitHub CLI for pull requests, issues, checks, and releases. Use for gh pr, gh issue, gh run, gh release, or repo metadata.",
            include_str!("../assets/skills/gh/SKILL.md"),
        ),
        (
            "docker",
            "Inspect and run containers with docker or podman. Use for images, containers, logs, compose, ports, or disk usage.",
            include_str!("../assets/skills/docker/SKILL.md"),
        ),
        (
            "macos",
            "Inspect and change things on macOS. Use for launchd services, defaults, brew, disk space, network, or system settings.",
            include_str!("../assets/skills/macos/SKILL.md"),
        ),
    ]
    .into_iter()
    .map(|(n, d, body)| (n.to_string(), d.to_string(), body.to_string()))
    .collect()
}

/// Global, project, then extra roots; project entries shadow global by name.
pub fn agent_dirs(cfg: &Config, root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(extra) = &cfg.skills_dir {
        dirs.push(extra.with_file_name("agents"));
    }
    dirs.push(root.join(".agents").join("agents"));
    dirs.push(root.join(".imp").join("agents"));
    dirs.push(dirs_config().join("imp").join("agents"));
    dirs
}

pub fn load_agents(cfg: &Config, root: &Path) -> Vec<Agent> {
    let mut found: Vec<Agent> = Vec::new();
    for dir in agent_dirs(cfg, root) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Some(agent) = Agent::from_file(&path) {
                if !found.iter().any(|a| a.name == agent.name) {
                    found.push(agent);
                }
            }
        }
    }
    for agent in builtin_agents() {
        if !found.iter().any(|a| a.name == agent.name) {
            found.push(agent);
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "imp-agent-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn project_dir_is_dot_imp() {
        let root = Path::new("/tmp/project");
        let cfg = Config::default();
        assert!(agent_dirs(&cfg, root).contains(&root.join(".imp").join("agents")));
    }

    #[test]
    fn dot_agents_skills_shadow_dot_imp() {
        let root = tmp();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let write = |dir: &Path, desc: &str| {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: zz-shared\ndescription: {desc}\n---\nbody from {desc}\n"),
            )
            .unwrap();
        };
        write(&root.join(".imp/skills/zz-shared"), "imp");
        write(&root.join(".agents/skills/zz-shared"), "agents");
        std::fs::write(root.join(".agents/skills/.openspec-target"), "x").unwrap();

        let cfg = Config::default();
        let skills = load_skills(&cfg, &root);
        let found = skills.iter().find(|s| s.name == "zz-shared").unwrap();
        assert_eq!(found.description, "agents");
        assert!(found.body.contains("body from agents"));
    }

    #[test]
    fn dot_agents_profiles_are_loaded_first() {
        let root = tmp();
        std::fs::create_dir_all(root.join(".agents/agents")).unwrap();
        std::fs::write(
            root.join(".agents/agents/zz-agent.md"),
            "# zz-agent\nagents body\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join(".imp/agents")).unwrap();
        std::fs::write(root.join(".imp/agents/zz-agent.md"), "# zz-agent\nimp body\n")
            .unwrap();

        let cfg = Config::default();
        let agents = load_agents(&cfg, &root);
        let found = agents.iter().find(|a| a.name == "zz-agent").unwrap();
        assert!(found.instructions.contains("agents body"));
        let dirs = agent_dirs(&cfg, &root);
        assert_eq!(dirs[0], root.join(".agents").join("agents"));
    }

    #[test]
    fn dot_imp_marks_a_project() {
        let d = tmp();
        std::fs::create_dir_all(d.join(".imp")).unwrap();
        assert!(is_project(&d));
    }

    #[test]
    fn parses_frontmatter_and_body() {
        let d = tmp();
        let p = d.join("reviewer.md");
        std::fs::write(
            &p,
            "---\ntools: read, bash\nmodel: deepseek-v4-pro\nthinking: true\ndescription: review code\n---\n\nYou review code.\n",
        )
        .unwrap();
        let a = Agent::from_file(&p).unwrap();
        assert_eq!(a.name, "reviewer");
        assert_eq!(a.description, "review code");
        assert!(a.allows("read") && a.allows("bash"));
        assert!(!a.allows("write"));
        assert_eq!(a.model.as_deref(), Some("deepseek-v4-pro"));
        assert_eq!(a.thinking, Some(true));
        assert!(a.instructions.contains("You review code."));
    }

    #[test]
    fn file_without_frontmatter_still_loads() {
        let d = tmp();
        let p = d.join("plain.md");
        std::fs::write(&p, "Just instructions.\n").unwrap();
        let a = Agent::from_file(&p).unwrap();
        assert_eq!(a.name, "plain");
        assert!(a.tools.is_none());
        assert_eq!(a.description, "Just instructions.");
    }

    #[test]
    fn three_agents_are_built_in() {
        let d = tmp();
        let cfg = Config::default();
        let agents = load_agents(&cfg, &d);
        let get = |n: &str| agents.iter().find(|a| a.name == n).unwrap_or_else(|| panic!("no {n}"));
        assert!(get("build").allows("write") && get("build").allows("edit"));
        for name in ["plan", "review"] {
            let a = get(name);
            assert!(a.allows("read") && a.allows("bash") && a.allows("ask"), "{name} cannot look");
            assert!(!a.allows("write") && !a.allows("edit") && !a.allows("memory"), "{name} can write");
            assert!(!a.instructions.is_empty() && !a.description.is_empty());
        }
        assert!(!agents.iter().any(|a| a.name == "default"), "the old name is still listed");
        assert_eq!(Agent::builtin().name, "build");
        assert_eq!(resolve_agent("default"), "build");
        assert_eq!(resolve_agent("plan"), "plan");
    }

    #[test]
    fn a_file_replaces_a_built_in() {
        let d = tmp();
        let dir = d.join(".agents").join("agents");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plan.md"), "My own planner.\n").unwrap();
        let agents = load_agents(&Config::default(), &d);
        let plans: Vec<&Agent> = agents.iter().filter(|a| a.name == "plan").collect();
        assert_eq!(plans.len(), 1, "plan is listed twice");
        assert_eq!(plans[0].instructions, "My own planner.");
    }

    #[test]
    fn permissions_key_is_ignored() {
        let d = tmp();
        let p = d.join("sneaky.md");
        std::fs::write(&p, "---\npermissions: yolo\n---\nhi\n").unwrap();
        let a = Agent::from_file(&p).unwrap();
        assert!(a.instructions.contains("hi"));
        assert!(a.model.is_none());
    }
}

// ---------------------------------------------------------------- sessions

use crate::api::{Message, Role};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Meta {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub model: String,
    pub messages: u64,
    pub updated: u64,
}

#[derive(Clone)]
pub struct Log {
    pub id: String,
    pub path: PathBuf,
    pub meta_path: PathBuf,
}

impl Log {
    pub fn create(dir: &Path, cwd: &Path, model: &str) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let id = claim_id(dir);
        let log = Log {
            path: dir.join(format!("{id}.jsonl")),
            meta_path: dir.join(format!("{id}.json")),
            id,
        };
        log.write_meta(&Meta {
            id: log.id.clone(),
            title: String::new(),
            cwd: cwd.display().to_string(),
            model: model.to_string(),
            messages: 0,
            updated: now(),
        });
        log
    }

    pub fn open(dir: &Path, id: &str) -> Option<Self> {
        let path = dir.join(format!("{id}.jsonl"));
        if !path.is_file() {
            return None;
        }
        Some(Log {
            id: id.to_string(),
            path,
            meta_path: dir.join(format!("{id}.json")),
        })
    }

    pub fn latest(dir: &Path) -> Option<Self> {
        let mut all = Self::list(dir);
        all.sort_by_key(|m| m.updated);
        all.last().and_then(|m| Self::open(dir, &m.id))
    }

    pub fn meta(&self) -> Meta {
        std::fs::read_to_string(&self.meta_path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .map(|v| Meta {
                id: v["id"].as_str().unwrap_or(&self.id).to_string(),
                title: v["title"].as_str().unwrap_or("").to_string(),
                cwd: v["cwd"].as_str().unwrap_or("").to_string(),
                model: v["model"].as_str().unwrap_or("").to_string(),
                messages: v["messages"].as_u64().unwrap_or(0),
                updated: v["updated"].as_u64().unwrap_or(0),
            })
            .unwrap_or(Meta {
                id: self.id.clone(),
                title: String::new(),
                cwd: String::new(),
                model: String::new(),
                messages: 0,
                updated: 0,
            })
    }

    /// Copy the transcript and sidecar into a new session.
    pub fn fork(&self) -> Result<Log, String> {
        let id = claim_id(self.path.parent().unwrap_or(Path::new(".")));
        let path = self.path.with_file_name(format!("{id}.jsonl"));
        let meta_path = self.meta_path.with_file_name(format!("{id}.json"));
        if self.path.exists() {
            std::fs::copy(&self.path, &path)
                .map_err(|e| format!("could not fork transcript: {e}"))?;
        } else if let Err(e) = std::fs::File::create(&path) {
            return Err(format!("could not fork transcript: {e}"));
        }
        let mut meta = self.meta();
        meta.id = id.clone();
        meta.title = if meta.title.is_empty() {
            "fork".into()
        } else {
            format!("fork of {}", meta.title).chars().take(60).collect()
        };
        meta.updated = now();
        let log = Log { id, path, meta_path };
        log.write_meta(&meta);
        Ok(log)
    }

    pub fn meta_cwd(&self) -> Option<PathBuf> {
        let cwd = self.meta().cwd;
        if cwd.is_empty() {
            None
        } else {
            Some(PathBuf::from(cwd))
        }
    }

    pub fn set_cwd(&self, path: &Path) -> Result<(), String> {
        let mut meta = self.meta();
        meta.cwd = path.display().to_string();
        meta.updated = now();
        self.write_meta(&meta);
        Ok(())
    }

    fn write_meta(&self, meta: &Meta) {
        let v = serde_json::json!({
            "id": meta.id,
            "title": meta.title,
            "cwd": meta.cwd,
            "model": meta.model,
            "messages": meta.messages,
            "updated": meta.updated,
        });
        let _ = std::fs::write(&self.meta_path, v.to_string());
    }

    pub fn append(&self, message: &Message) {
        let line = serde_json::json!({ "t": "msg", "m": message.to_json() }).to_string();
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{line}");
        }
        let mut meta = self.meta();
        meta.messages += 1;
        meta.updated = now();
        if meta.title.is_empty() && message.role == Role::User {
            meta.title = message.content.chars().take(60).collect();
        }
        self.write_meta(&meta);
    }

    pub fn event(&self, kind: &str, detail: &str) {
        let line = serde_json::json!({ "t": kind, "d": detail, "at": now() }).to_string();
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{line}");
        }
    }

    /// The most recent event of a kind, for example the active agent.
    pub fn last_event(&self, kind: &str) -> Option<String> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        text.lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter(|v| v["t"].as_str() == Some(kind))
            .filter_map(|v| v["d"].as_str().map(str::to_string))
            .next_back()
    }

    /// Rebuild the wire history, dropping orphan tool results.
    pub fn load(&self) -> Vec<Message> {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        let mut out: Vec<Message> = Vec::new();
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if v["t"].as_str() == Some("retry") {
                // A retry drops the user message it names and all after it.
                let text = v["d"].as_str().unwrap_or_default();
                if let Some(i) = out.iter().rposition(|m| m.role == Role::User && m.content == text) {
                    out.truncate(i);
                }
                continue;
            }
            if v["t"].as_str() != Some("msg") {
                continue;
            }
            let Some(m) = Message::from_json(&v["m"]) else {
                continue;
            };
            if m.role == Role::Tool {
                let id = m.tool_call_id.clone().unwrap_or_default();
                let known = out
                    .iter()
                    .any(|p| p.tool_calls.iter().any(|c| c.id == id));
                if !known {
                    continue;
                }
            }
            out.push(m);
        }
        out
    }

    pub fn list(dir: &Path) -> Vec<Meta> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let id = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if let Some(log) = Log::open(dir, &id) {
                out.push(log.meta());
            }
        }
        out.sort_by_key(|m| std::cmp::Reverse(m.updated));
        out
    }

    /// Give this open log a new title.
    pub fn retitle(&self, title: &str) -> Result<(), String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("the title is empty".into());
        }
        let mut meta = self.meta();
        meta.title = title.chars().take(120).collect();
        meta.updated = now();
        self.write_meta(&meta);
        Ok(())
    }

    pub fn rename(dir: &Path, id: &str, title: &str) -> Result<(), String> {
        let Some(log) = Log::open(dir, id) else {
            return Err(format!("no session {id}"));
        };
        let mut meta = log.meta();
        meta.title = title.to_string();
        meta.updated = now();
        log.write_meta(&meta);
        Ok(())
    }

    pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
        let Some(log) = Log::open(dir, id) else {
            return Err(format!("no session {id}"));
        };
        let trash = dir.join(".trash");
        std::fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
        let _ = std::fs::rename(&log.path, trash.join(format!("{id}.jsonl")));
        let _ = std::fs::rename(&log.meta_path, trash.join(format!("{id}.json")));
        Ok(())
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A time-based id no stored session has, claimed by creating its sidecar
/// exclusively, so two processes in the same second still differ.
fn claim_id(dir: &Path) -> String {
    let base = timestamp_id();
    for attempt in 0u32.. {
        let id = if attempt == 0 { base.clone() } else { format!("{base}-{attempt}") };
        let taken = dir.join(format!("{id}.jsonl")).exists();
        let claimed = !taken
            && std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join(format!("{id}.json")))
                .is_ok();
        if claimed {
            return id;
        }
        if attempt > 1000 {
            // An unwritable directory: let the caller's writes report it.
            return base;
        }
    }
    unreachable!()
}

fn timestamp_id() -> String {
    // Local-time-ish id without a date crate: seconds since epoch, base 36.
    let s = now();
    let days = s / 86_400;
    let secs = s % 86_400;
    format!("{days:x}{secs:05x}")
}

// ------------------------------------------------------------------ memory

pub struct Memory {
    pub dirs: Vec<PathBuf>,
}

impl Memory {
    pub fn discover(cfg: &Config, root: &Path) -> Self {
        let mut dirs = Vec::new();
        if is_project(root) {
            dirs.push(root.join(".imp").join("memory"));
        }
        dirs.push(crate::config::dirs_state().join("imp").join("memory"));
        let _ = cfg;
        Self { dirs }
    }

    /// Slug and title of each note, project notes first, sorted, capped.
    pub fn index(&self, limit: usize) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for dir in &self.dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut notes: Vec<(String, String)> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
                .filter_map(|p| {
                    let stem = p.file_stem()?.to_string_lossy().to_string();
                    let body = std::fs::read_to_string(&p).ok()?;
                    let title = body
                        .lines()
                        .find_map(|l| l.strip_prefix("# "))
                        .map(|t| t.trim().to_string())
                        .unwrap_or_else(|| stem.clone());
                    Some((stem, title))
                })
                .collect();
            notes.sort();
            out.extend(notes);
        }
        out.truncate(limit);
        out
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(PathBuf, String)> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let needle = words.first().cloned().unwrap_or_default();
        let mut hits = Vec::new();
        for dir in &self.dirs {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for e in entries.flatten() {
                let path = e.path();
                if path.extension().and_then(|s| s.to_str()) != Some("md") {
                    continue;
                }
                let Ok(body) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let hay = format!("{} {}", name.to_lowercase(), body.to_lowercase());
                if words.iter().all(|w| hay.contains(w.as_str())) {
                    hits.push((path, excerpt(&body, &needle)));
                }
                if hits.len() >= limit {
                    return hits;
                }
            }
        }
        hits
    }

    pub fn save(&self, title: &str, body: &str) -> Result<PathBuf, String> {
        let dir = self.dirs.first().ok_or("no memory directory")?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let slug = slugify(title);
        let path = dir.join(format!("{slug}.md"));
        if path.exists() {
            return Err(format!("a note called {slug}.md already exists"));
        }
        let text = format!("# {title}\n\n{}\n", body.trim());
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

fn excerpt(body: &str, needle: &str) -> String {
    let lower = body.to_lowercase();
    let at = lower.find(needle).unwrap_or(0);
    let start = at.saturating_sub(80);
    let end = (at + needle.len() + 160).min(body.len());
    let mut s = body[start..end].replace('\n', " ");
    if start > 0 {
        s.insert(0, '…');
    }
    if end < body.len() {
        s.push('…');
    }
    s
}

pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "note".into()
    } else {
        out
    }
}

/// One line for `--list` and the picker: id, age, title, messages.
/// One session as a left part (id, age, one-line title) and a right part
/// (message count). Every session list shares it.
pub fn session_row(meta: &Meta, now_secs: u64) -> (String, String) {
    let age = now_secs.saturating_sub(meta.updated);
    let age = match age {
        0..=59 => format!("{age}s"),
        60..=3599 => format!("{}m", age / 60),
        3600..=86399 => format!("{}h", age / 3600),
        _ => format!("{}d", age / 86_400),
    };
    let title = meta.title.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = if title.is_empty() { "(untitled)".to_string() } else { title };
    (
        format!("{:<9}  {age:>3}  {title}", meta.id),
        format!("{} msgs", meta.messages),
    )
}

/// Sessions whose user or assistant text contains `query`, newest first, each
/// with a one-line snippet around the first hit.
// ponytail: ASCII case folding keeps byte offsets valid; full Unicode folding if asked.
pub fn search(dir: &Path, query: &str, limit: usize) -> Vec<(Meta, String)> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for meta in Log::list(dir) {
        let Some(log) = Log::open(dir, &meta.id) else { continue };
        // A byte scan first: most logs miss, and parsing them is the slow part
        // on a Pi. Words with JSON escapes cannot be checked raw.
        let raw = std::fs::read_to_string(&log.path).unwrap_or_default().to_ascii_lowercase();
        if !needle.split_whitespace().all(|w| w.contains(['"', '\\']) || raw.contains(w)) {
            continue;
        }
        let hit = log.load().into_iter().filter(|m| matches!(m.role, Role::User | Role::Assistant)).find_map(|m| {
            let text = m.content.split_whitespace().collect::<Vec<_>>().join(" ");
            let at = text.to_ascii_lowercase().find(&needle)?;
            let start = text[..at].char_indices().rev().nth(24).map_or(0, |(i, _)| i);
            let snippet: String = text[start..].chars().take(needle.len() + 48).collect();
            Some(if start > 0 { format!("…{snippet}") } else { snippet })
        });
        if let Some(snippet) = hit {
            out.push((meta, snippet));
            if out.len() == limit {
                break;
            }
        }
    }
    out
}

/// Sessions as aligned text rows no wider than `width`: titles give way,
/// counts stay whole and end in one column.
pub fn session_table(metas: &[Meta], now_secs: u64, width: usize) -> String {
    let rows: Vec<(String, String)> = metas.iter().map(|m| session_row(m, now_secs)).collect();
    let right = rows.iter().map(|(_, r)| r.chars().count()).max().unwrap_or(0);
    let longest = rows.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0);
    let left = longest.min(width.saturating_sub(right + 2)).max(16);
    let mut out = String::new();
    for (l, r) in rows {
        let l = if l.chars().count() > left {
            let mut cut: String = l.chars().take(left - 1).collect();
            cut.push('…');
            cut
        } else {
            l
        };
        out.push_str(&format!("{l:<left$}  {r:>right$}\n"));
    }
    out
}

pub fn is_project(root: &Path) -> bool {
    root.join(".git").exists() || root.join(".jj").exists() || root.join(".imp").exists()
}

// ------------------------------------------------------------------ skills

#[allow(dead_code)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub path: Option<PathBuf>,
}

pub fn skill_dirs(cfg: &Config, root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![crate::config::dirs_config().join("imp").join("skills")];
    if is_project(root) {
        dirs.push(root.join(".imp").join("skills"));
        dirs.push(root.join(".agents").join("skills"));
    }
    if let Some(extra) = &cfg.skills_dir {
        dirs.push(extra.clone());
    }
    dirs
}

pub fn load_skills(cfg: &Config, root: &Path) -> Vec<Skill> {
    let mut out: Vec<Skill> = builtin_skills()
        .into_iter()
        .map(|(name, description, body)| Skill {
            name,
            description,
            body,
            path: None,
        })
        .collect();

    for dir in skill_dirs(cfg, root) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            let path = e.path().join("SKILL.md");
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let (name, description) = frontmatter(&text);
            let name = name.unwrap_or_else(|| {
                e.file_name().to_string_lossy().to_string()
            });
            if description.is_empty() {
                continue;
            }
            // Project and extra roots shadow built-ins and global ones.
            if let Some(existing) = out.iter_mut().find(|s| s.name == name) {
                *existing = Skill {
                    name,
                    description,
                    body: strip_frontmatter(&text),
                    path: Some(path),
                };
            } else {
                out.push(Skill {
                    name,
                    description,
                    body: strip_frontmatter(&text),
                    path: Some(path),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.truncate(40);
    out
}

fn frontmatter(text: &str) -> (Option<String>, String) {
    let mut name = None;
    let mut description = String::new();
    let mut in_fm = false;
    for (i, line) in text.lines().enumerate() {
        if i == 0 && line.trim() == "---" {
            in_fm = true;
            continue;
        }
        if in_fm {
            if line.trim() == "---" {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                let v = v.trim().trim_matches('"').to_string();
                match k.trim() {
                    "name" => name = Some(v),
                    "description" => description = v,
                    _ => {}
                }
            }
        }
    }
    (name, description)
}

fn strip_frontmatter(text: &str) -> String {
    let mut lines = text.lines();
    if lines.next().map(|l| l.trim()) != Some("---") {
        return text.trim().to_string();
    }
    let mut body = String::new();
    let mut closed = false;
    for line in lines {
        if !closed {
            if line.trim() == "---" {
                closed = true;
            }
            continue;
        }
        body.push_str(line);
        body.push('\n');
    }
    body.trim().to_string()
}

// ------------------------------------------------------------ instructions

/// Global, repository, then session directory. `CLAUDE.md` only as a fallback.
pub fn load_instructions(root: &Path) -> (String, Vec<(PathBuf, usize)>) {
    let mut files: Vec<PathBuf> = Vec::new();
    let global = crate::config::dirs_config().join("imp").join("AGENTS.md");
    if global.is_file() {
        files.push(global);
    }
    let repo = repo_root(root);
    if let Some(repo) = &repo {
        if let Some(f) = pick_agents(repo) {
            files.push(f);
        }
    }
    if repo.as_deref() != Some(root) {
        if let Some(f) = pick_agents(root) {
            files.push(f);
        }
    }

    let mut out = String::new();
    let mut loaded = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let room = 8192usize.saturating_sub(out.len());
        if room == 0 {
            break;
        }
        let piece: String = text.chars().take(room).collect();
        let truncated = piece.len() < text.len();
        loaded.push((path.clone(), piece.len()));
        out.push_str("\n# Instructions from ");
        out.push_str(&path.display().to_string());
        out.push('\n');
        out.push_str(&piece);
        if truncated {
            out.push_str("\n[truncated at 8 KB]\n");
        }
    }
    (out, loaded)
}

fn pick_agents(dir: &Path) -> Option<PathBuf> {
    let a = dir.join("AGENTS.md");
    if a.is_file() {
        return Some(a);
    }
    let c = dir.join("CLAUDE.md");
    if c.is_file() {
        return Some(c);
    }
    None
}

pub fn repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        if d.join(".git").exists() || d.join(".jj").exists() {
            return Some(d);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}

#[cfg(test)]
mod session_tests {
    use super::*;

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "imp-store-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn fork_copies_the_transcript() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        log.append(&Message::user("hello"));
        let bytes = std::fs::read(&log.path).unwrap();
        let copy = log.fork().unwrap();
        assert_ne!(copy.id, log.id);
        assert_eq!(std::fs::read(&copy.path).unwrap(), bytes);
        assert!(copy.meta().title.starts_with("fork of"), "{:?}", copy.meta().title);
        assert_eq!(copy.meta().cwd, log.meta().cwd);
        assert!(log.path.exists());
    }

    #[test]
    fn sessions_in_the_same_second_get_their_own_ids() {
        let dir = tmp();
        let a = Log::create(&dir, &dir, "m");
        let b = Log::create(&dir, &dir, "m");
        let c = a.fork().unwrap();
        assert!(a.id != b.id && b.id != c.id && a.id != c.id, "{} {} {}", a.id, b.id, c.id);
        a.append(&Message::user("first"));
        b.append(&Message::user("second"));
        assert_eq!(a.meta().title, "first");
        assert_eq!(b.meta().title, "second");
        assert_eq!(a.load().len(), 1);
    }

    #[test]
    fn retitle_changes_the_meta_title() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        log.retitle("fix the parser").unwrap();
        assert_eq!(log.meta().title, "fix the parser");
        assert!(log.retitle("   ").is_err());
    }

    #[test]
    fn search_finds_message_text_with_a_snippet() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        log.append(&Message::system("wttr secret prompt"));
        log.append(&Message::user("what is the weather"));
        log.append(&Message::assistant("I asked wttr.in and it says rain in Woking"));
        let hits = search(&dir, "WTTR", 20);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0.id, log.id);
        assert!(hits[0].1.contains("wttr.in"), "{:?}", hits[0].1);
        assert!(search(&dir, "prompt", 20).is_empty(), "system text matched");
        assert!(search(&dir, "nothing like this", 20).is_empty());
    }

    #[test]
    fn title_comes_from_the_first_user_message() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        log.append(&Message::system("base"));
        log.append(&Message::user("fix the flaky rsync test"));
        assert_eq!(log.meta().title, "fix the flaky rsync test");
        assert_eq!(log.meta().messages, 2);
    }

    #[test]
    fn list_is_newest_first_and_rename_keeps_the_transcript() {
        let dir = tmp();
        let older = Log::create(&dir, &dir, "deepseek-flash");
        older.append(&Message::user("one"));
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let newer = Log::create(&dir, &dir, "deepseek-flash");
        newer.append(&Message::user("two"));
        let listed = Log::list(&dir);
        assert_eq!(listed[0].id, newer.id);
        let before = std::fs::read(&older.path).unwrap();
        Log::rename(&dir, &older.id, "renamed").unwrap();
        assert_eq!(std::fs::read(&older.path).unwrap(), before);
        assert_eq!(Log::open(&dir, &older.id).unwrap().meta().title, "renamed");
    }

    #[test]
    fn delete_moves_both_files_to_trash() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        log.append(&Message::user("bye"));
        Log::delete(&dir, &log.id).unwrap();
        assert!(!log.path.exists());
        assert!(!log.meta_path.exists());
        assert!(dir.join(".trash").join(format!("{}.jsonl", log.id)).exists());
        assert!(Log::list(&dir).is_empty());
    }

    #[test]
    fn load_drops_orphan_tool_results() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        log.append(&Message::tool("gone", "orphan"));
        log.append(&Message::user("hello"));
        let loaded = log.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "hello");
    }

    fn meta(id: &str, title: &str, messages: u64) -> Meta {
        Meta {
            id: id.into(),
            title: title.into(),
            cwd: "/tmp".into(),
            model: "m".into(),
            messages,
            updated: 1_000,
        }
    }

    #[test]
    fn session_rows_show_age_and_messages() {
        let (left, right) = session_row(&meta("abc", "flaky test", 7), 1_090);
        assert!(left.starts_with("abc") && left.contains(" 1m  flaky test"), "{left}");
        assert_eq!(right, "7 msgs");
    }

    #[test]
    fn session_tables_align_counts_and_keep_one_line() {
        let metas = [
            meta("51280befc", "Rate my machine.", 13),
            meta("51280ba80", &"a long title ".repeat(10), 67),
            meta("51270a84e", "alpha\nbeta", 1),
            meta("512706203", "", 0),
        ];
        let text = session_table(&metas, 1_090, 60);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "{text}");
        for line in &lines {
            assert!(line.ends_with("msgs"), "{line}");
            assert_eq!(line.chars().count(), lines[0].chars().count(), "{text}");
            assert!(line.chars().count() <= 60, "{line}");
        }
        assert!(lines[1].contains('…') && lines[1].ends_with("67 msgs"), "{text}");
        assert!(lines[2].contains("alpha beta"), "{text}");
        assert!(lines[3].contains("(untitled)"), "{text}");
    }

    #[test]
    fn sidecar_defaults_when_missing() {
        let dir = tmp();
        let log = Log::create(&dir, &dir, "deepseek-flash");
        std::fs::remove_file(&log.meta_path).unwrap();
        assert_eq!(log.meta().id, log.id);
        assert_eq!(log.meta().title, "");
    }
}
