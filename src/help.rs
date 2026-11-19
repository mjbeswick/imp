//! Help text. One renderer, used by `--help`, `/help`, and line mode.

pub const KEYS: &str = "Enter send · Shift+Enter/Alt+Enter newline · Esc cancel · Ctrl+C quit · Ctrl+L clear · PgUp/PgDn scroll · Up/Down history · Tab complete · !cmd shell · picker: type to filter, Enter choose, Esc cancel";

pub const COMMANDS: &[(&str, &str)] = &[
    ("/help", "show keys and commands"),
    ("/new", "start a fresh conversation"),
    ("/sessions", "pick a session"),
    ("/reload", "re-read instructions and skills"),
    ("/agent", "switch agent profile"),
    ("/compact", "compact the conversation now"),
    ("/think", "toggle thinking for the next turn"),
    ("/yolo", "prompt-free for this session"),
    ("/verbosity", "tool detail: low, high, or pick"),
    ("/cwd", "show the session directory"),
    ("/cd", "browse or change the directory"),
    ("/image", "browse or attach an image"),
    ("/fork", "copy the session and continue there"),
    ("/theme", "set the colour theme"),
    ("/quit", "leave"),
];

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn section(out: &mut String, title: &str, rows: &[(&str, &str)], width: usize) {
    out.push('\n');
    out.push_str(title);
    out.push('\n');
    let col = rows
        .iter()
        .map(|(f, _)| f.chars().count())
        .max()
        .unwrap_or(0)
        .max(12);
    for (flag, desc) in rows {
        let pad = col - flag.chars().count();
        let mut parts = wrap(desc, width.saturating_sub(col + 4).max(24)).into_iter();
        let first = parts.next().unwrap_or_default();
        out.push_str(&format!("  {flag}{}  {first}\n", " ".repeat(pad)));
        for extra in parts {
            out.push_str(&format!("  {}{}  {extra}\n", " ".repeat(col), " ".repeat(pad)));
        }
    }
}

/// `--help`. Width comes from the terminal so long descriptions wrap.
pub fn usage(width: usize) -> String {
    let width = width.clamp(60, 110);
    let mut out = String::from("imp — a small terminal agent for your model provider\n");

    out.push_str("\nUSAGE\n");
    out.push_str("  imp [OPTIONS] [PROMPT]\n  imp init\n  imp prompt\n");

    section(
        &mut out,
        "SESSIONS",
        &[
            ("-c, --continue", "continue the most recent session"),
            ("-r, --resume <ID>", "resume a session by id"),
            ("-l, --list", "list sessions and exit"),
            ("--rename <ID> <TITLE>", "rename a session"),
            ("--delete <ID>", "move a session to trash"),
        ],
        width,
    );

    section(
        &mut out,
        "RUNNING",
        &[
            ("--agent <NAME>", "agent profile to start with [default]"),
            ("--provider <NAME>", "deepseek · openai · anthropic · gemini · ollama · custom [deepseek]"),
            ("--model <NAME>", "model to use [deepseek-flash]"),
            ("--max-steps <N>", "tool steps per turn [24]"),
            ("--permissions <MODE>", "normal · strict · yolo [normal]"),
            ("--skills-dir <PATH>", "extra skills directory"),
            ("-C, --cwd <PATH>", "session directory [launch directory]"),
            ("--cli", "line mode, even in a terminal"),
        ],
        width,
    );

    section(
        &mut out,
        "OTHER",
        &[
            ("-h, --help", "show this help"),
            ("-V, --version", "show the version"),
        ],
        width,
    );

    out.push_str("\nEXAMPLES\n");
    for (cmd, what) in [
        ("imp", "open the chat"),
        ("imp \"why is the build failing?\"", "one request, then exit"),
        ("imp -c", "continue where you left off"),
        ("imp -r 512705c23", "resume a specific session"),
        ("printf 'ls\\n/quit\\n' | imp", "scripted line mode"),
        ("!git status", "run a shell command from the prompt"),
    ] {
        out.push_str(&format!("  {cmd:<36} {what}\n"));
    }

    section(
        &mut out,
        "ENVIRONMENT",
        &[
            ("IMP_PROVIDER", "provider profile [deepseek]"),
            ("DEEPSEEK_API_KEY", "deepseek key"),
            ("OPENAI_API_KEY", "openai key"),
            ("ANTHROPIC_API_KEY", "anthropic key"),
            ("GEMINI_API_KEY", "gemini key"),
            ("IMP_API_KEY", "key for any provider"),
            ("IMP_MODEL", "overrides the model"),
            ("IMP_PERMISSIONS", "overrides the permission mode"),
            ("IMP_COMPACT_AT", "token budget for compaction"),
            ("NO_COLOR", "disable colour"),
        ],
        width,
    );

    out.push_str("\nFILES\n");
    out.push_str("  ~/.config/imp/          config, agents, skills\n");
    out.push_str("  ~/.local/state/imp/     sessions, memory, spilled output\n");
    out.push_str("\nRun `imp prompt` to see the system prompt.\n");
    out
}

