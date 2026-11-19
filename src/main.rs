mod agent;
mod api;
mod config;
mod help;
mod http;
mod images;
mod init;
mod jobs;
mod perm;
mod pricing;
mod prompt;
mod render;
mod store;
mod theme;
mod tools;
mod transport;
mod tui;

use agent::{Hooks, Session, Ui};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;


fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("imp: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{}", help::usage(terminal_width()));
        return Ok(());
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("imp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.first().map(|a| a.as_str()) == Some("prompt") {
        print!("{}", prompt::BASELINE);
        return Ok(());
    }
    if args.first().map(|a| a.as_str()) == Some("init") {
        let report = init::run()?;
        init::print(&report);
        return Ok(());
    }

    let mut cfg = config::Config::load(&config::Config::config_path())?;
    let mut prompt_words: Vec<String> = Vec::new();
    let mut mode = "auto";
    let mut agent_name: Option<String> = None;
    let mut resume: Option<String> = None;
    let mut continue_last = false;
    let mut cwd: Option<String> = None;
    let mut list = false;
    let mut rename: Option<(String, String)> = None;
    let mut delete: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => cfg.model = next(&args, &mut i, "--model")?,
            "--max-steps" => {
                cfg.max_steps = config::parse_max_steps(&next(&args, &mut i, "--max-steps")?)?
            }
            "--permissions" => {
                cfg.permissions = config::Permissions::parse(&next(&args, &mut i, "--permissions")?)?
            }
            "--agent" => agent_name = Some(next(&args, &mut i, "--agent")?),
            "-c" | "--continue" => continue_last = true,
            "-r" | "--resume" => resume = Some(next(&args, &mut i, "--resume")?),
            "-l" | "--list" => list = true,
            "--rename" => {
                let id = next(&args, &mut i, "--rename")?;
                let title = next(&args, &mut i, "--rename")?;
                rename = Some((id, title));
            }
            "--delete" => delete = Some(next(&args, &mut i, "--delete")?),
            "--cli" => mode = "cli",
            "--skills-dir" => {
                cfg.skills_dir = Some(PathBuf::from(next(&args, &mut i, "--skills-dir")?))
            }
            "-C" | "--cwd" => cwd = Some(next(&args, &mut i, "--cwd")?),
            other if other.starts_with('-') && other.len() > 1 => {
                return Err(format!("unknown option {other}"))
            }
            other => prompt_words.push(other.to_string()),
        }
        i += 1;
    }

    let prompt = prompt_from(&prompt_words);

    let sessions_dir = config::dirs_state().join("imp").join("sessions");
    let launch = std::env::current_dir().map_err(|e| e.to_string())?;

    if list {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        for m in store::Log::list(&sessions_dir) {
            println!("{}", store::format_session(&m, now));
        }
        return Ok(());
    }
    if let Some((id, title)) = rename {
        store::Log::rename(&sessions_dir, &id, &title)?;
        println!("renamed {id}");
        return Ok(());
    }
    if let Some(id) = delete {
        store::Log::delete(&sessions_dir, &id)?;
        println!("deleted {id} (moved to trash)");
        return Ok(());
    }

    if cfg.api_key.is_empty() {
        return Err("DEEPSEEK_API_KEY is not set".into());
    }

    let existing = match &resume {
        Some(id) => store::Log::open(&sessions_dir, id),
        None if continue_last => store::Log::latest(&sessions_dir),
        None => None,
    };
    let recorded = existing.as_ref().and_then(|l| l.meta_cwd());
    let (root, gone) = resume_root(cwd.as_deref(), recorded, &launch)?;
    if let Some(path) = gone {
        eprintln!(
            "imp: session directory {} is gone; using {}",
            path.display(),
            launch.display()
        );
    }

    let out_dir = config::dirs_state().join("imp").join("out");
    tools::prune_out(&out_dir, std::time::Duration::from_secs(7 * 24 * 3600));
    let system = system_prompt(&root, &cfg);
    let tui_cfg = cfg.clone();
    let roster = store::load_agents(&cfg, &root);
    let agent = match &agent_name {
        Some(name) => roster
            .iter()
            .find(|a| &a.name == name)
            .cloned()
            .ok_or_else(|| format!("unknown agent {name}"))?,
        None => roster
            .iter()
            .find(|a| a.name == "default")
            .cloned()
            .unwrap_or_else(store::Agent::builtin),
    };
    let cols = std::sync::Arc::new(std::sync::atomic::AtomicU16::new(
        std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    ));
    let mut session = Session::new(cfg, root.clone(), out_dir, system, agent, roster, cols);

    match existing {
        Some(log) => {
            let id = log.id.clone();
            let count = log.load().len();
            session.resume_id(&sessions_dir, &id)?;
            eprintln!("imp: resumed {id} ({count} messages, agent {})", session.agent.name);
        }
        None => {
            session.attach_log(store::Log::create(&sessions_dir, &root, &session.cfg.model));
        }
    }

    let (result, id) = match prompt {
        Some(text) => {
            let mut session = session;
            let result = session.turn(&text, &mut Cli { verbose: false, palette: tui::Palette::detect(&tui_cfg) });
            let id = session.log.as_ref().map(|l| l.id.clone());
            (result, id)
        }
        None => {
            let interactive = unsafe { isatty(0) } != 0 && unsafe { isatty(1) } != 0;
            let chat = wants_chat(mode, interactive);
            if chat {
                match tui::run(session, tui_cfg) {
                    Ok(id) => (Ok(()), id),
                    Err(e) => (Err(e), None),
                }
            } else {
                let mut session = session;
                let mut cli = Cli { verbose: false, palette: tui::Palette::detect(&tui_cfg) };
                let result = repl(&mut session, &mut cli, &sessions_dir);
                let id = session.log.as_ref().map(|l| l.id.clone());
                (result, id)
            }
        }
    };
    if let Some(id) = id {
        eprintln!("{}", resume_line(&id));
    }
    result
}

