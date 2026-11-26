//! Help text. One renderer, used by `--help`, `/help`, and line mode.

/// Keys in the full-screen chat.
pub const KEYS: &[(&str, &str)] = &[
    ("Enter", "send; during a turn, queue the message"),
    ("Ctrl+S", "steer now; send when idle"),
    ("Shift+Enter, Alt+Enter, Ctrl+J", "newline"),
    ("Esc", "stop the turn or close a picker"),
    ("Ctrl+C", "stop the turn; idle, twice to quit (the draft stays)"),
    ("Ctrl+D", "quit, from anywhere"),
    ("Tab", "complete a command"),
    ("Shift+Tab", "next agent, with or without a draft"),
    ("Ctrl+B", "next mode: normal, strict, yolo"),
    ("Right", "take the suggestion"),
    ("Up, Down", "queued messages, then history"),
    ("Ctrl+X", "delete the open queued message"),
    ("Shift+arrows, Shift+Home, Shift+End", "select in the draft"),
    ("Delete", "delete forward"),
    ("Option+Left, Option+Right", "move by word; Alt+B, Alt+F too"),
    ("Option+Backspace, Ctrl+W", "delete the word before"),
    ("Option+Delete, Alt+D", "delete the word after"),
    ("Shift+Option+Left, Shift+Option+Right", "select by word"),
    ("Ctrl+A", "start of the row"),
    ("Ctrl+U, Ctrl+K", "delete to the row's start or end"),
    ("Ctrl+L", "clear the screen"),
    ("Ctrl+E", "expand the latest turn's tool rows"),
    ("Ctrl+R", "send the last message again"),
    ("Ctrl+T", "thinking on or off for the next turn"),
    ("Ctrl+V", "paste a clipboard image, or the last copy"),
    ("Ctrl+G", "release the mouse to select text"),
    ("Ctrl+O", "next session waiting on you"),
    ("Ctrl+N", "next live session"),
    ("?", "this list, from an empty draft; also /keys"),
    ("PgUp, PgDn", "scroll"),
    ("Picker", "type to filter, Enter choose, Esc cancel"),
];

/// Keys in line mode, which reads whole lines.
const LINE_KEYS: &[(&str, &str)] = &[("Enter", "send"), ("Ctrl+D", "leave")];

/// Commands by area, named from `COMMANDS`.
const GROUPS: &[(&str, &[&str])] = &[
    ("Conversation", &["/new", "/goal", "/compact", "/retry", "/think", "/queue", "/copy"]),
    ("Model and agent", &["/model", "/provider", "/agent", "/mode", "/effort", "/reload"]),
    ("Workspace", &["/cwd", "/cd", "/image", "/shell", "/edit", "/open", "/skills", "/diff"]),
    ("Session", &["/sessions", "/next", "/fork", "/rename", "/quit"]),
    ("Interface", &["/help", "/keys", "/theme", "/verbosity", "/settings"]),
];

pub const COMMANDS: &[(&str, &str)] = &[
    ("/help", "show the commands"),
    ("/keys", "show the keys"),
    ("/new", "start another live session"),
    ("/sessions", "pick a session"),
    ("/next", "next session waiting on you"),
    ("/reload", "re-read the config, instructions and skills"),
    ("/agent", "switch agent profile"),
    ("/compact", "compact the conversation now"),
    ("/goal", "keep working until a goal is met; off to stop"),
    ("/retry", "send the last message again, dropping its reply"),
    ("/think", "toggle thinking for the next turn"),
    ("/effort", "reasoning effort: low, medium, high, or default"),
    ("/mode", "normal, strict, or yolo for this session"),
    ("/verbosity", "step detail: low, medium, high, or pick"),
    ("/cwd", "show the session directory"),
    ("/cd", "browse or change the directory"),
    ("/image", "browse or attach an image"),
    ("/fork", "copy the session and continue there"),
    ("/theme", "set the colour theme"),
    ("/settings", "change settings in a form"),
    ("/shell", "open a shell"),
    ("/edit", "edit a file or folder in the terminal editor"),
    ("/open", "open a file or folder in the GUI editor"),
    ("/skills", "turn each skill on or off; applies on /reload"),
    ("/diff", "what imp changed this session; git for git diff"),
    ("/model", "set or pick the session model"),
    ("/provider", "providers: space on/off, u use, e edit, d delete, n new"),
    ("/queue", "messages waiting to send: add, pop, delete, clear"),
    ("/copy", "copy the last reply; code for its last block"),
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

/// Fewest description columns before descriptions move under their cells.
const MIN_TEXT: usize = 24;

/// Named sections of `(left, description)` rows.
type Sections<'a> = [(&'a str, Vec<(&'a str, &'a str)>)];