/// `/help` in either front-end.
pub fn commands() -> String {
    let mut out = format!("Keys\n  {KEYS}\n\nCommands\n");
    let col = COMMANDS.iter().map(|(n, _)| n.len()).max().unwrap_or(8);
    for (name, desc) in COMMANDS {
        out.push_str(&format!("  {name:<col$}  {desc}\n", col = col));
    }
    out.push_str("\nAnything else goes to the model. Prefix a line with `!` to run it as a shell command.");
    out
}

pub fn unknown(name: &str) -> String {
    let nearest = COMMANDS
        .iter()
        .map(|(n, _)| (*n, distance(n.trim_start_matches('/'), name)))
        .min_by_key(|(_, d)| *d);
    match nearest {
        Some((n, d)) if d <= 3 => format!("unknown command /{name} — did you mean {n}?"),
        _ => format!("unknown command /{name}; try /help"),
    }
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_lists_every_flag() {
        let text = usage(100);
        for flag in [
            "--continue",
            "--resume",
            "--list",
            "--rename",
            "--delete",
            "--agent",
            "--provider",
            "--model",
            "--max-steps",
            "--permissions",
            "--skills-dir",
            "--cwd",
            "--cli",
            "--version",
        ] {
            assert!(text.contains(flag), "missing {flag}");
        }
    }

    #[test]
    fn usage_shows_defaults_and_examples() {
        let text = usage(100);
        assert!(text.contains("[normal]"));
        assert!(text.contains("[deepseek-flash]"));
        assert!(text.contains("EXAMPLES"));
        assert!(text.contains("imp -c"));
        assert!(!text.contains("mote"));
    }

    #[test]
    fn usage_names_every_provider_key() {
        let text = usage(100);
        for key in [
            "DEEPSEEK_API_KEY",
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "GEMINI_API_KEY",
            "IMP_API_KEY",
            "IMP_PROVIDER",
        ] {
            assert!(text.contains(key), "missing {key}");
        }
    }

    #[test]
    fn lines_fit_the_width() {
        let text = usage(80);
        for line in text.lines() {
            assert!(line.chars().count() <= 80, "too wide: {line:?}");
        }
    }

    #[test]
    fn keys_mention_shift_enter() {
        let text = commands();
        assert!(text.contains("Shift+Enter"), "{text}");
        assert!(text.contains("PgUp/PgDn scroll"), "{text}");
    }

    #[test]
    fn commands_lists_every_command() {
        let text = commands();
        assert_eq!(COMMANDS.len(), 15);
        for (name, _) in COMMANDS {
            assert!(text.contains(name), "missing {name}");
        }
        assert!(text.contains("Tab"));
    }

    #[test]
    fn commands_describe_the_picker_forms() {
        let text = commands();
        for phrase in [
            "pick a session",
            "low, high, or pick",
            "browse or change the directory",
            "browse or attach an image",
        ] {
            assert!(text.contains(phrase), "missing {phrase:?}");
        }
    }

    #[test]
    fn unknown_suggests_the_nearest() {
        assert!(unknown("compact").contains("/compact"));
        assert!(unknown("zzzzzz").contains("try /help"));
    }
}