/// The line printed when a session ends, naming the id `imp -r` accepts.
fn resume_line(id: &str) -> String {
    format!("imp: resume with imp -r {id}")
}

/// Choose the root for a run: an explicit `--cwd` wins, then the directory
/// recorded with the session, then the launch directory. The second value is a
/// recorded directory that no longer exists, for the caller to report.
fn resume_root(
    cwd: Option<&str>,
    recorded: Option<PathBuf>,
    launch: &std::path::Path,
) -> Result<(PathBuf, Option<PathBuf>), String> {
    if let Some(raw) = cwd {
        let root = config::resolve_root(Some(raw), launch)
            .map_err(|e| format!("--cwd {e}"))?;
        return Ok((root, None));
    }
    match recorded {
        Some(path) if path.is_dir() => Ok((path, None)),
        Some(path) => Ok((launch.to_path_buf(), Some(path))),
        None => Ok((launch.to_path_buf(), None)),
    }
}

fn terminal_width() -> usize {
    #[repr(C)]
    struct Winsize {
        rows: u16,
        cols: u16,
        x: u16,
        y: u16,
    }
    extern "C" {
        fn ioctl(fd: std::ffi::c_int, request: std::ffi::c_ulong, ...) -> std::ffi::c_int;
    }
    #[cfg(target_os = "macos")]
    const TIOCGWINSZ: std::ffi::c_ulong = 0x4008_7468;
    #[cfg(not(target_os = "macos"))]
    const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;
    let mut ws = Winsize {
        rows: 0,
        cols: 0,
        x: 0,
        y: 0,
    };
    let ok = unsafe { ioctl(1, TIOCGWINSZ, &mut ws as *mut Winsize) } == 0;
    if ok && ws.cols > 0 {
        ws.cols as usize
    } else {
        100
    }
}

extern "C" {
    fn isatty(fd: std::ffi::c_int) -> std::ffi::c_int;
}

/// Read one entry, accepting either Enter encoding.
///
/// `read_line` waits for a line feed, but a terminal with its CR-to-NL
/// mapping off sends a bare carriage return. Stop at either, treat CRLF as
/// one ending, and return `None` only when input ends before anything arrives.
fn read_entry<R: BufRead>(r: &mut R) -> std::io::Result<Option<String>> {
    let mut line = Vec::new();
    let mut saw_input = false;
    let mut byte = [0u8; 1];
    loop {
        match r.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => saw_input = true,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
        match byte[0] {
            b'\n' => break,
            b'\r' => {
                let followed_by_lf = matches!(r.fill_buf(), Ok(b) if b.starts_with(b"\n"));
                if followed_by_lf {
                    let _ = r.read(&mut byte);
                }
                break;
            }
            b => line.push(b),
        }
    }
    if !saw_input {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&line).into_owned()))
}