/// Render sections with one description column for the whole output: two
/// spaces after the longest left cell, which may be at most `LEFT_CAP` wide.
/// A longer cell keeps its own row and its description starts on the next.
/// Long-only flags sit past the short-flag column, a `(default: X)` takes its
/// own row, and examples put their description above the command. When
/// fewer than `MIN_TEXT` columns are left, every description goes under its
/// cell instead. `color` adds bold headings, accent cells, and dim notes.
fn table(out: &mut String, sections: &Sections, width: usize, color: bool) {
    let paint = |code: &str, text: &str| if color { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() };
    let cell_of = |c: &str| if c.starts_with("--") { format!("    {c}") } else { c.to_string() };
    let left = sections
        .iter()
        .filter(|(title, _)| *title != "EXAMPLES")
        .flat_map(|(_, rows)| rows.iter().map(|(l, _)| cell_of(l).chars().count()))
        .filter(|&n| n <= LEFT_CAP)
        .max()
        .unwrap_or(0);
    let mut col = 2 + left + 2;
    if width.saturating_sub(col) < MIN_TEXT {
        col = 6;
    }
    let stacked = col == 6;
    let text = width.saturating_sub(col).max(20);
    for (i, (title, rows)) in sections.iter().enumerate() {
        if i > 0 || !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&paint("1", title));
        out.push('\n');
        for (cell, desc) in rows {
            if *title == "EXAMPLES" {
                for line in wrap(&format!("# {desc}"), width.saturating_sub(2)) {
                    out.push_str(&format!("  {}\n", paint("2", &line)));
                }
                out.push_str(&format!("  {cell}\n"));
                continue;
            }
            let (desc, default) = match desc.split_once(" (default: ") {
                Some((d, rest)) => (d, Some(format!("(default: {rest}"))),
                None => (*desc, None),
            };
            let cell = cell_of(cell);
            let len = cell.chars().count();
            let mut parts = wrap(desc, text).into_iter();
            if stacked || len > left {
                out.push_str(&format!("  {}\n", paint("36", &cell)));
            } else if let Some(first) = parts.next() {
                out.push_str(&format!("  {}{}{first}\n", paint("36", &cell), " ".repeat(col - 2 - len)));
            }
            for rest in parts {
                out.push_str(&format!("{}{rest}\n", " ".repeat(col)));
            }
            if let Some(default) = default {
                out.push_str(&format!("{}{}\n", " ".repeat(col), paint("2", &default)));
            }
        }
    }
}

