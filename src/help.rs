//! Help text. One renderer, used by `--help`, `/help`, and line mode.

/// Keys in the full-screen chat.
pub const KEYS: &[(&str, &str)] = &[
    ("Enter", "send"),
    ("Shift+Enter, Alt+Enter", "newline"),
    ("Esc", "cancel the turn or close a picker"),
    ("Ctrl+C", "clear the draft, or quit"),
    ("Ctrl+L", "clear the screen"),
    ("Ctrl+E", "expand the last tool call"),
    ("Ctrl+O", "inbox: live sessions and what they need"),
    ("Ctrl+N, Ctrl+P", "next or previous live session"),
    ("?", "this list, from an empty composer"),
    ("Up, Down", "history"),
    ("PgUp, PgDn", "scroll"),
    ("Tab", "complete"),
    ("Picker", "type to filter, Enter choose, Esc cancel"),
];

/// Keys in line mode, which reads whole lines.
const LINE_KEYS: &[(&str, &str)] = &[("Enter", "send"), ("Ctrl+D", "leave")];

/// Commands by area, named from `COMMANDS`.
const GROUPS: &[(&str, &[&str])] = &[
    ("Conversation", &["/new", "/compact", "/think", "/stash"]),
    ("Model and agent", &["/model", "/provider", "/agent", "/mode", "/reload"]),
    ("Workspace", &["/cwd", "/cd", "/image", "/shell"]),
    ("Session", &["/sessions", "/inbox", "/fork", "/rename", "/quit"]),
    ("Interface", &["/help", "/theme", "/verbosity"]),
];

pub const COMMANDS: &[(&str, &str)] = &[
    ("/help", "show keys and commands"),
    ("/new", "start another live session"),
    ("/sessions", "pick a session"),
    ("/inbox", "live sessions and what they need"),
    ("/reload", "re-read instructions and skills"),
    ("/agent", "switch agent profile"),
    ("/compact", "compact the conversation now"),
    ("/think", "toggle thinking for the next turn"),
    ("/mode", "normal, strict, or yolo for this session"),
    ("/verbosity", "tool detail: low, high, or pick"),
    ("/cwd", "show the session directory"),
    ("/cd", "browse or change the directory"),
    ("/image", "browse or attach an image"),
    ("/fork", "copy the session and continue there"),
    ("/theme", "set the colour theme"),
    ("/shell", "open a shell"),
    ("/model", "set or pick the session model"),
    ("/provider", "list, add, edit, delete, or use providers"),
    ("/stash", "park messages for this session"),
    ("/rename", "rename the current session"),
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

/// Widest left cell that still shares the description column.
const LEFT_CAP: usize = 22;

/// Named sections of `(left, description)` rows.
type Sections<'a> = [(&'a str, Vec<(&'a str, &'a str)>)];

/// Render sections with one description column for the whole output: two
/// spaces after the longest left cell, which may be at most `LEFT_CAP` wide.
/// A longer cell keeps its own row and its description starts on the next.
fn table(out: &mut String, sections: &Sections, width: usize) {
    let left = sections
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|(l, _)| l.chars().count()))
        .filter(|&n| n <= LEFT_CAP)
        .max()
        .unwrap_or(0);
    let col = 2 + left + 2;
    let text = width.saturating_sub(col).max(20);
    for (i, (title, rows)) in sections.iter().enumerate() {
        if i > 0 || !out.is_empty() {
            out.push('\n');
        }
        out.push_str(title);
        out.push('\n');
        for (cell, desc) in rows {
            let mut parts = wrap(desc, text).into_iter();
            let len = cell.chars().count();
            if len > left {
                out.push_str(&format!("  {cell}\n"));
            } else if let Some(first) = parts.next() {
                out.push_str(&format!("  {cell}{}{first}\n", " ".repeat(col - 2 - len)));
            }
            for rest in parts {
                out.push_str(&format!("{}{rest}\n", " ".repeat(col)));
            }
        }
    }
}