fn repl(session: &mut Session, cli: &mut Cli, sessions_dir: &std::path::Path) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let tty = unsafe { isatty(0) } != 0;
    let _line_mode = if tty { Some(tui::LineMode::enter()) } else { None };
    loop {
        if tty {
            print!("{}", cli.palette.paint(tui::Role::User, "> "));
            out.flush().ok();
        }
        let line = match read_entry(&mut stdin.lock()).map_err(|e| e.to_string())? {
            Some(line) => line,
            None => {
                if tty {
                    println!();
                }
                return Ok(());
            }
        };
        if tty {
            println!();
        }
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        if let Some(cmd) = text.strip_prefix('/') {
            match cmd {
                "quit" | "exit" => return Ok(()),
                "help" => println!("{}", help::commands()),
                "new" => {
                    let system = session
                        .history
                        .iter()
                        .find(|m| m.role == api::Role::System)
                        .cloned();
                    session.history.clear();
                    session.tools.clear_buffers();
                    if let Some(m) = system {
                        session.history.push(m);
                    }
                    println!("new conversation");
                }
                "compact" => {
                    if let Err(e) = session.compact(cli) {
                        eprintln!("imp: {e}");
                    }
                }
                "think" => {
                    session.cfg.thinking = !session.cfg.thinking;
                    println!("thinking {}", if session.cfg.thinking { "on" } else { "off" });
                }
                "yolo" => {
                    let line = session.toggle_yolo();
                    println!("{line} ({})", session.cfg.permissions.as_str());
                }
                "reload" => match session.reload() {
                    Ok(()) => println!("reloaded; prompt cache reset"),
                    Err(e) => eprintln!("imp: {e}"),
                },
                other if other == "agent" || other.starts_with("agent ") => {
                    let rest = other.strip_prefix("agent").unwrap_or("").trim();
                    match rest {
                        "" => {
                            println!("agent: {}", session.agent.name);
                            for a in &session.roster {
                                println!("  {:<12} {}", a.name, a.description);
                            }
                        }
                        "current" => println!(
                            "{}",
                            agent::describe_agent(&session.agent, &session.cfg)
                        ),
                        name => {
                            let found = session
                                .roster
                                .iter()
                                .find(|a| a.name == name)
                                .cloned();
                            match found {
                                Some(agent) => {
                                    let applied = session.switch_agent(agent);
                                    println!("agent: {applied}");
                                }
                                None => {
                                    let names: Vec<String> =
                                        session.roster.iter().map(|a| a.name.clone()).collect();
                                    println!("no agent {name}; available: {}", names.join(", "));
                                }
                            }
                        }
                    }
                }
                "sessions" => {
                    for m in store::Log::list(&sessions_dir) {
                        println!("{:<12} {:<40} {:>5} msgs", m.id, m.title, m.messages);
                    }
                }
                "cwd" => println!("{}", session.tools.root.display()),
                "fork" => match session.fork() {
                    Ok((new_id, old_id)) => println!("fork: {new_id} (from {old_id})"),
                    Err(e) => println!("{e}"),
                },
                other if other == "theme" || other.starts_with("theme ") => {
                    let rest = other.strip_prefix("theme").unwrap_or("").trim();
                    if rest.is_empty() {
                        let current = cli.palette.theme_name();
                        let names: Vec<String> = theme::names()
                            .into_iter()
                            .map(|n| if n == current { format!("*{n}") } else { n.to_string() })
                            .collect();
                        println!("{}", names.join("  "));
                    } else if theme::by_name(rest).is_some() {
                        match theme::save(rest) {
                            Ok(()) => {
                                cli.palette = tui::Palette::detect(&session.cfg);
                                println!("theme: {rest}");
                            }
                            Err(e) => eprintln!("imp: {e}"),
                        }
                    } else {
                        println!("no theme {rest}; use {}", theme::names().join(", "));
                    }
                }
                other if other == "image" || other.starts_with("image ") => {
                    let raw = other.strip_prefix("image").unwrap_or("").trim();
                    if raw.is_empty() {
                        println!("image <path>");
                    } else {
                        match images::attach(raw, &session.tools.root) {
                            Ok(image) => {
                                println!("{}", images::marker(&image));
                                if let Err(e) = session.attach_image(image) {
                                    eprintln!("imp: {e}");
                                }
                            }
                            Err(e) => println!("{e}"),
                        }
                    }
                }
                other if other == "cd" || other.starts_with("cd ") => {
                    let raw = other[2..].trim();
                    if raw.is_empty() {
                        println!("cd <path>");
                    } else {
                        match config::resolve_root(Some(raw), &session.tools.root) {
                            Ok(path) => match session.set_root(path.clone()) {
                                Ok(()) => println!(
                                    "cwd: {} (prompt cache reset)",
                                    path.display()
                                ),
                                Err(e) => eprintln!("imp: {e}"),
                            },
                            Err(e) => println!("{e}"),
                        }
                    }
                }
                other if other == "verbosity" || other.starts_with("verbosity ") => {
                    match other.strip_prefix("verbosity").unwrap_or("").trim() {
                        "" => println!("verbosity: {}", if cli.verbose { "high" } else { "low" }),
                        "high" => {
                            cli.verbose = true;
                            println!("verbosity: high");
                        }
                        "low" => {
                            cli.verbose = false;
                            println!("verbosity: low");
                        }
                        level => println!("unknown verbosity {level:?}; use low or high"),
                    }
                }
                other => println!("{}", help::unknown(other)),
            }
            continue;
        }
        if let Err(e) = session.turn(text, cli) {
            eprintln!("imp: {e}");
        }
        println!();
    }
}

