mod agent;
mod api;
mod config;
mod help;
mod http;
mod init;
mod jobs;
mod perm;
mod prompt;
mod store;
mod tools;
mod tui;

use agent::{Hooks, Session, Ui};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;


fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("mote: {msg}");
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
        println!("mote {}", env!("CARGO_PKG_VERSION"));
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
    let mut prompt: Option<String> = None;
    let mut mode = "auto";
    let mut agent_name: Option<String> = None;
    let mut resume: Option<String> = None;
    let mut continue_last = false;
    let mut list = false;
    let mut rename: Option<(String, String)> = None;
    let mut delete: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => cfg.model = next(&args, &mut i, "--model")?,
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
            "--chat" => mode = "chat",
            "--skills-dir" => {
                cfg.skills_dir = Some(PathBuf::from(next(&args, &mut i, "--skills-dir")?))
            }
            other if other.starts_with('-') && other.len() > 1 => {
                return Err(format!("unknown option {other}"))
            }
            other => prompt = Some(other.to_string()),
        }
        i += 1;
    }

    let sessions_dir = config::dirs_state().join("mote").join("sessions");

    if list {
        for m in store::Log::list(&sessions_dir) {
            println!(
                "{:<12} {:<40} {:>5} msgs",
                m.id,
                if m.title.is_empty() { "(untitled)" } else { &m.title },
                m.messages
            );
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

    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let out_dir = config::dirs_state().join("mote").join("out");
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
    let mut session = Session::new(cfg, root.clone(), out_dir, system, agent, roster);

    let existing = match &resume {
        Some(id) => store::Log::open(&sessions_dir, id),
        None if continue_last => store::Log::latest(&sessions_dir),
        None => None,
    };
    match existing {
        Some(log) => {
            let restored = log.load();
            let count = restored.len();
            session.resume(restored);
            session.attach_log(log.clone());
            eprintln!("mote: resumed {} ({} messages)", log.id, count);
        }
        None => {
            session.attach_log(store::Log::create(&sessions_dir, &root, &session.cfg.model));
        }
    }

    match prompt {
        Some(text) => {
            let mut session = session;
            session.turn(&text, &mut Cli)
        }
        None => {
            let interactive = unsafe { isatty(0) } != 0 && unsafe { isatty(1) } != 0;
            let chat = match mode {
                "chat" => true,
                "cli" => false,
                _ => interactive,
            };
            if chat {
                tui::run(session, tui_cfg)
            } else {
                let mut session = session;
                repl(&mut session, &mut Cli)
            }
        }
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

fn repl(session: &mut Session, cli: &mut Cli) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    loop {
        print!("> ");
        out.flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            println!();
            return Ok(());
        }
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        match text {
            "/quit" | "/exit" => return Ok(()),
            "/help" => {
                println!("{}", help::commands());
                continue;
            }
            _ => {}
        }
        if let Err(e) = session.turn(text, cli) {
            eprintln!("mote: {e}");
        }
        println!();
    }
}

struct Cli;

impl Hooks for Cli {
    fn ui(&mut self, event: Ui) {
        print_ui(event)
    }

    fn confirm(&mut self, what: &str) -> bool {
        confirm(what)
    }

    fn ask(&mut self, question: &str, options: &[String]) -> Option<String> {
        println!("\n{question}");
        for (i, o) in options.iter().enumerate() {
            println!("  {}. {o}", i + 1);
        }
        print!("answer{}: ", if options.is_empty() { "" } else { " (number or text)" });
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).ok()?;
        let line = line.trim().to_string();
        if let Ok(n) = line.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return Some(options[n - 1].clone());
            }
        }
        Some(line)
    }
}

fn print_ui(event: Ui) {
    let mut out = std::io::stdout();
    match event {
        Ui::Content(t) => {
            print!("{t}");
            out.flush().ok();
        }
        Ui::Reasoning(t) => {
            // Reasoning is collapsed unless asked for; a single marker is enough.
            let _ = t;
        }
        Ui::ToolStart { name, args } => {
            println!("\n\u{25b8} {name}  {args}");
        }
        Ui::ToolEnd { ok, preview, .. } => {
            let mark = if ok { "" } else { "!" };
            println!("  {mark}{preview}");
        }
        Ui::Info(t) => println!("\n\u{25e6} {t}"),
        Ui::Error(t) => println!("\n! {t}"),
    }
}

fn confirm(prompt: &str) -> bool {
    print!("{prompt}? [y/N] ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn system_prompt(root: &std::path::Path, cfg: &config::Config) -> String {
    prompt::compose(root, cfg)
}

fn next(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}
