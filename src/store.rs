//! Agents are markdown profiles: instructions in the body, knobs in frontmatter.

use crate::config::{dirs_config, Config};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
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
    pub fn builtin() -> Self {
        Self {
            name: "default".into(),
            description: "all tools, no extra instructions".into(),
            instructions: String::new(),
            tools: None,
            model: None,
            thinking: None,
            source: None,
        }
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

        let name = path.file_stem()?.to_string_lossy().to_string();
        if description.is_empty() {
            description = body
                .lines()
                .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .unwrap_or("")
                .chars()
                .take(80)
                .collect();
        }
        Some(Self {
            name,
            description,
            instructions: body.trim().to_string(),
            tools,
            model,
            thinking,
            source: Some(path.to_path_buf()),
        })
    }
}

/// Skills compiled in, available without any files on disk.
pub fn builtin_skills() -> Vec<(String, String, String)> {
    vec![(
        "config".to_string(),
        "Change mote's own configuration, skills, agents, or memory. Use when asked to add a skill, create an agent, remember a preference, or explain where config lives. Cannot edit the main config file.".to_string(),
        include_str!("../assets/skills/config/SKILL.md").to_string(),
    )]
}

/// Global, project, then extra roots; project entries shadow global by name.
pub fn agent_dirs(cfg: &Config, root: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![root.join(".mote").join("agents")];
    if let Some(extra) = &cfg.skills_dir {
        dirs.push(extra.with_file_name("agents"));
    }
    dirs.push(dirs_config().join("mote").join("agents"));
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
    found.sort_by(|a, b| a.name.cmp(&b.name));
    if found.is_empty() {
        found.push(Agent::builtin());
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "mote-agent-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
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
    fn empty_directory_gives_default() {
        let d = tmp();
        let cfg = Config::default();
        let agents = load_agents(&cfg, &d);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "default");
        assert!(agents[0].allows("write"));
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