struct Cli {
    verbose: bool,
    palette: tui::Palette,
}

impl Hooks for Cli {
    fn ui(&mut self, event: Ui) {
        print_ui(event, self.verbose, &self.palette)
    }

    fn confirm(&mut self, what: &str) -> bool {
        confirm(what)
    }

    fn run_attached(&mut self, command: &str) -> Option<i32> {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .status()
            .ok()
            .map(|s| s.code().unwrap_or(-1))
    }

    fn ask(&mut self, question: &str, options: &[String]) -> Option<String> {
        println!("\n{question}");
        for (i, o) in options.iter().enumerate() {
            println!("  {}. {o}", i + 1);
        }
        print!("answer{}: ", if options.is_empty() { "" } else { " (number or text)" });
        std::io::stdout().flush().ok();
        let line = read_entry(&mut std::io::stdin().lock()).ok()??;
        let line = line.trim().to_string();
        if let Ok(n) = line.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return Some(options[n - 1].clone());
            }
        }
        Some(line)
    }
}

fn print_ui(event: Ui, verbose: bool, palette: &tui::Palette) {
    let mut out = std::io::stdout();
    print_ui_to(&mut out, event, verbose, palette).ok();
}

fn preview_lines(text: &str) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for line in text.lines().take(2) {
        if used > 0 {
            out.push('\n');
        }
        used += 1;
        out.push_str(line);
    }
    if text.lines().count() > 2 || out.chars().count() > 240 {
        out.truncate(
            out.char_indices()
                .take(240)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0),
        );
        if !out.contains("more") {
            out.push_str(" …");
        }
    }
    out
}

fn print_ui_to<W: Write>(
    out: &mut W,
    event: Ui,
    verbose: bool,
    palette: &tui::Palette,
) -> std::io::Result<()> {
    match event {
        Ui::Content(t) => {
            write!(out, "{t}")?;
            out.flush().ok();
        }
        Ui::Reasoning(t) => {
            let _ = t;
        }
        Ui::ToolStart { name, summary, args } => {
            let line = format!("\u{25b8} {summary}");
            writeln!(out, "\n{}", palette.paint(tui::Role::Tool, &line))?;
            if verbose && !args.is_empty() {
                for line in args.lines() {
                    writeln!(out, "  {}", palette.paint(tui::Role::Thinking, line))?;
                }
            }
            let _ = name;
        }
        Ui::ToolEnd { ok, preview, output, .. } => {
            let role = if ok { tui::Role::Tool } else { tui::Role::Error };
            let mark = if ok { "✓" } else { "✗" };
            let line = format!("{mark} {}", preview_lines(&preview));
            writeln!(out, "  {}", palette.paint(role, &line))?;
            if verbose && !output.is_empty() {
                for line in preview_lines(&output).lines() {
                    writeln!(out, "  {}", palette.paint(tui::Role::Thinking, line))?;
                }
            }
        }
        Ui::Info(t) => {
            let line = format!("\u{25e6} {t}");
            writeln!(out, "\n{}", palette.paint(tui::Role::Info, &line))?;
        }
        Ui::Error(t) => {
            let line = format!("! {t}");
            writeln!(out, "\n{}", palette.paint(tui::Role::Error, &line))?;
        }
    }
    Ok(())
}

