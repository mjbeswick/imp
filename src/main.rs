mod agent;
mod api;
mod config;
mod http;
mod perm;
mod tools;

use agent::{Session, Ui};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
mote — a small terminal agent for DeepSeek

USAGE:
    mote [OPTIONS] [PROMPT]

OPTIONS:
    -c, --continue          Continue the most recent session
    -r, --resume <ID>       Resume a session by id or file path
    -l, --list              List sessions and exit
        --rename <ID> <T>   Rename a session
        --delete <ID>       Delete a session (moves it to trash)
        --skills-dir <P>    Add a skills directory
        --model <NAME>      Model to use
        --permissions <P>   normal | strict | yolo
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

    let mut cfg = config::Config::load(&config::Config::config_path())?;
    let mut prompt: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--model" => cfg.model = next(&args, &mut i, "--model")?,
            "--permissions" => {
                cfg.permissions = config::Permissions::parse(&next(&args, &mut i, "--permissions")?)?
            }
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
    let mut session = Session::new(cfg, root, out_dir, system);

    match prompt {
        Some(text) => session.turn(&text, &mut print_ui, &mut confirm),
        None => repl(&mut session),
    }
}

fn repl(session: &mut Session) -> Result<(), String> {
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
        if let Err(e) = session.turn(text, &mut print_ui, &mut confirm) {
            eprintln!("mote: {e}");
        }
        println!();
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
         Tools: bash, read, edit, write, ask. Use ask only when a real decision blocks you.\n",
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