/// `--help`. Width comes from the terminal so long descriptions wrap.
pub fn usage(width: usize, color: bool) -> String {
    let width = width.clamp(40, 110);
    let mut out = String::from("imp · a small terminal agent\n");
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
                    ("-r, --resume [ID]", "resume a session; the newest without an id"),
                    ("-l, --list", "list sessions and exit"),
                    ("--rename ID TITLE", "rename a session"),
                    ("--delete ID", "move a session to trash"),
                ],
            ),
            (
                "RUNNING",
                vec![
                    ("--agent NAME", "agent profile to start with (default: default)"),
                    (
                        "--provider NAME",
                        "deepseek, openai, anthropic, gemini, ollama, or custom (default: deepseek)",
                    ),
                    ("--model NAME", "model to use (default: deepseek-flash)"),
                    ("--max-steps N", "tool steps per turn (default: 24)"),
                    ("--permissions MODE", "normal, strict, or yolo (default: normal)"),
                    ("--skills-dir PATH", "extra skills directory"),
                    ("-C, --cwd PATH", "session directory (default: launch directory)"),
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
        color,
    );
    out.push('\n');
    for line in wrap("In the chat, a line starting with `!` runs as a shell command.", width) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// The name a command shows in `/help`. `/compact` takes optional focus text,
/// so its row shows the argument form; the name itself stays the lookup key.
fn shown(name: &str) -> &str {
    if name == "/compact" {
        "/compact [focus]"
    } else {
        name
    }
}

/// `/help`, wrapped to `width`, with the keys of the chat or of line mode.
/// `color` gives bold titles and cells in SGR 36, which the chat swaps for its accent.
pub fn commands(width: usize, chat: bool, color: bool) -> String {
    let desc = |name: &str| {
        COMMANDS
            .iter()
            .find(|(n, _)| *n == name)
            .map_or("", |(_, d)| *d)
    };
    // The chat's keys have their own card, `?` or `/keys`; line mode has two.
    let mut sections = if chat { Vec::new() } else { vec![("Keys", LINE_KEYS.to_vec())] };
    for (title, names) in GROUPS {
        sections.push((title, names.iter().map(|n| (shown(n), desc(n))).collect()));
    }
    let mut out = String::new();
    table(&mut out, &sections, width, color);
    out.push('\n');
    let note = "Anything else goes to the model. Prefix a line with `!` to run it as a shell command.";
    out.push_str(&wrap(note, width).join("\n"));
    if chat {
        out.push_str("\nKeys: ? or /keys");
    }
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
        let text = usage(100, false);
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
        let text = usage(100, false);
        assert!(text.contains("(default: normal)"));
        assert!(text.contains("(default: deepseek-flash)"));
        assert!(text.contains("EXAMPLES"));
        assert!(text.contains("imp -c"));
        assert!(text.contains("-r, --resume [ID]"));
        assert!(!text.contains("mote"));
    }

    #[test]
    fn usage_names_every_provider_key() {
        let text = usage(100, false);
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
        let text = usage(80, false);
        for line in text.lines() {
            assert!(line.chars().count() <= 80, "too wide: {line:?}");
        }
    }

    /// Column where a row's description starts, if it has one on that row.
    fn desc_col(line: &str) -> Option<usize> {
        let body = line.strip_prefix("  ")?;
        // Long-only flags are indented; their cell starts past that indent.
        let indent = body.len() - body.trim_start().len();
        let gap = indent + body[indent..].find("  ")?;
        let rest = &body[gap..];
        Some(2 + gap + rest.len() - rest.trim_start().len())
    }

    #[test]
    fn usage_shares_one_column_and_fits_sixty() {
        let text = usage(60, false);
        let mut cols = std::collections::BTreeSet::new();
        for line in text.lines() {
            assert!(line.chars().count() <= 60, "too wide: {line:?}");
            if let Some(c) = desc_col(line) {
                cols.insert(c);
            }
        }
        assert_eq!(cols.len(), 1, "{cols:?}\n{text}");
        // Long-only flags line up past the short-flag column.
        assert!(text.contains("\n  -c, --continue    "), "{text}");
        assert!(text.contains("\n      --rename ID TITLE "), "{text}");
    }

    #[test]
    fn defaults_take_their_own_row() {
        let text = usage(54, false);
        let at = text.find("--model NAME").unwrap();
        let next = text[at..].lines().nth(1).unwrap();
        assert_eq!(next.trim(), "(default: deepseek-flash)", "{text}");
        assert!(!text.lines().any(|l| l.trim_end().ends_with("(default:")), "{text}");
    }

    #[test]
    fn fits_the_hackberry_and_narrower() {
        for width in [54, 40] {
            let text = usage(width, false);
            for line in text.lines() {
                assert!(line.chars().count() <= width, "too wide at {width}: {line:?}");
            }
        }
        // At 40 there is no room for a column, so descriptions go under.
        let text = usage(40, false);
        let at = text.find("  -l, --list\n").expect("cell on its own row");
        assert_eq!(text[at..].lines().nth(1).unwrap(), "      list sessions and exit");
    }

    #[test]
    fn examples_put_the_description_first() {
        let text = usage(80, false);
        assert!(text.contains("  # one request, then exit\n  imp \"why is the build failing?\"\n"), "{text}");
    }

    #[test]
    fn colour_only_when_asked() {
        assert!(!usage(80, false).contains('\x1b'));
        let painted = usage(80, true);
        assert!(painted.contains("\x1b[1mUSAGE\x1b[0m"), "{painted}");
    }

    #[test]
    fn keys_live_on_their_own_card() {
        let text = commands(80, true, false);
        assert!(!text.contains("Ctrl+C") && !text.contains("\nKeys\n"), "{text}");
        assert!(text.trim_end().ends_with("Keys: ? or /keys"), "{text}");
        let keys: Vec<&str> = KEYS.iter().map(|(k, _)| *k).collect();
        for k in ["Ctrl+S", "Shift+Tab", "Ctrl+C", "PgUp, PgDn", "Shift+Enter, Alt+Enter, Ctrl+J", "Ctrl+X"] {
            assert!(keys.contains(&k), "{k} is not on the card");
        }
    }

    #[test]
    fn line_mode_lists_only_its_own_keys() {
        let text = commands(80, false, false);
        for key in ["Picker", "PgUp", "Tab", "Ctrl+E", "Esc"] {
            assert!(!text.contains(key), "{key} in line mode:\n{text}");
        }
        assert!(text.contains("Ctrl+D"), "{text}");
        assert!(text.contains("/compact"), "{text}");
    }

    #[test]
    fn compact_help_line_shows_the_focus_argument() {
        let text = commands(80, true, false);
        let line = text
            .lines()
            .find(|l| l.contains("compact the conversation now"))
            .unwrap_or_else(|| panic!("no /compact line:\n{text}"));
        assert!(line.contains("/compact [focus]"), "{line}");
    }

    #[test]
    fn every_command_sits_in_one_group() {
        assert_eq!(COMMANDS.len(), 31);
        for (name, _) in COMMANDS {
            let n = GROUPS.iter().filter(|(_, g)| g.contains(name)).count();
            assert_eq!(n, 1, "{name} is in {n} groups");
        }
        let text = commands(60, true, false);
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
        let text = commands(80, true, false);
        for phrase in [
            "pick a session",
            "low, medium, high, or pick",
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