fn confirm(prompt: &str) -> bool {
    print!("{prompt}? [y/N] ");
    std::io::stdout().flush().ok();
    let line = match read_entry(&mut std::io::stdin().lock()) {
        Ok(Some(line)) => line,
        _ => return false,
    };
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// `--cli` wins; otherwise a real terminal decides.
pub fn wants_chat(mode: &str, tty: bool) -> bool {
    mode != "cli" && tty
}

fn system_prompt(root: &std::path::Path, cfg: &config::Config) -> String {
    prompt::compose(root, cfg)
}

fn prompt_from(words: &[String]) -> Option<String> {
    if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
    }
}

fn next(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_selection() {
        assert!(wants_chat("auto", true));
        assert!(!wants_chat("auto", false));
        assert!(!wants_chat("cli", true));
    }

    #[test]
    fn positional_words_join_into_one_prompt() {
        let words = ["find", "the", "biggest", "file"].map(str::to_string);
        assert_eq!(
            prompt_from(&words).as_deref(),
            Some("find the biggest file")
        );
        assert_eq!(prompt_from(&[]), None);
    }

    #[test]
    fn resume_line_names_the_id() {
        assert_eq!(
            resume_line("3k9a0aa"),
            "imp: resume with imp -r 3k9a0aa"
        );
    }

    #[test]
    fn cwd_resolves_against_the_launch_directory() {
        let launch = std::env::temp_dir();
        let root = config::resolve_root(Some("."), &launch).unwrap();
        assert_eq!(root, launch.canonicalize().unwrap());
        assert!(config::resolve_root(Some("/nope/imp-missing"), &launch).is_err());
        assert_eq!(config::resolve_root(None, &launch).unwrap(), launch);
    }

    #[test]
    fn resume_root_prefers_explicit_then_recorded() {
        let launch = std::env::temp_dir();
        let recorded = std::env::temp_dir().join(format!("imp-rec-{}", std::process::id()));
        std::fs::create_dir_all(&recorded).unwrap();

        let (root, gone) = resume_root(None, Some(recorded.clone()), &launch).unwrap();
        assert_eq!(root, recorded);
        assert!(gone.is_none());

        let (root, _) = resume_root(Some("."), Some(recorded.clone()), &launch).unwrap();
        assert_eq!(root, launch.canonicalize().unwrap());

        let missing = launch.join("imp-recorded-missing");
        let (root, gone) = resume_root(None, Some(missing.clone()), &launch).unwrap();
        assert_eq!(root, launch);
        assert_eq!(gone, Some(missing));
        let _ = std::fs::remove_dir(recorded);
    }

    #[test]
    fn line_mode_high_shows_args_and_output() {
        let palette = tui::Palette::detect(&config::Config::default());
        let mut out = Vec::new();
        print_ui_to(
            &mut out,
            Ui::ToolStart {
                name: "bash".into(),
                summary: "run `ls`".into(),
                args: "{\"command\":\"ls\"}".into(),
            },
            true,
            &palette,
        )
        .unwrap();
        print_ui_to(
            &mut out,
            Ui::ToolEnd {
                ok: true,
                preview: "total 8".into(),
                output: "total 8\nfoo".into(),
                detail: String::new(),
                ms: 4,
            },
            true,
            &palette,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("{\"command\":\"ls\"}"), "{text}");
        assert!(text.contains("✓ total 8") && text.contains("  foo"), "{text}");

        let mut low = Vec::new();
        print_ui_to(
            &mut low,
            Ui::ToolStart {
                name: "bash".into(),
                summary: "run `ls`".into(),
                args: "{\"command\":\"ls\"}".into(),
            },
            false,
            &palette,
        )
        .unwrap();
        let low = String::from_utf8(low).unwrap();
        assert!(!low.contains("command"), "{low}");
    }

    #[test]
    fn entry_ends_on_carriage_return_or_line_feed() {
        use std::io::Cursor;

        let mut cr = Cursor::new(b"hello\r".to_vec());
        assert_eq!(read_entry(&mut cr).unwrap().as_deref(), Some("hello"));

        let mut mixed = Cursor::new(b"a\r\nb\n".to_vec());
        assert_eq!(read_entry(&mut mixed).unwrap().as_deref(), Some("a"));
        assert_eq!(read_entry(&mut mixed).unwrap().as_deref(), Some("b"));
        assert_eq!(read_entry(&mut mixed).unwrap(), None);

        let mut empty = Cursor::new(Vec::new());
        assert_eq!(read_entry(&mut empty).unwrap(), None);
    }
}
