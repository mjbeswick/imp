mod agent;
mod api;
mod config;
mod http;
mod init;
mod jobs;
mod perm;
mod store;
mod tools;
mod tui;

use agent::{Hooks, Session, Ui};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
mote — a small terminal agent for DeepSeek

USAGE:
    mote [OPTIONS] [PROMPT]
    mote init

OPTIONS:
    -c, --continue          Continue the most recent session
    -r, --resume <ID>       Resume a session by id or file path
    -l, --list              List sessions and exit
        --rename <ID> <T>   Rename a session
        --delete <ID>       Delete a session (moves it to trash)
        --agent <NAME>      Agent profile to start with
        --skills-dir <P>    Add a skills directory
        --model <NAME>      Model to use
        --permissions <P>   normal | strict | yolo
        --cli               Line mode, even in a terminal
        --chat              Full-screen chat, even when piped
    -h, --help              Show this help
    -V, --version           Show the version
";

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
        print!("{USAGE}");
        return Ok(());
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("mote {}", env!("CARGO_PKG_VERSION"));
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
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => cfg.model = next(&args, &mut i, "--model")?,
            "--permissions" => {
                cfg.permissions = config::Permissions::parse(&next(&args, &mut i, "--permissions")?)?
            }
            "--agent" => agent_name = Some(next(&args, &mut i, "--agent")?),
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

    if cfg.api_key.is_empty() {
        return Err("DEEPSEEK_API_KEY is not set".into());
    }

    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    let out_dir = config::dirs_state().join("mote").join("out");
    let system = system_prompt(&root);
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
    let session = Session::new(cfg, root, out_dir, system, agent, roster);

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
                println!("/help /quit  —  or just talk to it");
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
    print!("allow {prompt}? [y/N] ");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn system_prompt(root: &std::path::Path) -> String {
    let mut prompt = String::from(
        "You are mote, a terminal agent. You run commands and edit files on the user's machine.\n\
         Be direct and brief. Prefer doing over explaining. Read a file before editing it.\n\
         Tools: bash, read, edit, write, ask. Use `read` rather than cat, `edit` rather than sed,\n\
         and `write` rather than shell redirection, so caps and checks apply.\n\
         Use ask only when a real decision blocks you.\n",
    );
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let path = root.join(name);
        if let Ok(text) = std::fs::read_to_string(&path) {
            let text: String = text.chars().take(8192).collect();
            prompt.push_str("\n# Project instructions\n");
            prompt.push_str(&text);
            break;
        }
    }
    prompt
}

fn next(args: &[String], i: &mut usize, flag: &str) -> Result<String, String> {
    *i += 1;
    args.get(*i)
        .filter(|v| !v.is_empty())
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}
