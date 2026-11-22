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
mod providers;
mod render;
mod secrets;
mod store;
mod theme;
mod tools;
mod transport;
mod tui;
mod web;

use agent::{Hooks, Session, Ui};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;


fn main() -> ExitCode {
    let result = run();
    // ponytail: a signal exit skips this; the next start sweeps the folder.
    if let Some(dir) = tools::scratch() {
        let _ = std::fs::remove_dir_all(dir);
    }
    match result {
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
        // Colour only on a terminal, and never with NO_COLOR.
        let color = unsafe { isatty(1) } != 0 && std::env::var_os("NO_COLOR").is_none();
        print!("{}", help::usage(terminal_width(), color));
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
    secrets::keep(&cfg.pass_env);
    render::set_renderers(&cfg.render);
    secrets::also(&cfg.auth.iter().map(|(_, header)| header.clone()).collect::<Vec<_>>());
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
            "--provider" => {
                cfg.set_provider(providers::Provider::parse(&next(&args, &mut i, "--provider")?)?)
            }
            "--max-steps" => {
                cfg.max_steps = config::parse_max_steps(&next(&args, &mut i, "--max-steps")?)?
            }
            "--permissions" => {
                let mode =
                    config::Permissions::parse(&next(&args, &mut i, "--permissions")?)?;
                cfg.set_permissions(mode);
            }
            "--agent" => agent_name = Some(next(&args, &mut i, "--agent")?),
            "-c" | "--continue" => continue_last = true,
            // With no id after it, or a flag next, -r resumes the newest session.
            "-r" | "--resume" => match resume_id(&args, i) {
                Some(id) => {
                    resume = Some(id.clone());
                    i += 1;
                }
                None => continue_last = true,
            },
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
                return Err(format!("unknown option {other}; try imp --help"))
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
        let metas = store::Log::list(&sessions_dir);
        print!("{}", store::session_table(&metas, now, terminal_width()));
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

    // A chat can ask for a key; a line-mode run has nobody to ask.
    let interactive = unsafe { isatty(0) } != 0 && unsafe { isatty(1) } != 0;
    let chat = prompt.is_none() && wants_chat(mode, interactive);
    let missing_key = key_gate(chat, cfg.resolve_key_env(config::key_lookup()))?;

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
    if let Err(e) = tools::make_scratch(&out_dir) {
        eprintln!("imp: no scratch folder: {e}");
    }
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
    let mut session = Session::new(cfg, root.clone(), out_dir.clone(), system.clone(), agent, roster, cols);

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
            let start = std::time::Instant::now();
            let result = session.turn(&text, &mut Cli { verbosity: tui_cfg.verbosity, palette: tui::Palette::detect(&tui_cfg) });
            finish_line(&session, start, tui_cfg.verbosity);
            let id = session.log.as_ref().map(|l| l.id.clone());
            (result, id)
        }
        None => {
            if chat {
                // Builds each further live session, with its own mode and buffers.
                let fresh: tui::Fresh = {
                    let (cfg, out_dir, system) = (session.cfg.detached(), out_dir.clone(), system.clone());
                    let (agent, roster, cols) = (session.agent.clone(), session.roster.clone(), session.cols.clone());
                    Box::new(move |root: &std::path::Path| {
                        Session::new(
                            cfg.detached(),
                            root.to_path_buf(),
                            out_dir.clone(),
                            system.clone(),
                            agent.clone(),
                            roster.clone(),
                            cols.clone(),
                        )
                    })
                };
                match tui::run(session, tui_cfg, fresh, missing_key) {
                    Ok(id) => (Ok(()), id),
                    Err(e) => (Err(e), None),
                }
            } else {
                let mut session = session;
                let mut cli = Cli { verbosity: tui_cfg.verbosity, palette: tui::Palette::detect(&tui_cfg) };
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

/// A missing key stops line mode; the chat gets a chance to ask for one.
fn key_gate(chat: bool, resolved: Result<(), String>) -> Result<Option<String>, String> {
    match resolved {
        Ok(()) => Ok(None),
        Err(var) if chat => Ok(Some(var)),
        Err(var) => Err(format!(
            "{var} is not set; run imp in a terminal to enter a key, or export it"
        )),
    }
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
    let mut stash: Vec<String> = Vec::new();
    loop {
        if tty {
            // Yolo shows by colour alone.
            let yolo = session.cfg.live_permissions() == config::Permissions::Yolo;
            let tone = if yolo { tui::Role::Error } else { tui::Role::User };
            print!("{}", cli.palette.paint(tone, "> "));
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
                "help" => println!("{}", help::commands(terminal_width(), false)),
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
                    println!("thinking: {}", if session.cfg.thinking { "on" } else { "off" });
                }
                other if other == "mode" || other.starts_with("mode ") => {
                    match other.strip_prefix("mode").unwrap_or("").trim() {
                        "" => print!(
                            "{}",
                            value_rows(
                                &[
                                    ("normal".into(), "unlisted commands ask".into()),
                                    ("strict".into(), "every action asks".into()),
                                    ("yolo".into(), "never asks; deny list holds".into()),
                                ],
                                session.cfg.live_permissions().as_str(),
                            )
                        ),
                        name => match config::Permissions::parse(name) {
                            Ok(mode) => {
                                session.set_permissions(mode);
                                println!("{}", perm::mode_line(mode));
                            }
                            Err(_) => eprintln!("unknown mode {name}; use normal, strict, or yolo"),
                        },
                    }
                }
                "next" => println!("line mode runs one session; /next is in the chat"),
                "reload" => match session.reload() {
                    Ok(()) => println!("reloaded; prompt cache reset"),
                    Err(e) => eprintln!("imp: {e}"),
                },
                other if other == "agent" || other.starts_with("agent ") => {
                    let rest = other.strip_prefix("agent").unwrap_or("").trim();
                    match rest {
                        "" => {
                            let rows: Vec<(String, String)> = session
                                .roster
                                .iter()
                                .map(|a| (a.name.clone(), a.description.clone()))
                                .collect();
                            print!("{}", value_rows(&rows, &session.agent.name));
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
                other if other.starts_with("sessions ") => {
                    let id = other["sessions".len()..].trim();
                    match session.resume_id(sessions_dir, id) {
                        Ok(()) => println!("resumed {id} ({} messages)", session.history.len()),
                        Err(e) => eprintln!("imp: {e}"),
                    }
                }
                "sessions" => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let mut metas = store::Log::list(sessions_dir);
                    metas.truncate(20);
                    print!("{}", store::session_table(&metas, now, terminal_width()));
                }
                "cwd" => println!("{}", session.tools.root.display()),
                "fork" => match session.fork() {
                    Ok((new_id, old_id)) => println!("fork: {new_id} (from {old_id})"),
                    Err(e) => println!("{e}"),
                },
                other if other == "theme" || other.starts_with("theme ") => {
                    let rest = other.strip_prefix("theme").unwrap_or("").trim();
                    if rest.is_empty() {
                        let rows: Vec<(String, String)> =
                            theme::names().into_iter().map(|n| (n.into(), String::new())).collect();
                        print!("{}", value_rows(&rows, cli.palette.theme_name()));
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
                                Ok(()) => println!("{}", tui::root_report(&path)),
                                Err(e) => eprintln!("imp: {e}"),
                            },
                            Err(e) => println!("{e}"),
                        }
                    }
                }
                other if other == "verbosity" || other.starts_with("verbosity ") => {
                    match other.strip_prefix("verbosity").unwrap_or("").trim() {
                        "" => print!(
                            "{}",
                            value_rows(
                                &config::Verbosity::ALL.map(|(l, about)| (l.as_str().to_string(), about.to_string())),
                                cli.verbosity.as_str(),
                            )
                        ),
                        level => match config::Verbosity::parse(level) {
                            Ok(v) => {
                                cli.verbosity = v;
                                println!("verbosity: {}", v.as_str());
                            }
                            Err(e) => println!("{e}"),
                        },
                    }
                }
                "shell" => {
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
                    match std::process::Command::new(shell).current_dir(&session.tools.root).status() {
                        Ok(status) => {
                            println!("shell exited {}", status.code().unwrap_or(-1));
                        }
                        Err(e) => eprintln!("imp: {e}"),
                    }
                }
                other if other == "rename" || other.starts_with("rename ") => {
                    let title = other.strip_prefix("rename").unwrap_or("").trim();
                    if title.is_empty() {
                        println!("rename <title>");
                    } else {
                        match session.log.as_ref().map(|log| log.retitle(title)) {
                            Some(Ok(())) => println!("renamed to {title:?}"),
                            Some(Err(e)) => println!("{e}"),
                            None => println!("nothing to rename yet"),
                        }
                    }
                }
                other if other == "model" || other.starts_with("model ") => {
                    let name = other.strip_prefix("model").unwrap_or("").trim();
                    if name.is_empty() {
                        let default = session.cfg.provider.profile().model.to_string();
                        let mut names = vec![session.cfg.model.clone(), default.clone()];
                        names.extend(providers::catalog(session.cfg.provider));
                        let mut rows: Vec<(String, String)> = Vec::new();
                        for n in names {
                            if !rows.iter().any(|(r, _)| *r == n) {
                                let about = if n == default { "default" } else { "" };
                                rows.push((n, about.into()));
                            }
                        }
                        print!("{}", value_rows(&rows, &session.cfg.model));
                    } else {
                        session.cfg.model = name.to_string();
                        println!("model: {name}");
                    }
                }
                other if other == "stash" || other.starts_with("stash ") => {
                    let rest = other.strip_prefix("stash").unwrap_or("").trim();
                    if rest.is_empty() {
                        if stash.is_empty() {
                            println!("stash is empty");
                        } else {
                            for (i, text) in stash.iter().enumerate() {
                                println!("{}. {text}", i + 1);
                            }
                        }
                    } else if rest == "pop all" {
                        println!("{}", stash.join("\n\n"));
                        stash.clear();
                    } else if let Some(n) = rest.strip_prefix("pop").map(str::trim) {
                        let index = n.parse::<usize>().unwrap_or(1);
                        if index >= 1 && index <= stash.len() {
                            println!("{}", stash.remove(index - 1));
                        } else {
                            println!("stash: no entry {index}");
                        }
                    } else if let Some(n) = rest.strip_prefix("delete").map(str::trim) {
                        let index = n.parse::<usize>().unwrap_or(0);
                        if index >= 1 && index <= stash.len() {
                            stash.remove(index - 1);
                            println!("stash: deleted {index}");
                        } else {
                            println!("stash: no entry {index}");
                        }
                    } else {
                        stash.push(rest.to_string());
                        println!("stash: added ({})", stash.len());
                    }
                }
                "provider" => {
                    let rows: Vec<(String, String)> = [
                        providers::Provider::DeepSeek,
                        providers::Provider::OpenAi,
                        providers::Provider::Anthropic,
                        providers::Provider::Gemini,
                        providers::Provider::Ollama,
                        providers::Provider::Custom,
                    ]
                    .into_iter()
                    .map(|p| {
                        let profile = p.profile();
                        let about = format!("{}  {}", profile.base_url, profile.model);
                        (format!("{p:?}").to_lowercase(), about)
                    })
                    .chain(session.cfg.named.iter().map(|n| {
                        (n.name.clone(), format!("{}  {}  {}", n.base, n.model, n.dialect))
                    }))
                    .collect();
                    print!("{}", value_rows(&rows, &session.cfg.active_name));
                }
                other if other.starts_with("provider ") => {
                    println!("provider: add, edit, delete, and use run in the chat");
                }
                other => {
                    let line = help::unknown(other);
                    println!("{}", cli.palette.paint(tui::Role::Error, &line));
                }
            }
            continue;
        }
        if let Some(command) = text.strip_prefix('!') {
            // Same runner as the chat: local, never sent to the model.
            // Typed means approved, but the deny list still holds.
            let mode = session.cfg.live_permissions();
            if let perm::Decision::Deny(why) =
                perm::classify_command(command, mode, session.cfg.allow_sudo)
            {
                println!("{}", cli.palette.paint(tui::Role::Error, &format!("$ {command}: {why}")));
                continue;
            }
            let cancel = std::sync::atomic::AtomicBool::new(false);
            let done = session.tools.bash(command, &cancel);
            let role = if done.ok { tui::Role::Tool } else { tui::Role::Error };
            println!("{}", cli.palette.paint(role, &format!("$ {command}")));
            // Your own command's output keeps its colours when colour is on.
            let shown = done.full.as_deref().map(secrets::redact).unwrap_or_else(|| done.text.clone());
            let shown = agent::first_lines(&shown, agent::SHOW_LINES);
            if cli.palette.on {
                for line in render::keep_sgr(&shown).lines() {
                    println!("  {line}\u{1b}[0m");
                }
            } else {
                for line in render::clean(&shown).lines() {
                    println!("  {}", cli.palette.paint(role, line));
                }
            }
            println!();
            continue;
        }
        let start = std::time::Instant::now();
        if let Err(e) = session.turn(text, cli) {
            eprintln!("imp: {e}");
        }
        finish_line(session, start, cli.verbosity);
        println!();
    }
}

/// The task's time, steps, tokens, and cost on stderr, as the chat shows it.
fn finish_line(session: &Session, start: std::time::Instant, verbosity: config::Verbosity) {
    if verbosity == config::Verbosity::Low {
        return;
    }
    let t = session.task;
    let cost = (session.cfg.provider == providers::Provider::DeepSeek)
        .then(|| pricing::cost(&session.cfg.model, pricing::now(), t.hit, t.miss, t.out))
        .flatten();
    eprintln!("\n{}", tui::task_line(start.elapsed(), t.steps, t.hit, t.miss, t.out, cost));
}

/// Values as aligned rows, with `current` beside the active one.
fn value_rows(rows: &[(String, String)], current: &str) -> String {
    let width = rows.iter().map(|(n, _)| n.chars().count()).max().unwrap_or(0);
    let mut out = String::new();
    for (name, about) in rows {
        let right = match (name == current, about.is_empty()) {
            (true, true) => "current".to_string(),
            (true, false) => format!("current · {about}"),
            (false, _) => about.clone(),
        };
        let line = format!("  {name:<width$}  {right}");
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

struct Cli {
    verbosity: config::Verbosity,
    palette: tui::Palette,
}

impl Hooks for Cli {
    fn ui(&mut self, event: Ui) {
        print_ui(event, self.verbosity, &self.palette)
    }

    fn confirm(&mut self, what: &str) -> bool {
        answer(what, false) == agent::Answer::Yes
    }

    fn confirm_always(&mut self, what: &str) -> agent::Answer {
        answer(what, true)
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

fn print_ui(event: Ui, verbosity: config::Verbosity, palette: &tui::Palette) {
    let mut out = std::io::stdout();
    print_ui_to(&mut out, event, verbosity, palette).ok();
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
    verbosity: config::Verbosity,
    palette: &tui::Palette,
) -> std::io::Result<()> {
    let verbose = verbosity == config::Verbosity::High;
    match event {
        Ui::Content(t) => {
            // Model text never drives the terminal, piped or not.
            write!(out, "{}", render::clean(&t))?;
            out.flush().ok();
        }
        Ui::Reasoning(t) => {
            let _ = t;
        }
        // Totalled by the session; printed once the task ends.
        Ui::Usage { .. } => {}
        Ui::ToolStart { name, summary, args } => {
            let (summary, args) = (render::clean(&summary), render::clean(&args));
            let line = format!("\u{25b8} {summary}");
            writeln!(out, "\n{}", palette.paint(tui::Role::Tool, &line))?;
            if verbose && !args.is_empty() {
                for line in args.lines() {
                    writeln!(out, "  {}", palette.paint(tui::Role::Thinking, line))?;
                }
            }
            let _ = name;
        }
        Ui::ToolEnd { ok, preview, output, ms, .. } => {
            let role = if ok { tui::Role::Tool } else { tui::Role::Error };
            let time = tui::duration(ms);
            let preview = preview_lines(&render::clean(&preview));
            if ok {
                // Low names the step only; medium and high say what came back.
                let line = match verbosity {
                    config::Verbosity::Low => format!("✓ {time}"),
                    _ => format!("✓ {} · {time}", preview.trim_end()),
                };
                writeln!(out, "  {}", palette.paint(role, &line))?;
            } else {
                // The failure reason starts on its own indented line.
                writeln!(out, "  {}", palette.paint(role, &format!("✗ {time}")))?;
                for line in preview.lines() {
                    writeln!(out, "    {}", palette.paint(role, line.trim_end()))?;
                }
            }
            if verbose && !output.is_empty() {
                for line in preview_lines(&render::clean(&output)).lines() {
                    writeln!(out, "  {}", palette.paint(tui::Role::Thinking, line))?;
                }
            }
        }
        Ui::Show(t) => {
            for line in t.lines() {
                if palette.on {
                    writeln!(out, "  {}\u{1b}[0m", render::keep_sgr(line))?;
                } else {
                    writeln!(out, "  {}", render::clean(line))?;
                }
            }
        }
        Ui::Info(t) => {
            let t = render::clean(&t);
            let line = format!("\u{25e6} {t}");
            writeln!(out, "\n{}", palette.paint(tui::Role::Info, &line))?;
        }
        Ui::Error(t) => {
            let t = render::clean(&t);
            let line = format!("! {t}");
            writeln!(out, "\n{}", palette.paint(tui::Role::Error, &line))?;
        }
    }
    Ok(())
}

fn answer(prompt: &str, always: bool) -> agent::Answer {
    let keys = if always { "[y] yes  [n] no  [s] session  [a] always" } else { "[y] yes  [n] no" };
    print!("{prompt}? {keys} ");
    std::io::stdout().flush().ok();
    let Ok(Some(line)) = read_entry(&mut std::io::stdin().lock()) else {
        return agent::Answer::No;
    };
    parse_answer(&line, always)
}

fn parse_answer(line: &str, always: bool) -> agent::Answer {
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => agent::Answer::Yes,
        "s" | "session" if always => agent::Answer::Session,
        "a" | "always" if always => agent::Answer::Always,
        _ => agent::Answer::No,
    }
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

/// The id after `-r` at `i`, or `None` when nothing or a flag follows.
fn resume_id(args: &[String], i: usize) -> Option<&String> {
    args.get(i + 1).filter(|a| !a.is_empty() && !a.starts_with('-'))
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
    fn line_answers_take_always_only_when_offered() {
        use agent::Answer;
        assert_eq!(parse_answer("y\n", true), Answer::Yes);
        assert_eq!(parse_answer("a", true), Answer::Always);
        assert_eq!(parse_answer("always", true), Answer::Always);
        assert_eq!(parse_answer("s", true), Answer::Session);
        assert_eq!(parse_answer("s", false), Answer::No);
        assert_eq!(parse_answer("a", false), Answer::No);
        assert_eq!(parse_answer("", true), Answer::No);
    }

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
    fn resume_takes_an_id_only_when_one_follows() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(resume_id(&args(&["-r"]), 0), None);
        assert_eq!(resume_id(&args(&["-r", "--model", "x"]), 0), None);
        assert_eq!(resume_id(&args(&["-r", "512705c23"]), 0).map(|s| s.as_str()), Some("512705c23"));
        // Strict: a word is an id, so a typo fails instead of resuming the newest.
        assert_eq!(resume_id(&args(&["-r", "fix", "it"]), 0).map(|s| s.as_str()), Some("fix"));
    }

    #[test]
    fn a_missing_key_stops_line_mode_but_not_the_chat() {
        let err = key_gate(false, Err("DEEPSEEK_API_KEY".into())).unwrap_err();
        assert!(err.contains("DEEPSEEK_API_KEY"), "{err}");
        assert!(err.contains("terminal"), "no fix named: {err}");
        assert_eq!(
            key_gate(true, Err("DEEPSEEK_API_KEY".into())).unwrap(),
            Some("DEEPSEEK_API_KEY".to_string())
        );
        assert_eq!(key_gate(true, Ok(())).unwrap(), None);
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
            config::Verbosity::High,
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
            config::Verbosity::High,
            &palette,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("{\"command\":\"ls\"}"), "{text}");
        assert!(text.contains("✓ total 8 · 4ms") && text.contains("  foo"), "{text}");

        let mut low = Vec::new();
        print_ui_to(
            &mut low,
            Ui::ToolStart {
                name: "bash".into(),
                summary: "run `ls`".into(),
                args: "{\"command\":\"ls\"}".into(),
            },
            config::Verbosity::Low,
            &palette,
        )
        .unwrap();
        let low = String::from_utf8(low).unwrap();
        assert!(!low.contains("command"), "{low}");
    }

    #[test]
    fn line_mode_failure_preview_sits_on_its_own_line() {
        let palette = tui::Palette::detect(&config::Config::default());
        let mut out = Vec::new();
        print_ui_to(
            &mut out,
            Ui::ToolEnd {
                ok: false,
                preview: "exit 1 hi ".into(),
                output: String::new(),
                detail: String::new(),
                ms: 1400,
            },
            config::Verbosity::Low,
            &palette,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        let plain: String = text.split('\x1b').map(|p| p.split_once('m').map_or(p, |(_, r)| r)).collect();
        assert!(plain.contains("  ✗ 1.4s\n    exit 1 hi\n"), "{plain:?}");
        assert!(!plain.lines().any(|l| l.ends_with(' ')), "{plain:?}");
    }

    #[test]
    fn value_rows_align_and_mark_the_current_one() {
        let rows: Vec<(String, String)> = ["default", "mono", "gruvbox-dark"]
            .iter()
            .map(|n| (n.to_string(), String::new()))
            .collect();
        let text = value_rows(&rows, "mono");
        assert_eq!(text, "  default\n  mono          current\n  gruvbox-dark\n");
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
