//! `imp init`: create the layout, write two examples, overwrite nothing.

use std::path::Path;

const CONFIG_TEMPLATE: &str = "\
# imp configuration. Environment variables override every key.
# Keys: DEEPSEEK_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY.
# Ollama and custom hosts need none. Do not put a key in this file.

# provider=deepseek           # deepseek | openai | anthropic | gemini | ollama | custom
# model=deepseek-flash
# base_url=https://api.deepseek.com
# permissions=normal          # normal | strict | yolo
# allow_sudo=false
# thinking=false
# links=auto                  # auto | always | never
# prune_at=45000
# compact_at=60000
# max_tokens=8192
# color=auto                  # auto | always | never
# mouse=on                    # wheel scrolls the chat; Option-drag selects
# skills_dir=~/.config/imp/skills
";

const AGENT_EXAMPLE: &str = "\
---
description: reviews a diff and reports only defects
# tools: read, bash
# model: deepseek-v4-pro
# thinking: true
---

You review code. Read the diff, then report only real defects, each with a
file and line. Say nothing about style unless it changes behaviour.
";

const SKILL_EXAMPLE: &str = "\
---
name: git
description: Inspect and change a git repository. Use for status, diff, log, commit, rebase, stash, branch, worktree, and undoing changes.
---

# git

- Inspect first: `git status --short`, `git diff`, `git log --oneline -10`.
- Never `git reset --hard` or `git push --force` without being asked.
- Prefer `git switch -c <name>` over checkout for new branches.
- Show the diff before committing, and write the message from the change.
";

pub struct Report {
    pub created: Vec<String>,
    pub kept: Vec<String>,
}

pub fn run() -> Result<Report, String> {
    run_at(
        &crate::config::dirs_config().join("imp"),
        &crate::config::dirs_state().join("imp"),
    )
}

pub fn run_at(config_dir: &Path, state_dir: &Path) -> Result<Report, String> {
    let mut report = Report {
        created: Vec::new(),
        kept: Vec::new(),
    };

    for dir in [
        config_dir.to_path_buf(),
        config_dir.join("agents"),
        config_dir.join("skills"),
        state_dir.to_path_buf(),
        state_dir.join("sessions"),
        state_dir.join("memory"),
        state_dir.join("out"),
    ] {
        ensure_dir(&dir, &mut report)?;
    }

    write_once(
        &config_dir.join("config"),
        CONFIG_TEMPLATE,
        &mut report,
    )?;
    write_once(
        &config_dir.join("agents").join("example.md"),
        AGENT_EXAMPLE,
        &mut report,
    )?;
    write_once(
        &config_dir.join("skills").join("git").join("SKILL.md"),
        SKILL_EXAMPLE,
        &mut report,
    )?;

    Ok(report)
}

fn ensure_dir(path: &Path, report: &mut Report) -> Result<(), String> {
    if path.is_dir() {
        report.kept.push(format!("{}", path.display()));
        return Ok(());
    }
    std::fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))?;
    report.created.push(format!("{}", path.display()));
    Ok(())
}

fn write_once(path: &Path, content: &str, report: &mut Report) -> Result<(), String> {
    if path.exists() {
        report.kept.push(format!("{}", path.display()));
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    report.created.push(format!("{}", path.display()));
    Ok(())
}

pub fn print(report: &Report) {
    for p in &report.created {
        println!("  created  {p}");
    }
    for p in &report.kept {
        println!("  kept     {p}");
    }
    println!();
    println!("  next: set your provider API key, then run imp");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "imp-init-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn creates_layout_and_examples() {
        let base = tmp();
        let cfg = base.join("config-dir");
        let state = base.join("state-dir");
        let report = run_at(&cfg, &state).unwrap();
        assert!(cfg.join("config").is_file());
        assert!(cfg.join("agents/example.md").is_file());
        assert!(cfg.join("skills/git/SKILL.md").is_file());
        assert!(state.join("sessions").is_dir());
        assert!(state.join("memory").is_dir());
        assert!(state.join("out").is_dir());
        assert_eq!(report.kept.len(), 0);
    }

    #[test]
    fn never_overwrites() {
        let base = tmp();
        let cfg = base.join("c");
        let state = base.join("s");
        run_at(&cfg, &state).unwrap();
        std::fs::write(cfg.join("config"), "model=custom\n").unwrap();
        let report = run_at(&cfg, &state).unwrap();
        assert_eq!(std::fs::read_to_string(cfg.join("config")).unwrap(), "model=custom\n");
        assert!(report.kept.iter().any(|p| p.ends_with("config")));
        assert!(report.created.is_empty() || !report.created.iter().any(|p| p.ends_with("config")));
    }

    #[test]
    fn template_has_no_key() {
        assert!(!CONFIG_TEMPLATE.contains("api_key="));
        assert!(CONFIG_TEMPLATE.contains("# model=deepseek-flash"));
    }

    #[test]
    fn examples_parse() {
        let base = tmp();
        let cfg = base.join("c");
        let state = base.join("s");
        run_at(&cfg, &state).unwrap();
        let agent = crate::store::Agent::from_file(&cfg.join("agents/example.md")).unwrap();
        assert!(agent.instructions.contains("You review code"));
        let skill = std::fs::read_to_string(cfg.join("skills/git/SKILL.md")).unwrap();
        assert!(skill.contains("description:"));
    }
}