/// `--help`. Width comes from the terminal so long descriptions wrap.
pub fn usage(width: usize) -> String {
    let width = width.clamp(60, 110);
    let mut out = String::from("imp — a small terminal agent for your model provider\n");
    table(
        &mut out,
        &[
            (
                "USAGE",
                vec![
                    ("imp [OPTIONS] [PROMPT]", "open the chat, or answer one prompt and exit"),
                    ("imp init", "create the config and state directories"),
                    ("imp prompt", "print the system prompt"),
                ],
            ),
            (
                "SESSIONS",
                vec![
                    ("-c, --continue", "continue the most recent session"),
                    ("-r, --resume <ID>", "resume a session by id"),
                    ("-l, --list", "list sessions and exit"),
                    ("--rename <ID> <TITLE>", "rename a session"),
                    ("--delete <ID>", "move a session to trash"),
                ],
            ),
            (
                "RUNNING",
                vec![
                    ("--agent <NAME>", "agent profile to start with (default: default)"),
                    (
                        "--provider <NAME>",
                        "deepseek, openai, anthropic, gemini, ollama, or custom (default: deepseek)",
                    ),
                    ("--model <NAME>", "model to use (default: deepseek-flash)"),
                    ("--max-steps <N>", "tool steps per turn (default: 24)"),
                    ("--permissions <MODE>", "normal, strict, or yolo (default: normal)"),
                    ("--skills-dir <PATH>", "extra skills directory"),
                    ("-C, --cwd <PATH>", "session directory (default: launch directory)"),
                    ("--cli", "line mode, even in a terminal"),
                ],
            ),
            (
                "OTHER",
                vec![("-h, --help", "show this help"), ("-V, --version", "show the version")],
            ),
            (
                "EXAMPLES",
                vec![
                    ("imp", "open the chat"),
                    ("imp \"why is the build failing?\"", "one request, then exit"),
                    ("imp -c", "continue where you left off"),
                    ("imp -r 512705c23", "resume a specific session"),
                    ("printf 'ls\\n/quit\\n' | imp", "scripted line mode"),
                ],
            ),
            (
                "ENVIRONMENT",
                vec![
                    ("IMP_PROVIDER", "provider profile (default: deepseek)"),
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
            ),
            (
                "FILES",
                vec![
                    ("~/.config/imp/", "config, agents, skills"),
                    ("~/.local/state/imp/", "sessions, memory, spilled output"),
                ],
            ),
        ],
        width,
    );
    out.push('\n');
    for line in wrap("In the chat, a line starting with `!` runs as a shell command.", width) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// `/help`, wrapped to `width`, with the keys of the chat or of line mode.
pub fn commands(width: usize, chat: bool) -> String {
    let desc = |name: &str| {
        COMMANDS
            .iter()
            .find(|(n, _)| *n == name)
            .map_or("", |(_, d)| *d)
    };
    let mut sections = vec![("Keys", if chat { KEYS } else { LINE_KEYS }.to_vec())];
    for (title, names) in GROUPS {
        sections.push((title, names.iter().map(|n| (*n, desc(n))).collect()));
    }
    let mut out = String::new();
    table(&mut out, &sections, width);
    out.push('\n');
    let note = "Anything else goes to the model. Prefix a line with `!` to run it as a shell command.";
    out.push_str(&wrap(note, width).join("\n"));
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
        assert!(text.contains("(default: normal)"));
        assert!(text.contains("(default: deepseek-flash)"));
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

    /// Column where a row's description starts, if it has one on that row.
    fn desc_col(line: &str) -> Option<usize> {
        let body = line.strip_prefix("  ")?;
        let gap = body.find("  ")?;
        let rest = &body[gap..];
        Some(2 + gap + rest.len() - rest.trim_start().len())
    }

    #[test]
    fn usage_shares_one_column_and_fits_sixty() {
        let text = usage(60);
        let mut cols = std::collections::BTreeSet::new();
        for line in text.lines() {
            assert!(line.chars().count() <= 60, "too wide: {line:?}");
            if let Some(c) = desc_col(line) {
                cols.insert(c);
            }
        }
        assert_eq!(cols.len(), 1, "{cols:?}\n{text}");
        assert!(text.contains("  --rename <ID> <TITLE>   rename"), "{text}");
    }

    #[test]
    fn an_entry_over_the_cap_moves_its_description_down() {
        let text = usage(80);
        let at = text.find("  imp \"why is the build failing?\"\n").expect("own row");
        let next = text[at..].lines().nth(1).unwrap();
        assert_eq!(next.trim(), "one request, then exit");
        assert_eq!(next.len() - next.trim_start().len(), 26);
    }

    #[test]
    fn keys_are_rows() {
        let text = commands(80, true);
        assert!(text.contains("Ctrl+C"), "{text}");
        assert!(text.contains("Shift+Enter"), "{text}");
        assert!(text.contains("PgUp, PgDn"), "{text}");
    }

    #[test]
    fn line_mode_lists_only_its_own_keys() {
        let text = commands(80, false);
        for key in ["Picker", "PgUp", "Tab", "Ctrl+E", "Esc"] {
            assert!(!text.contains(key), "{key} in line mode:\n{text}");
        }
        assert!(text.contains("Ctrl+D"), "{text}");
        assert!(text.contains("/compact"), "{text}");
    }

    #[test]
    fn every_command_sits_in_one_group() {
        assert_eq!(COMMANDS.len(), 21);
        for (name, _) in COMMANDS {
            let n = GROUPS.iter().filter(|(_, g)| g.contains(name)).count();
            assert_eq!(n, 1, "{name} is in {n} groups");
        }
        let text = commands(60, true);
        for line in text.lines() {
            assert!(line.chars().count() <= 60, "too wide: {line:?}");
        }
        let cols: std::collections::BTreeSet<usize> =
            text.lines().filter_map(desc_col).collect();
        assert_eq!(cols.len(), 1, "{cols:?}\n{text}");
        assert!(text.contains("\nModel and agent\n"), "{text}");
    }

    #[test]
    fn commands_describe_the_picker_forms() {
        let text = commands(80, true);
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
