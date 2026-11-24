//! The turn loop: request, stream, run tools, repeat.

use crate::api::{self, Accum, Image, Message, RequestOptions, SseParser, ToolCall};
use crate::config::{Config, Permissions};
use crate::perm::{self, Decision};
use crate::jobs::Jobs;
use crate::transport::{Live, Transport};
use crate::store::{Agent, Log, Memory, Skill};
use crate::tools::Tools;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

/// An answer to a confirmation that may offer `session` and `always`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    No,
    Yes,
    /// This shape, for the rest of the session.
    Session,
    /// This shape, saved to the config for every session.
    Always,
}

pub trait Hooks {
    fn ui(&mut self, event: Ui);
    fn confirm(&mut self, what: &str) -> bool;

    /// A confirmation that also offers `always`. Front-ends that cannot show it
    /// answer as `confirm` does.
    fn confirm_always(&mut self, what: &str) -> Answer {
        if self.confirm(what) { Answer::Yes } else { Answer::No }
    }

    /// A command shape the user allowed, for this session or for good.
    fn allowed(&mut self, _shape: &str, _always: bool) {}

    /// The answer given for one command shape, for the permissions review.
    fn answered(&mut self, _shape: &str, _answer: Answer) {}
    fn ask(&mut self, question: &str, options: &[String]) -> Option<String>;

    /// A question whose options carry descriptions, and may allow several
    /// picks. Front-ends that cannot show them ask with the labels alone.
    fn ask_choices(&mut self, question: &str, options: &[(String, String)], multiple: bool) -> Option<String> {
        let _ = multiple;
        let labels: Vec<String> = options.iter().map(|(l, _)| l.clone()).collect();
        self.ask(question, &labels)
    }

    fn ask_many(&mut self, questions: &[AskQuestion]) -> Option<Vec<Vec<String>>> {
        questions.iter().map(|q| {
            let title = if q.header.is_empty() { q.question.clone() } else { format!("{}: {}", q.header, q.question) };
            self.ask_choices(&title, &q.options, q.multiple).map(|answer| vec![answer])
        }).collect()
    }

    /// Time the last confirmation kept the user, which is not the tool's cost.
    fn last_wait(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }

    /// Run a command attached to the terminal, for prompts that need a tty.
    /// `None` means the front-end cannot hand the terminal over.
    fn run_attached(&mut self, _command: &str) -> Option<i32> {
        None
    }

    /// Whether a command can take over the screen: the chat can; line mode,
    /// which prints rows, cannot.
    fn can_foreground(&self) -> bool {
        false
    }
}

pub enum Ui {
    Content(String),
    Reasoning(String),
    ToolStart { name: String, summary: String, args: String },
    ToolEnd {
        ok: bool,
        preview: String,
        output: String,
        detail: String,
        ms: u128,
    },
    Info(String),
    /// A finished background job: an info row in the chat, a toast in a session out of view.
    Notice(String),
    Error(String),
    /// Waiting `secs` before attempt `attempt + 1` of `of` retries.
    Retry { secs: u64, attempt: u32, of: u32 },
    /// Output shown to the user with its colours; the model gets text.
    Show(String),
    /// One request's usage, sent after every request.
    Usage { hit: u64, miss: u64, output: u64, model: Option<String>, context: u64 },
}

/// A `/goal`: its text and how many rounds it has run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goal {
    pub text: String,
    pub round: u32,
    pub rounds: u32,
    /// Kept but not run: `/goal pause` sets it, `/goal resume` clears it.
    pub paused: bool,
}

/// A file before imp first changed it this session.
#[derive(Debug, Clone, PartialEq)]
pub enum Before {
    /// It did not exist.
    Missing,
    Text(String),
    /// Over 1 MB or not UTF-8: listed, not diffed.
    Untracked,
}

/// An `ask` call's options as label and description. A bare string is a
/// label, so the older form still works.
fn ask_options(args: &Value) -> Vec<(String, String)> {
    let Some(list) = args.get("options").and_then(Value::as_array) else { return Vec::new() };
    list.iter()
        .filter_map(|v| match v {
            Value::String(label) => Some((label.clone(), String::new())),
            Value::Object(o) => {
                let label = o.get("label")?.as_str()?.to_string();
                let about = o.get("description").and_then(Value::as_str).unwrap_or("").to_string();
                Some((label, about))
            }
            _ => None,
        })
        .collect()
}

pub struct AskQuestion {
    pub header: String,
    pub question: String,
    pub options: Vec<(String, String)>,
    pub multiple: bool,
}

fn ask_question(args: &Value) -> Result<AskQuestion, String> {
    let question = args.get("question").and_then(Value::as_str).filter(|s| !s.trim().is_empty())
        .ok_or("refused: ask needs a question")?;
    let options = ask_options(args);
    if let Some(raw) = args.get("options") {
        let list = raw.as_array().ok_or("refused: ask options must be a list")?;
        if list.len() != options.len() || options.iter().any(|(label, _)| label.trim().is_empty()) {
            return Err("refused: ask needs labelled options".into());
        }
        if list.len() == 1 || list.len() > 6 {
            return Err("refused: ask accepts two to six options, or none".into());
        }
    }
    Ok(AskQuestion {
        header: args.get("header").and_then(Value::as_str).unwrap_or("").to_string(),
        question: question.to_string(),
        options,
        multiple: args.get("multiple").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// One file's change for `/diff`.
#[derive(Debug, Clone, PartialEq)]
pub struct FileDiff {
    /// The path from the session directory, or in full outside it.
    pub name: String,
    pub ext: String,
    pub adds: usize,
    pub dels: usize,
    /// ' ', '-', or '+' rows, and `⋯` between hunks.
    pub rows: Vec<String>,
    /// Over 1 MB or not text: listed only.
    pub untracked: bool,
}

/// `git diff` output as one `FileDiff` per file: each `+++ b/<path>` names
/// the file, hunks are parted by `⋯`, and a binary file is listed only.
pub fn parse_git_diff(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut in_hunk = false;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let name = rest.rsplit_once(" b/").map_or(rest, |(_, b)| b).to_string();
            let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
            files.push(FileDiff { name, ext, adds: 0, dels: 0, rows: Vec::new(), untracked: false });
            in_hunk = false;
            continue;
        }
        let Some(f) = files.last_mut() else { continue };
        if let Some(path) = line.strip_prefix("+++ b/") {
            f.name = path.to_string();
            f.ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        } else if line.starts_with("Binary files ") {
            f.untracked = true;
        } else if line.starts_with("@@") {
            if in_hunk {
                f.rows.push("⋯".into());
            }
            in_hunk = true;
        } else if in_hunk && (line.starts_with('+') || line.starts_with('-') || line.starts_with(' ')) {
            match line.as_bytes()[0] {
                b'+' => f.adds += 1,
                b'-' => f.dels += 1,
                _ => {}
            }
            f.rows.push(line.to_string());
        }
    }
    files
}

/// What one task has used so far: steps and tokens over all its requests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TaskTotals {
    pub steps: usize,
    pub hit: u64,
    pub miss: u64,
    pub out: u64,
    /// The user declined a confirmation during the task.
    pub declined: bool,
}

/// Lines of output `show` puts on screen.
pub const SHOW_LINES: usize = 200;

/// The first `n` lines of text.
pub fn first_lines(text: &str, n: usize) -> String {
    text.split_inclusive('\n').take(n).collect()
}

/// Shown output as the model gets it: plain text, or for a picture drawn in
/// block or braille characters, only its size.
fn for_model(shown: &str) -> String {
    let plain = crate::render::clean(shown);
    let ink: Vec<char> = plain.chars().filter(|c| !c.is_whitespace()).collect();
    let drawn = ink
        .iter()
        .filter(|&&c| matches!(c, '\u{2580}'..='\u{259f}' | '\u{2800}'..='\u{28ff}' | '\u{1fb00}'..='\u{1fbff}'))
        .count();
    if !ink.is_empty() && drawn * 3 > ink.len() {
        let width = plain.lines().map(|l| l.trim_end().chars().count()).max().unwrap_or(0);
        let height = plain.trim_end().lines().count();
        format!("[shown to the user: a {width}×{height} picture]")
    } else {
        plain
    }
}

/// Check-ins one session may have waiting.
const MAX_CHECKINS: usize = 4;
/// Retries after a failed request, each waiting twice the last.
const RETRIES: u32 = 5;

pub fn schema() -> Vec<Value> {
    vec![
        json!({"type":"function","function":{"name":"bash","description":"Run a shell command. Output is capped; larger output is written to a file whose path is returned.","parameters":{"type":"object","properties":{"command":{"type":"string","description":"Command line, run with sh -c."},"background":{"type":"boolean","description":"Start it as a job and return an id immediately."},"foreground":{"type":"boolean","description":"Hand the user's terminal to an interactive app, game, or server; you get only its exit code. The user is always asked first."},"delay_s":{"type":"integer","description":"Start it as a job after this many seconds, up to 86400."},"notify":{"type":"boolean","description":"With background or delay_s: wake you when the job ends."},"description":{"type":"string","description":"A few words on what it does, shown to the user instead of a long command."},"show":{"type":"boolean","description":"Show the output to the user with its colours, for pictures, charts, and diagrams; you get the text, or only the size of a picture."}},"required":["command"]}}}),
        json!({"type":"function","function":{"name":"job","description":"List background jobs with progress, wait for one, kill one, or check back later: check wakes you after after_s seconds with the job's state, so you can report progress while the user is away.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["list","wait","kill","check"]},"id":{"type":"integer"},"timeout_s":{"type":"integer","description":"For wait; up to 55 seconds."},"after_s":{"type":"integer","description":"For check; 30 to 86400 seconds."}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"read","description":"Read files with line numbers, or list a directory. Add several paths in one call.","parameters":{"type":"object","properties":{"paths":{"type":"array","items":{"type":"string"}},"offset":{"type":"integer"},"limit":{"type":"integer"},"depth":{"type":"integer","description":"Directory levels, 1 to 3. Default 1."}},"required":["paths"]}}}),
        json!({"type":"function","function":{"name":"edit","description":"Replace exact text in a file. Every old string must match exactly once; all edits apply or none do.","parameters":{"type":"object","properties":{"path":{"type":"string"},"edits":{"type":"array","items":{"type":"object","properties":{"old":{"type":"string"},"new":{"type":"string"}},"required":["old","new"]}}},"required":["path","edits"]}}}),
        json!({"type":"function","function":{"name":"write","description":"Create or replace a file.","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}}),
        json!({"type":"function","function":{"name":"fetch","description":"One HTTP request from this machine, including the local network. GET by default; HTML comes back as markdown, feeds as a list, JSON and text as they are. Configured credentials are added for their host by imp; never put keys in headers.","parameters":{"type":"object","properties":{"url":{"type":"string"},"method":{"type":"string","enum":["GET","POST","PUT","PATCH","DELETE"]},"headers":{"type":"object","description":"Extra request headers, name to value."},"body":{"description":"Text, or JSON sent as application/json."}},"required":["url"]}}}),
        json!({"type":"function","function":{"name":"search","description":"Search the web; results are summaries with URLs. Read a page with fetch.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["search"]},"query":{"type":"string"},"count":{"type":"integer"}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"memory","description":"Search stored notes, or save a new one.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["search","save"]},"query":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"task","description":"Hand a self-contained task to a named agent profile. The profile runs in its own context with its own tools, and its final answer comes back as this result. Use it for independent research or review that would otherwise fill your context. The user can open the child's transcript from its row.","parameters":{"type":"object","properties":{"agent":{"type":"string","description":"Profile name, such as reviewer."},"prompt":{"type":"string","description":"Everything the profile needs: the task, the paths, and what to return."}},"required":["agent","prompt"]}}}),
        json!({"type":"function","function":{"name":"skill_load","description":"Load the full instructions of a skill by name.","parameters":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}}),
        json!({"type":"function","function":{"name":"goal","description":"Propose a goal for the user to confirm. It runs in rounds only after the user says yes; if the user says no, it is dropped.","parameters":{"type":"object","properties":{"operation":{"type":"string","enum":["set"]},"text":{"type":"string"}},"required":["operation","text"]}}}),
        json!({"type":"function","function":{"name":"ask","description":"Ask only when blocked on a decision. Prefer an ordered questions array, each with a short header and two to six described choices or free text; multiple selections are allowed. Recommend an option first. Batch results contain arrays of exact chosen labels; legacy single-question calls return a string.","parameters":{"type":"object","properties":{"questions":{"type":"array","minItems":1,"items":{"type":"object","properties":{"header":{"type":"string","description":"Short question label."},"question":{"type":"string"},"options":{"type":"array","minItems":2,"maxItems":6,"items":{"type":"object","properties":{"label":{"type":"string"},"description":{"type":"string"}},"required":["label"]}},"multiple":{"type":"boolean"}},"required":["question"]}},"question":{"type":"string","description":"Legacy single-question form."},"options":{"type":"array","items":{"type":"object","properties":{"label":{"type":"string"},"description":{"type":"string"}},"required":["label"]}},"multiple":{"type":"boolean"}}}}}),
    ]
}

/// `command` for `sh -c`, run from `dir` rather than imp's launch directory.
pub fn in_dir(dir: &std::path::Path, command: &str) -> String {
    let dir = dir.display().to_string().replace('\'', "'\\''");
    format!("cd '{dir}' && {command}")
}

fn diff_detail(name: &str, args: &Value) -> String {
    let mut lines: Vec<String> = Vec::new();
    match name {
        "edit" => {
            for edit in args.get("edits").and_then(Value::as_array).into_iter().flatten() {
                let old = edit.get("old").and_then(Value::as_str).unwrap_or("");
                let new = edit.get("new").and_then(Value::as_str).unwrap_or("");
                let (old, new): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
                // Lines both sides share are context, not change; keep one each side.
                let head = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
                let tail = old[head..].iter().rev().zip(new[head..].iter().rev()).take_while(|(a, b)| a == b).count();
                if head > 0 {
                    lines.push(format!(" {}", old[head - 1]));
                }
                for line in &old[head..old.len() - tail] {
                    lines.push(format!("-{line}"));
                }
                for line in &new[head..new.len() - tail] {
                    lines.push(format!("+{line}"));
                }
                if tail > 0 {
                    lines.push(format!(" {}", old[old.len() - tail]));
                }
                lines.push(String::new());
            }
        }
        "write" => {
            for line in args
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .lines()
            {
                lines.push(format!("+{line}"));
            }
        }
        _ => return String::new(),
    }
    let mut out = lines.into_iter().take(200).collect::<Vec<_>>().join("\n");
    if out.len() > 8192 {
        out.truncate(8192);
        out.push_str("\n…");
    }
    out
}

pub fn describe_agent(agent: &Agent, cfg: &Config) -> String {
    let tools = agent
        .tools
        .as_ref()
        .map(|t| t.join(", "))
        .unwrap_or_else(|| "all".into());
    let model = agent.model.as_deref().unwrap_or(&cfg.model);
    let thinking = agent.thinking.unwrap_or(cfg.thinking);
    format!(
        "agent: {} · model {} · thinking {} · tools {}",
        agent.name,
        model,
        if thinking { "on" } else { "off" },
        tools
    )
}

pub struct Session {
    pub cfg: Config,
    /// The first backoff wait; doubled on each retry. Short in tests.
    pub retry_base: std::time::Duration,
    pub tools: Tools,
    pub history: Vec<Message>,
    pub cancel: std::sync::Arc<AtomicBool>,
    pub cols: std::sync::Arc<std::sync::atomic::AtomicU16>,
    last_cols: Option<(u16, u16)>,
    /// The day the model was last told the date, as days since the epoch.
    last_day: Option<u64>,
    pub agent: Agent,
    pub roster: Vec<Agent>,
    pub jobs: Jobs,
    pub job_dir: std::path::PathBuf,
    /// Command shapes answered `always` in this session.
    always: std::collections::HashSet<String>,
    /// Hosts the user let credentialed reads reach in this session.
    auth_hosts: std::collections::HashSet<String>,
    /// True when a worker can wake this session; line mode cannot.
    pub wakes: bool,
    /// Pending check-ins: when they are due, the job if one, and when asked.
    checkins: Vec<(std::time::Instant, Option<u64>, std::time::Instant)>,
    /// Jobs whose end wakes the session.
    notify: std::collections::HashSet<u64>,
    pub log: Option<Log>,
    pub last_usage: (u64, u64, u64),
    /// The current or last task's totals.
    pub task: TaskTotals,
    /// A goal that carries the work across turns.
    pub goal: Option<Goal>,
    /// The user declined a confirmation since the goal last checked.
    declined: bool,
    pub last_model: Option<String>,
    pub attachments: Vec<Image>,
    /// Each file `edit` or `write` touched this session, as it was before.
    pub originals: Vec<(std::path::PathBuf, Before)>,
    /// Messages the user steered into the running turn, taken between steps.
    pub steer: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Set by the chat while a queued message is edited: the turn waits
    /// before its next step.
    pub hold: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub transport: Box<dyn Transport>,
    pub memory: Memory,
    pub skills: Vec<Skill>,
    /// Set while a subagent runs: the `task` call id that started it.
    pub child_call: Option<String>,
    /// Finished subagent transcripts, by the `task` call id that ran them.
    pub children: std::collections::HashMap<String, Vec<Message>>,
    /// The id of the tool call being dispatched, for `task`.
    current_call: String,
}

/// Wraps the parent's interface for a subagent: its prose stays out of the
/// transcript, its tool rows and prompts carry its name.
pub struct ChildHooks<'a> {
    pub parent: &'a mut dyn Hooks,
    pub name: String,
}

impl Hooks for ChildHooks<'_> {
    fn ui(&mut self, event: Ui) {
        match event {
            Ui::Content(_) | Ui::Reasoning(_) => {}
            Ui::ToolStart { name, summary, args } => self.parent.ui(Ui::ToolStart {
                name,
                summary: format!("{}: {summary}", self.name),
                args,
            }),
            other => self.parent.ui(other),
        }
    }
    fn confirm(&mut self, what: &str) -> bool {
        self.parent.confirm(&format!("[{}] {what}", self.name))
    }
    fn confirm_always(&mut self, what: &str) -> Answer {
        self.parent.confirm_always(&format!("[{}] {what}", self.name))
    }
    fn allowed(&mut self, shape: &str, always: bool) {
        self.parent.allowed(shape, always)
    }
    fn answered(&mut self, shape: &str, answer: Answer) {
        self.parent.answered(shape, answer)
    }
    fn ask(&mut self, question: &str, options: &[String]) -> Option<String> {
        self.parent.ask(&format!("[{}] {question}", self.name), options)
    }
    fn ask_choices(&mut self, question: &str, options: &[(String, String)], multiple: bool) -> Option<String> {
        self.parent.ask_choices(&format!("[{}] {question}", self.name), options, multiple)
    }
    fn last_wait(&self) -> std::time::Duration {
        self.parent.last_wait()
    }
    fn run_attached(&mut self, command: &str) -> Option<i32> {
        self.parent.run_attached(command)
    }
    fn can_foreground(&self) -> bool {
        self.parent.can_foreground()
    }
}

impl Session {
    pub fn new(
        cfg: Config,
        root: std::path::PathBuf,
        out_dir: std::path::PathBuf,
        system: String,
        agent: Agent,
        roster: Vec<Agent>,
        cols: std::sync::Arc<std::sync::atomic::AtomicU16>,
    ) -> Self {
        let mut history = Vec::new();
        if !system.is_empty() {
            history.push(Message::system(system));
        }
        if !agent.instructions.is_empty() {
            history.push(Message::system(agent.instructions.clone()));
        }
        let job_dir = out_dir.join("jobs");
        let jail = crate::perm::jail_enabled(cfg.live_permissions());
        let memory = Memory::discover(&cfg, &root);
        let skills = crate::store::load_skills(&cfg, &root);
        Self {
            retry_base: std::time::Duration::from_secs(1),
            cfg,
            tools: {
                let mut t = Tools::new(root, out_dir);
                t.jail = jail;
                t
            },
            history,
            cancel: std::sync::Arc::new(AtomicBool::new(false)),
            cols,
            last_cols: None,
            last_day: None,
            agent,
            roster,
            jobs: Jobs::default(),
            job_dir,
            always: std::collections::HashSet::new(),
            auth_hosts: std::collections::HashSet::new(),
            wakes: false,
            checkins: Vec::new(),
            notify: std::collections::HashSet::new(),
            log: None,
            last_usage: (0, 0, 0),
            task: TaskTotals::default(),
            goal: None,
            declined: false,
            last_model: None,
            attachments: Vec::new(),
            steer: Default::default(),
            originals: Vec::new(),
            hold: Default::default(),
            transport: Box::new(Live),
            memory,
            skills,
            child_call: None,
            children: std::collections::HashMap::new(),
            current_call: String::new(),
        }
    }

    pub fn attach_log(&mut self, log: Log) {
        self.log = Some(log);
    }

    /// Copy the current session into a new file and continue there.
    pub fn fork(&mut self) -> Result<(String, String), String> {
        let Some(source) = self.log.clone() else {
            return Err("nothing to fork yet".into());
        };
        let copy = source.fork()?;
        let new_id = copy.id.clone();
        self.attach_log(copy);
        if let Some(log) = &self.log {
            log.event("fork", &source.id);
        }
        Ok((new_id, source.id))
    }

    /// Cut the history back to before the last user message reading `text`,
    /// so a turn can send it again. Its images go back to the attachments.
    /// False when no such message is left, for example after compaction.
    pub fn retry(&mut self, text: &str) -> bool {
        let Some(i) = self.history.iter().rposition(|m| m.role == crate::api::Role::User && m.content == text) else {
            return false;
        };
        let message = self.history.remove(i);
        self.history.truncate(i);
        if let Some(log) = &self.log {
            log.event("retry", text);
        }
        self.attachments.splice(0..0, message.images);
        true
    }

    /// Clear the conversation, keeping the system prompt.
    pub fn reset(&mut self) {
        let system: Vec<Message> = self
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::System)
            .take(1)
            .cloned()
            .collect();
        self.history = system;
        self.always.clear();
        self.auth_hosts.clear();
    }

    /// Load a session by id and continue it.
    pub fn resume_id(&mut self, dir: &std::path::Path, id: &str) -> Result<(), String> {
        let log = Log::open(dir, id).ok_or_else(|| format!("no session {id}"))?;
        let messages = log.load();
        let saved = log.meta().goal;
        self.resume(messages);
        let agent = log.last_event("agent");
        self.attach_log(log);
        // A goal restored on resume waits for `/goal resume`: resuming never runs rounds.
        if let Some(g) = saved {
            self.goal = Some(Goal { text: g.text, round: g.round, rounds: g.rounds, paused: true });
        }
        if let Some(name) = agent {
            let name = crate::store::resolve_agent(&name);
            if let Some(found) = self.roster.iter().find(|a| a.name == name).cloned() {
                self.apply_agent(found);
            }
        }
        Ok(())
    }

    /// Re-read instructions and skills, rebuild the prefix.
    pub fn reload(&mut self) -> Result<(), String> {
        self.skills = crate::store::load_skills(&self.cfg, &self.tools.root);
        self.memory = crate::store::Memory::discover(&self.cfg, &self.tools.root);
        let prompt = crate::prompt::compose(&self.tools.root, &self.cfg);
        let mut rest = std::mem::take(&mut self.history);
        let first = rest
            .iter()
            .position(|m| m.role != crate::api::Role::System)
            .unwrap_or(rest.len());
        rest.drain(..first);
        let mut rebuilt = vec![Message::system(prompt)];
        rebuilt.extend(rest);
        self.history = rebuilt;
        if let Some(log) = &self.log {
            log.event("reloaded", "");
        }
        Ok(())
    }

    /// Replace history with a resumed one, keeping the current system prompt.
    pub fn resume(&mut self, mut loaded: Vec<Message>) {
        let system: Vec<Message> = self
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::System)
            .cloned()
            .collect();
        loaded.retain(|m| m.role != crate::api::Role::System);
        self.history = system;
        self.history.extend(loaded);
    }

    /// What imp's `edit` and `write` changed this session: each file's net
    /// change against its first version. Files back as they were are left out.
    pub fn session_changes(&self) -> Vec<FileDiff> {
        let mut out = Vec::new();
        // Paths are recorded resolved; the root may not be (`/var` on macOS).
        let real = self.tools.root.canonicalize().unwrap_or_else(|_| self.tools.root.clone());
        for (path, before) in &self.originals {
            let short = path.strip_prefix(&real).or_else(|_| path.strip_prefix(&self.tools.root));
            let name = short.unwrap_or(path).display().to_string();
            let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            let now = std::fs::read_to_string(path).unwrap_or_default();
            let old = match before {
                Before::Untracked => {
                    out.push(FileDiff { name, ext, adds: 0, dels: 0, rows: Vec::new(), untracked: true });
                    continue;
                }
                Before::Missing => "",
                Before::Text(t) => t.as_str(),
            };
            let (rows, adds, dels) = crate::linediff::hunks(old, &now);
            if !rows.is_empty() {
                out.push(FileDiff { name, ext, adds, dels, rows, untracked: false });
            }
        }
        out
    }

    /// `git diff --no-color <args>` in the session directory, by file. The
    /// arguments are split on spaces and passed to git, not to a shell.
    // ponytail: no time limit; git diff on a working tree returns at once.
    pub fn git_changes(&self, args: &str) -> Result<Vec<FileDiff>, String> {
        let out = std::process::Command::new("git")
            .args(["diff", "--no-color"])
            .args(args.split_whitespace())
            .current_dir(&self.tools.root)
            .output()
            .map_err(|e| format!("git: {e}"))?;
        if !out.status.success() {
            let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if why.is_empty() { "git diff failed".into() } else { why });
        }
        Ok(parse_git_diff(&String::from_utf8_lossy(&out.stdout)))
    }

    /// Keep a file as it was before imp first changed it this session, for
    /// `/diff`. Over 1 MB or not text, only the path is kept.
    fn remember_original(&mut self, raw: &str) {
        let Ok(path) = self.tools.resolve(raw) else { return };
        if self.originals.iter().any(|(p, _)| *p == path) {
            return;
        }
        let before = match std::fs::metadata(&path) {
            Err(_) => Before::Missing,
            Ok(m) if m.len() > 1024 * 1024 => Before::Untracked,
            Ok(_) => std::fs::read_to_string(&path).map_or(Before::Untracked, Before::Text),
        };
        self.originals.push((path, before));
    }

    fn say(&mut self, message: Message) {
        if let Some(log) = &self.log {
            match &self.child_call {
                Some(call) => log.append_child(call, &message),
                None => log.append(&message),
            }
        }
        self.history.push(message);
    }

    /// Switch profile. Appends a note instead of rewriting the prefix.
    pub fn switch_agent(&mut self, agent: Agent) -> String {
        if let Some(model) = &agent.model {
            self.cfg.model = model.clone();
        }
        if let Some(thinking) = agent.thinking {
            self.cfg.thinking = thinking;
        }
        let note = format!(
            "You are now acting as {}. The instructions below supersede your previous role instructions.\n{}",
            agent.name, agent.instructions
        );
        self.say(Message::system(note));
        let name = agent.name.clone();
        if let Some(log) = &self.log {
            log.event("agent", &name);
        }
        self.apply_agent(agent);
        name
    }

    /// Set the profile without touching history; used by resume and switch.
    pub fn apply_agent(&mut self, agent: Agent) {
        if let Some(model) = &agent.model {
            self.cfg.model = model.clone();
        }
        if let Some(thinking) = agent.thinking {
            self.cfg.thinking = thinking;
        }
        if agent.effort.is_some() {
            self.cfg.effort = agent.effort.clone();
        }
        self.agent = agent;
    }

    pub fn active_tools(&self) -> Vec<Value> {
        schema()
            .into_iter()
            .filter(|t| {
                let Some(name) = t
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(Value::as_str)
                else {
                    return true;
                };
                if !self.agent.allows(name) {
                    return false;
                }
                // Subagents do not start subagents.
                if name == "task" && self.child_call.is_some() {
                    return false;
                }
                if name == "search" && !self.cfg.provider.caps().search {
                    return false;
                }
                include_tool(
                    name,
                    self.memory.dirs.iter().any(|d| d.exists()),
                    self.skills.iter().any(|s| s.path.is_some()),
                )
            })
            .collect()
    }

    fn request(&mut self, hooks: &mut dyn Hooks) -> Result<Accum, String> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let opts = RequestOptions {
                model: self.cfg.model.clone(),
                max_tokens: self.cfg.max_tokens,
                thinking: self.cfg.thinking,
                temperature: Some(0.2),
                effort: self.cfg.effort.clone(),
            };
            let tools = self.active_tools();
            let mut adapter = crate::providers::adapter(
                self.cfg.provider,
                &self.cfg.base_url,
                &self.cfg.api_key,
            );
            let wire = adapter.wire(&opts, &self.history, &tools);
            let headers: Vec<&str> = wire.headers.iter().map(String::as_str).collect();

            let mut parser = SseParser::default();
            let mut acc = Accum::default();
            let status;
            let mut raw_error = String::new();
            let mut raw = Vec::new();

            let result = self.transport.post(
                &wire.url,
                &headers,
                wire.body.to_string().as_bytes(),
                &self.cancel,
                &mut |bytes| {
                    if raw.len() < 4096 {
                        raw.extend_from_slice(bytes);
                    }
                    for ev in parser.push(bytes, adapter.as_mut()) {
                        match &ev {
                            api::Event::Content(t) => hooks.ui(Ui::Content(t.clone())),
                            api::Event::Reasoning(t) => hooks.ui(Ui::Reasoning(t.clone())),
                            _ => {}
                        }
                        acc.apply(ev);
                    }
                },
            );

            match result {
                Ok(code) => {
                    status = code;
                    if code != 200 {
                        raw_error = String::from_utf8_lossy(&raw).chars().take(300).collect();
                    }
                }
                Err(e) => {
                    if attempt <= RETRIES && !self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        self.backoff(attempt, &e, hooks)?;
                        continue;
                    }
                    return Err(e);
                }
            }

            if is_context_overflow(status, &raw_error) && attempt == 1 {
                let freed = self.emergency_truncate();
                hooks.ui(Ui::Info(format!(
                    "context full; dropped {freed} old tool results and retrying once"
                )));
                continue;
            }

            if status == 400
                && raw_error.to_ascii_lowercase().contains("image")
                && attempt == 1
            {
                let dropped = self.drop_last_images();
                if dropped > 0 {
                    hooks.ui(Ui::Info(format!(
                        "model refused images; dropped {dropped} and retrying"
                    )));
                    continue;
                }
            }

            self.last_usage = (acc.cache_hit, acc.cache_miss, acc.completion_tokens);
            // Every request is billed, so every request is counted.
            self.task.hit += acc.cache_hit;
            self.task.miss += acc.cache_miss;
            self.task.out += acc.completion_tokens;
            hooks.ui(Ui::Usage {
                hit: acc.cache_hit,
                miss: acc.cache_miss,
                output: acc.completion_tokens,
                model: acc.model.clone(),
                context: crate::api::estimate_tokens(&self.history),
            });
            self.last_model = acc.model.clone();
            match api::classify(status, acc.finish.as_deref(), &raw_error) {
                Ok(()) if status == 200 => return Ok(acc),
                Ok(()) => {}
                Err(f) => {
                    if f.retry == api::Retry::Once && attempt <= RETRIES {
                        self.backoff(attempt, &f.message, hooks)?;
                        continue;
                    }
                    return Err(f.message);
                }
            }
            if attempt <= RETRIES && acc.finish.is_none() && acc.content.is_empty() {
                self.backoff(attempt, "empty stream", hooks)?;
                continue;
            }
            return Ok(acc);
        }
    }

    /// Wait before retry `attempt`: the provider's `Retry-After` when it gave
    /// one, capped at a minute, else 1, 2, 4, 8, 16 times the base plus up to a
    /// quarter of it in jitter. Counts down each second; Esc ends it at once.
    fn backoff(&self, attempt: u32, why: &str, hooks: &mut dyn Hooks) -> Result<(), String> {
        use std::sync::atomic::Ordering::Relaxed;
        use std::time::{Duration, Instant};
        if attempt == 1 {
            hooks.ui(Ui::Info(format!("retrying: {why}")));
        }
        let wait = match crate::transport::take_retry_after() {
            Some(secs) => Duration::from_secs(secs.min(60)),
            None => {
                let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
                self.retry_base * 2u32.pow(attempt - 1) + self.retry_base.mul_f64(0.25 * f64::from(nanos % 1000) / 1000.0)
            }
        };
        let end = Instant::now() + wait;
        let mut shown = u64::MAX;
        while let Some(left) = end.checked_duration_since(Instant::now()).filter(|d| !d.is_zero()) {
            if self.cancel.load(Relaxed) {
                return Err("cancelled".into());
            }
            let secs = left.as_secs() + u64::from(left.subsec_nanos() > 0);
            if secs != shown {
                shown = secs;
                hooks.ui(Ui::Retry { secs, attempt, of: RETRIES });
            }
            std::thread::sleep(left.min(Duration::from_millis(100)));
        }
        hooks.ui(Ui::Retry { secs: 0, attempt, of: RETRIES });
        if self.cancel.load(Relaxed) {
            return Err("cancelled".into());
        }
        Ok(())
    }

    /// Collapse old tool output to stubs once the wire gets large.
    pub fn prune(&mut self, hooks: &mut dyn Hooks) -> usize {
        let before = crate::api::estimate_tokens(&self.history);
        if before < self.cfg.prune_at as u64 {
            return 0;
        }
        let cutoff = self.history.len().saturating_sub(10);
        let mut pruned = 0;
        for (i, m) in self.history.iter_mut().enumerate() {
            if i >= cutoff || m.role != crate::api::Role::Tool {
                continue;
            }
            if m.content.len() <= 300 {
                continue;
            }
            let len = m.content.len();
            let head: String = m.content.chars().take(240).collect();
            m.content = format!("{head}\n… [pruned; {len} bytes]");
            pruned += 1;
        }
        if pruned > 0 {
            hooks.ui(Ui::Info(format!(
                "pruned {pruned} old tool results: {} → {} tokens",
                before,
                crate::api::estimate_tokens(&self.history)
            )));
        }
        pruned
    }

    pub fn attach_image(&mut self, image: Image) -> Result<(), String> {
        if self.attachments.len() >= crate::images::MAX_PER_MESSAGE {
            return Err(format!(
                "at most {} images per message",
                crate::images::MAX_PER_MESSAGE
            ));
        }
        self.attachments.push(image);
        Ok(())
    }

    fn drop_last_images(&mut self) -> usize {
        for message in self.history.iter_mut().rev() {
            if message.role == crate::api::Role::User && !message.images.is_empty() {
                let count = message.images.len();
                message.images.clear();
                return count;
            }
        }
        0
    }

    fn emergency_truncate(&mut self) -> usize {
        let cutoff = self.history.len().saturating_sub(6);
        let mut dropped = 0;
        for (i, m) in self.history.iter_mut().enumerate() {
            if i >= cutoff || m.role != crate::api::Role::Tool || m.content.len() <= 200 {
                continue;
            }
            let head: String = m.content.chars().take(160).collect();
            m.content = format!("{head}… [dropped: context full]");
            dropped += 1;
        }
        dropped
    }

    /// Summarise everything except the last four exchanges. `focus` is the text
    /// after the user's `/compact`, added to the request as one `Focus:` line;
    /// automatic compaction passes `None`.
    pub fn compact(&mut self, hooks: &mut dyn Hooks, focus: Option<&str>) -> Result<(), String> {
        const FOCUS_CHARS: usize = 500;
        let before = crate::api::estimate_tokens(&self.history);
        let Some(keep_from) = keep_index(&self.history) else {
            return Ok(());
        };
        let system_prefix: Vec<Message> = self
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::System)
            .cloned()
            .collect();
        let tail: Vec<Message> = self.history[keep_from..].to_vec();

        let mut transcript = String::new();
        for m in &self.history {
            if m.role == crate::api::Role::System {
                continue;
            }
            let body: String = m.content.chars().take(200).collect();
            transcript.push_str(&format!("{}: {body}\n", m.role.as_str()));
            for c in &m.tool_calls {
                transcript.push_str(&format!("  call {} {}\n", c.name, c.arguments));
            }
        }

        // Focus is trimmed, kept to one line, and cut at FOCUS_CHARS characters.
        let mut focus_line = String::new();
        if let Some(text) = focus.map(str::trim).filter(|t| !t.is_empty()) {
            if text.chars().count() > FOCUS_CHARS {
                hooks.ui(Ui::Info(format!("focus cut to {FOCUS_CHARS} characters")));
            }
            let cut: String = text.chars().take(FOCUS_CHARS).collect();
            focus_line = format!("Focus: {}\n", cut.replace('\n', " "));
        }

        let prompt = format!(
            "Summarise this conversation for another model that must continue the work.\n\
             Cover: decisions made, files touched, open threads, user preferences, unresolved errors.\n\
             Then add up to three lines starting with MEMORY: for durable facts worth saving.\n\
             Be terse. 400 tokens maximum.\n\n{focus_line}{transcript}"
        );
        let (summary, _) = self.complete_once(
            &[
                Message::system("You compress conversations without losing decisions."),
                Message::user(prompt),
            ],
            400,
        )?;

        let mut facts = facts_from_summary(&summary);
        for fact in facts.drain(..) {
            let title: String = fact.chars().take(48).collect();
            // Yolo saves without asking; every other mode offers each fact.
            let yolo = self.cfg.live_permissions() == Permissions::Yolo;
            if yolo || hooks.confirm(&format!("save note `{title}`")) {
                match self.memory.save(&title, &fact) {
                    Ok(p) => hooks.ui(Ui::Info(format!("saved {}", p.display()))),
                    Err(e) => hooks.ui(Ui::Error(format!("memory: {e}"))),
                }
            }
        }

        let mut rebuilt = system_prefix;
        rebuilt.push(Message::system(format!("[compacted]\n{summary}")));
        rebuilt.extend(tail);
        self.history = rebuilt;
        if let Some(log) = &self.log {
            log.event("compacted", &format!("{before} tokens before"));
        }
        hooks.ui(Ui::Info(format!(
            "compacted: {} → {} tokens",
            before,
            crate::api::estimate_tokens(&self.history)
        )));
        Ok(())
    }

    /// The user's likely next message, from the history as it stands, and the
    /// request's usage. `None` when the reply is not a usable suggestion.
    /// Whether a suggestion should follow the turn just ended. A suggestion
    /// after a decline asks why, or offers the declined step again.
    pub fn wants_suggestion(&self) -> bool {
        self.cfg.suggest && !self.task.declined
    }

    pub fn suggest(&self) -> Option<(String, (u64, u64, u64))> {
        let mut messages = self.history.clone();
        messages.push(Message::system(
            "Write the user's most likely next message, 2 to 8 words, in their voice. \
             End a question with a question mark. \
             Reply with only that message, or with nothing if there is no clear next step.",
        ));
        let (text, usage) = self.complete_once(&messages, 24).ok()?;
        Some((clean_suggestion(&text)?, usage))
    }

    /// A summary title for the session, after its first turn: one request
    /// holding only the first message. `None` when it is not wanted or fails.
    pub fn title_summary(&self) -> Option<(String, (u64, u64, u64))> {
        let users: Vec<&Message> = self.history.iter().filter(|m| m.role == crate::api::Role::User).collect();
        // Only after the first turn, so a failed try is never repeated.
        if users.len() != 1 {
            return None;
        }
        let first = users[0].content.clone();
        let stored = self.log.as_ref()?.meta().title;
        if !title_wanted(self.cfg.title_summary, self.cfg.provider, &first, &stored) {
            return None;
        }
        let (text, usage) = self.complete_once(&title_messages(&first), 24).ok()?;
        Some((title_from_reply(&text)?, usage))
    }

    fn complete_once(&self, messages: &[Message], max_tokens: u32) -> Result<(String, (u64, u64, u64)), String> {
        let opts = RequestOptions {
            model: self.cfg.model.clone(),
            max_tokens,
            thinking: false,
            temperature: Some(0.0),
            effort: None,
        };
        let mut adapter = crate::providers::adapter(
            self.cfg.provider,
            &self.cfg.base_url,
            &self.cfg.api_key,
        );
        let wire = adapter.wire(&opts, messages, &[]);
        let headers: Vec<&str> = wire.headers.iter().map(String::as_str).collect();
        let mut parser = SseParser::default();
        let mut acc = Accum::default();
        let result = self.transport.post(
            &wire.url,
            &headers,
            wire.body.to_string().as_bytes(),
            &self.cancel,
            &mut |b| {
                for ev in parser.push(b, adapter.as_mut()) {
                    acc.apply(ev);
                }
            },
        );
        match result {
            Ok(200) => {
                let usage = (acc.cache_hit, acc.cache_miss, acc.completion_tokens);
                Ok((acc.content, usage))
            }
            Ok(code) => Err(format!("summariser failed: HTTP {code}")),
            Err(e) => Err(e),
        }
    }

    /// One user turn: stream, run tools, repeat until the model answers.
    pub fn turn(&mut self, user: &str, hooks: &mut dyn Hooks) -> Result<(), String> {
        self.cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        self.task = TaskTotals::default();
        self.note_width();
        self.note_date(crate::prompt::day_number());
        let mut message = Message::user(user);
        message.images = std::mem::take(&mut self.attachments);
        self.say(message);

        let max_steps = self.cfg.max_steps.max(1);
        for step in 0..max_steps {
            // The chat holds the turn while the user edits a queued message.
            while self.hold.load(std::sync::atomic::Ordering::SeqCst) && !self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                hooks.ui(Ui::Info("cancelled".into()));
                return Ok(());
            }
            // Messages the user steered in join before the next request.
            let steered = self.steer.lock().map(|mut s| std::mem::take(&mut *s)).unwrap_or_default();
            for text in steered {
                self.say(Message::user(&text));
            }
            self.prune(hooks);
            if crate::api::estimate_tokens(&self.history) >= self.cfg.compact_at as u64 {
                if let Err(e) = self.compact(hooks, None) {
                    hooks.ui(Ui::Error(format!("compaction failed: {e}")));
                }
            }
            for (id, code, path) in self.jobs.poll() {
                // Reported here, so its end needs no wake.
                self.notify.remove(&id);
                let note = format!(
                    "Background job {id} finished with exit {code}; output: {}",
                    path.display()
                );
                hooks.ui(Ui::Notice(note.clone()));
                self.say(Message::system(note));
            }
            self.cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let acc = match self.request(hooks) {
                Err(_) if self.cancel.load(std::sync::atomic::Ordering::Relaxed) => {
                    hooks.ui(Ui::Info("cancelled".into()));
                    return Ok(());
                }
                other => other?,
            };

            let mut assistant = Message::assistant(acc.content.clone());
            assistant.reasoning = acc.reasoning.clone();
            assistant.tool_calls = acc.tool_calls();
            let calls = assistant.tool_calls.clone();
            self.say(assistant);

            if calls.is_empty() {
                return Ok(());
            }

            for call in calls {
                let out = if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    "refused: the turn was cancelled".into()
                } else {
                    self.dispatch(&call, hooks)
                };
                self.say(Message::tool(call.id.clone(), out));
            }

            if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                hooks.ui(Ui::Info("cancelled".into()));
                return Ok(());
            }

            if step + 1 == max_steps {
                hooks.ui(Ui::Info(format!(
                    "paused after {max_steps} steps; send another message to continue"
                )));
                return Ok(());
            }
        }
        Ok(())
    }

    /// Switch the session to another root, reloading everything that depends
    /// on it and recording the change for a later resume.
    pub fn set_root(&mut self, root: std::path::PathBuf) -> Result<(), String> {
        if !root.is_dir() {
            return Err(format!("{}: not a directory", root.display()));
        }
        self.tools.root = root;
        self.reload()?;
        if let Some(log) = &self.log {
            log.set_cwd(&self.tools.root)?;
            log.event("cwd", &self.tools.root.display().to_string());
        }
        Ok(())
    }

    /// Forget one shape allowed in this session, and its config line when it
    /// was saved for every session.
    pub fn revoke_allow(&mut self, shape: &str, always: bool) -> Result<(), String> {
        self.always.remove(shape);
        if always {
            self.cfg.revoke_always(shape)?;
        }
        Ok(())
    }

    /// Apply an exact mode, including the path jail.
    pub fn set_permissions(&mut self, mode: Permissions) {
        self.cfg.set_permissions(mode);
        self.tools.jail = crate::perm::jail_enabled(mode);
    }

    /// A check-in due now, for tests of the worker.
    #[cfg(test)]
    pub fn check_in_now(&mut self) {
        let now = std::time::Instant::now();
        self.checkins.push((now, None, now));
    }

    /// A note for a turn the session should start on its own: due check-ins,
    /// and ends of jobs the model asked to be told about. Other jobs that
    /// ended are recorded as system notes without waking anything.
    pub fn due(&mut self) -> Option<String> {
        let now = std::time::Instant::now();
        let mut lines = Vec::new();
        for (id, code, path) in self.jobs.poll() {
            if self.notify.remove(&id) {
                let tail = crate::jobs::tail(&path, 600);
                lines.push(format!("Job {id} ended with exit {code}. The end of its output:\n{tail}"));
            } else {
                self.say(Message::system(format!(
                    "Background job {id} finished with exit {code}; output: {}",
                    path.display()
                )));
            }
        }
        let (due, later): (Vec<_>, Vec<_>) = self.checkins.drain(..).partition(|(at, _, _)| *at <= now);
        self.checkins = later;
        for (_, job, asked) in due {
            let ago = crate::jobs::span(now - asked);
            let state = match job {
                Some(id) => self.jobs.status(id).unwrap_or_else(|| format!("job {id} is gone")),
                None => self.jobs.list(),
            };
            lines.push(format!("The check-in you asked for {ago} ago:\n{state}"));
        }
        (!lines.is_empty()).then(|| {
            format!(
                "[Check-in from imp, not the user.]\n{}\nTell the user briefly how it is going. \
                 If it is still running and worth another look, schedule a job check.",
                lines.join("\n\n")
            )
        })
    }

    /// Tell the model the date before its first request and again when the
    /// day changes, so a session left open past midnight, or opened days
    /// after imp started, never works from a stale date.
    fn note_date(&mut self, day: u64) {
        if self.last_day == Some(day) {
            return;
        }
        self.last_day = Some(day);
        self.say(Message::system(format!("Today is {}.", crate::prompt::date_of(day))));
    }

    /// Set a goal and return the first round's message.
    pub fn start_goal(&mut self, text: &str) -> String {
        let rounds = self.cfg.goal_rounds.clamp(1, 20);
        self.goal = Some(Goal { text: text.to_string(), round: 1, rounds, paused: false });
        self.declined = false;
        self.save_goal();
        format!(
            "[Goal from imp, not the user.] Work until this goal is met: {text}\n{}",
            GOAL_ENDINGS
        )
    }

    /// Pause the goal: it is kept with its round, and no round runs until resumed.
    pub fn pause_goal(&mut self) {
        if let Some(goal) = &mut self.goal {
            goal.paused = true;
        }
        self.save_goal();
    }

    /// Write the goal to the session file, so it survives a restart.
    pub fn save_goal(&self) {
        if let Some(log) = &self.log {
            log.set_goal(self.goal.as_ref().map(|g| crate::store::SavedGoal {
                text: g.text.clone(),
                round: g.round,
                rounds: g.rounds,
                paused: g.paused,
            }));
        }
    }

    /// Clear the goal: it is removed.
    pub fn clear_goal(&mut self) {
        self.goal = None;
        self.save_goal();
    }

    /// Resume a paused goal: the message for its saved round, without advancing it.
    pub fn resume_goal(&mut self) -> Result<String, String> {
        let Some(goal) = self.goal.as_mut() else { return Err("no goal to resume".into()) };
        goal.paused = false;
        self.save_goal();
        let Some(goal) = self.goal.as_ref() else { return Err("no goal to resume".into()) };
        Ok(format!(
            "[Goal check from imp, not the user.] Round {} of {}. Goal: {}\n{GOAL_ENDINGS}",
            goal.round, goal.rounds, goal.text
        ))
    }

    /// Change the goal's text. The round and limit stay, and no round starts.
    pub fn edit_goal(&mut self, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("the goal needs text".into());
        }
        let Some(goal) = self.goal.as_mut() else { return Err("no goal to edit".into()) };
        goal.text = text.to_string();
        self.save_goal();
        Ok(())
    }

    /// After a round: the next round's message, or why the goal ended. The
    /// goal is cleared when it ends.
    pub fn goal_next(&mut self) -> Result<String, String> {
        let Some(goal) = self.goal.clone() else { return Err("no goal".into()) };
        let last = self
            .history
            .iter()
            .rev()
            .find(|m| m.role == crate::api::Role::Assistant && !m.content.trim().is_empty())
            .and_then(|m| m.content.lines().rev().find(|l| !l.trim().is_empty()))
            .unwrap_or("")
            .trim()
            .trim_matches(|c: char| c == '*' || c == '_' || c == '`')
            .to_string();
        let upper = last.to_ascii_uppercase();
        let end = if upper.starts_with("GOAL MET") {
            Some("goal met".to_string())
        } else if upper.starts_with("GOAL BLOCKED") {
            let why = last["GOAL BLOCKED".len()..].trim_start_matches([':', ' ']).trim();
            Some(if why.is_empty() { "goal blocked".into() } else { format!("goal blocked: {why}") })
        } else if std::mem::take(&mut self.declined) {
            Some("goal stopped: a confirmation was declined".into())
        } else if goal.round >= goal.rounds {
            Some(format!("goal not met after {} rounds", goal.rounds))
        } else {
            None
        };
        if let Some(why) = end {
            self.goal = None;
            self.save_goal();
            return Err(why);
        }
        let round = goal.round + 1;
        if let Some(g) = &mut self.goal {
            g.round = round;
        }
        self.save_goal();
        Ok(format!(
            "[Goal check from imp, not the user.] Round {round} of {}. Goal: {}\n{GOAL_ENDINGS}",
            goal.rounds, goal.text
        ))
    }

    /// Tell the model how wide the screen is, once per change.
    fn note_width(&mut self) {
        let width = self.cols.load(std::sync::atomic::Ordering::Relaxed);
        let rows = crate::tui::screen().map_or(0, |(_, r)| r);
        if width < 20 || self.last_cols == Some((width, rows)) {
            return;
        }
        self.last_cols = Some((width, rows));
        let size = if rows > 0 {
            format!("{width} columns wide and {rows} rows high")
        } else {
            format!("{width} columns wide")
        };
        self.say(Message::system(format!(
            "Environment: the terminal is {size}. Keep code, tables, and rules inside it."
        )));
    }

    /// The `task` tool: look up the profile, then run it as a child.
    fn run_task(&mut self, args: &Value, hooks: &mut dyn Hooks) -> (bool, String) {
        if self.child_call.is_some() {
            return (false, "refused: a subagent cannot start another".into());
        }
        let Some(agent) = args.get("agent").and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) else {
            return (false, "refused: task needs an agent name".into());
        };
        let Some(prompt) = args.get("prompt").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) else {
            return (false, "refused: task needs a prompt".into());
        };
        let Some(profile) = self.roster.iter().find(|a| a.name == agent).cloned() else {
            let names: Vec<&str> = self.roster.iter().map(|a| a.name.as_str()).collect();
            return (false, format!("refused: no such agent `{agent}`; available: {}", names.join(", ")));
        };
        let call = self.current_call.clone();
        self.run_child(&call, profile, prompt, hooks)
    }

    /// Run `profile` on `prompt` in a fresh history, then restore the parent's
    /// history, profile, model, thinking, effort, and child flag on every path.
    fn run_child(&mut self, call: &str, profile: Agent, prompt: &str, hooks: &mut dyn Hooks) -> (bool, String) {
        let mut seed: Vec<Message> = self
            .history
            .first()
            .filter(|m| m.role == crate::api::Role::System)
            .cloned()
            .into_iter()
            .collect();
        if !profile.instructions.is_empty() {
            seed.push(Message::system(profile.instructions.clone()));
        }
        seed.push(Message::user(prompt));

        let saved_history = std::mem::replace(&mut self.history, seed);
        let saved_agent = self.agent.clone();
        let saved_cfg = (self.cfg.model.clone(), self.cfg.thinking, self.cfg.effort.clone());
        let saved_call = self.child_call.replace(call.to_string());
        if let Some(model) = &profile.model {
            self.cfg.model = model.clone();
        }
        if let Some(thinking) = profile.thinking {
            self.cfg.thinking = thinking;
        }
        if profile.effort.is_some() {
            self.cfg.effort = profile.effort.clone();
        }
        let name = profile.name.clone();
        self.agent = profile;

        let result = {
            let mut child = ChildHooks { parent: hooks, name };
            self.child_loop(&mut child)
        };

        let transcript = std::mem::replace(&mut self.history, saved_history);
        self.children.insert(call.to_string(), transcript);
        self.agent = saved_agent;
        self.cfg.model = saved_cfg.0;
        self.cfg.thinking = saved_cfg.1;
        self.cfg.effort = saved_cfg.2;
        self.child_call = saved_call;
        result
    }

    /// The child's step loop: request, dispatch its tools, repeat until it
    /// answers in text or reaches the step cap.
    fn child_loop(&mut self, hooks: &mut dyn Hooks) -> (bool, String) {
        let max = self.cfg.max_steps.max(1);
        let mut last = String::new();
        for _ in 0..max {
            let acc = match self.request(hooks) {
                Ok(acc) => acc,
                Err(_) if self.cancel.load(std::sync::atomic::Ordering::Relaxed) => {
                    return (false, "cancelled".into());
                }
                Err(e) => return (false, format!("subagent failed: {e}")),
            };
            let mut assistant = Message::assistant(acc.content.clone());
            assistant.reasoning = acc.reasoning.clone();
            assistant.tool_calls = acc.tool_calls();
            let calls = assistant.tool_calls.clone();
            last = acc.content.clone();
            self.say(assistant);
            if calls.is_empty() {
                return (true, acc.content);
            }
            for call in calls {
                let out = self.dispatch(&call, hooks);
                self.say(Message::tool(call.id.clone(), out));
            }
        }
        (true, format!("{last}\n\n(stopped at the step cap of {max})"))
    }

    fn dispatch(&mut self, call: &ToolCall, hooks: &mut dyn Hooks) -> String {
        let out = self.dispatch_inner(call, hooks);
        if out == "refused: the user did not answer" {
            self.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        // Every decline reads "refused: the user declined …".
        if out.starts_with("refused: the user declined") {
            self.declined = true;
            self.task.declined = true;
        }
        out
    }

    fn dispatch_inner(&mut self, call: &ToolCall, hooks: &mut dyn Hooks) -> String {
        self.task.steps += 1;
        let args: Value = match serde_json::from_str(&call.arguments) {
            Ok(v) => v,
            Err(e) => return format!("bad arguments: {e}"),
        };
        hooks.ui(Ui::ToolStart {
            name: call.name.clone(),
            summary: relative(describe(&call.name, &args), &self.tools.root),
            args: call.arguments.clone(),
        });

        let start = std::time::Instant::now();
        self.current_call = call.id.clone();
        let (ok, out) = self.run_tool(&call.name, &args, hooks);
        // No secret value reaches the conversation, the log, or the screen.
        let out = crate::secrets::redact(&out);
        let spent = start.elapsed().saturating_sub(hooks.last_wait());
        let detail = if ok && self.cfg.diffs { diff_detail(&call.name, &args) } else { String::new() };
        hooks.ui(Ui::ToolEnd {
            ok,
            // A success shows what came back; a failure shows why.
            preview: if ok { gist(&call.name, &out) } else { summarise(&out) },
            output: out.clone(),
            detail,
            ms: spent.as_millis(),
        });
        out
    }

    /// Run one tool call. The flag says whether the call did what it set out to do.
    fn run_tool(&mut self, name: &str, args: &Value, hooks: &mut dyn Hooks) -> (bool, String) {
        match name {
            "task" => self.run_task(args, hooks),
            "bash" => {
                let Some(cmd) = args.get("command").and_then(Value::as_str) else {
                    return (false, "refused: bash needs a command".into());
                };
                if let Some(rule) =
                    perm::shell_touches_self_config(cmd, self.cfg.live_permissions())
                {
                    match rule {
                        Decision::Deny(why) => return (false, why),
                        Decision::Ask => {
                            if !hooks.confirm(&format!("run `{cmd}`")) {
                                return (false, "refused: the user declined to run this command".into());
                            }
                        }
                        Decision::Allow => {}
                    }
                }
                let mode = self.cfg.live_permissions();
                let decision = perm::classify_command(cmd, mode, self.cfg.allow_sudo);
                // Naming a stripped secret shows intent, even though it expands
                // to nothing now.
                let named = crate::secrets::mentioned(cmd);
                let decision = match decision {
                    Decision::Allow if named.is_some() && mode != Permissions::Yolo => Decision::Ask,
                    // Leaving the project is worth a look, even to read.
                    Decision::Allow if mode != Permissions::Yolo && perm::leaves_dir(cmd, &self.tools.root) => Decision::Ask,
                    other => other,
                };
                // In the foreground: the deny list holds, and it always asks
                // unless the mode is yolo, whatever is allowed.
                if args.get("foreground").and_then(Value::as_bool).unwrap_or(false) {
                    if let Decision::Deny(why) = decision {
                        return (false, why);
                    }
                    if !hooks.can_foreground() {
                        return (false, "refused: foreground needs the chat interface".into());
                    }
                    if mode != Permissions::Yolo && !hooks.confirm(&format!("run `{cmd}`")) {
                        return (false, "refused: the user declined to run this command".into());
                    }
                    return match hooks.run_attached(&in_dir(&self.tools.root, cmd)) {
                        Some(code) => (code == 0, format!("exit {code}\n(ran in the foreground; output not captured)")),
                        None => (false, "refused: foreground needs the chat interface".into()),
                    };
                }
                // A plain curl GET shares the per-host fetch approval.
                // Without a scratch folder `$IMP_TMP` is empty, and the target would be `/`.
                let scratch = || crate::tools::scratch().and_then(|_| perm::scratch_download(cmd));
                let get = perm::plain_get(cmd).or_else(scratch).filter(|_| !matches!(decision, Decision::Deny(_)) && named.is_none());
                if let Some(url) = get {
                    if perm::classify_fetch(mode, &url) == Decision::Ask && !hooks.confirm(&perm::fetch_question(&url)) {
                        return (false, format!("refused: the user declined fetching from {}", perm::url_host(&url)));
                    }
                } else {
                    match decision {
                        Decision::Deny(why) => return (false, why),
                        Decision::Ask => {
                            // `always` only for one simple command in normal.
                            let shape = perm::shape(cmd).filter(|_| matches!(mode, Permissions::Normal | Permissions::Auto));
                            let known = shape
                                .as_ref()
                                .is_some_and(|k| self.always.contains(k) || self.cfg.allowed(k));
                            let what = format!("run `{cmd}`");
                            let answer = match &shape {
                                _ if known => Answer::Yes,
                                Some(_) => hooks.confirm_always(&what),
                                None if hooks.confirm(&what) => Answer::Yes,
                                None => Answer::No,
                            };
                            if let Some(k) = &shape {
                                hooks.answered(k, answer);
                            }
                            match (answer, shape) {
                                (Answer::No, _) => {
                                    return (false, "refused: the user declined to run this command".into())
                                }
                                (Answer::Session, Some(k)) => {
                                    self.always.insert(k.clone());
                                    hooks.allowed(&k, false);
                                }
                                (Answer::Always, Some(k)) => {
                                    match self.cfg.allow_always(&k) {
                                        Ok(()) => hooks.allowed(&k, true),
                                        Err(e) => {
                                            hooks.ui(Ui::Error(format!("allow {k} not saved: {e}")))
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Decision::Allow => {}
                    }
                }
                // Only after the deny list, the opt-in, and any confirmation.
                if cmd.split_whitespace().next() == Some("sudo") {
                    if let Some(code) = hooks.run_attached(&in_dir(&self.tools.root, cmd)) {
                        return (code == 0, format!("exit {code}\n(ran attached to the terminal)"));
                    }
                }
                let delay = args.get("delay_s").and_then(Value::as_u64);
                if delay.is_some_and(|d| !(1..=86_400).contains(&d)) {
                    return (false, "refused: delay_s is 1 to 86400 seconds".into());
                }
                if delay.is_some() || args.get("background").and_then(Value::as_bool).unwrap_or(false) {
                    let wait = std::time::Duration::from_secs(delay.unwrap_or(0));
                    let notify = args.get("notify").and_then(Value::as_bool).unwrap_or(false);
                    if notify && !self.wakes {
                        return (false, "refused: notify needs the chat interface; use job wait in line mode".into());
                    }
                    return match self.jobs.start_after(cmd, &self.job_dir, wait) {
                        Ok(id) => {
                            if notify {
                                self.notify.insert(id);
                            }
                            (
                                true,
                                format!(
                                    "job {id} {} in the background; output: {}",
                                    if wait.is_zero() { "started".to_string() } else { format!("starts in {}", crate::jobs::span(wait)) },
                                    self.job_dir.join(format!("job-{id}.log")).display()
                                ),
                            )
                        }
                        Err(e) => (false, format!("refused: could not start job: {e}")),
                    };
                }
                let o = self.tools.bash_handover(cmd, &self.cancel);
                if let Some(h) = self.tools.handover.take() {
                    // Too slow to wait for: it carries on as a job, nothing reruns.
                    let log = h.log.display().to_string();
                    let id = self.jobs.adopt(&h.command, h.child, h.log);
                    let secs = self.tools.timeout.as_secs();
                    return (
                        true,
                        format!("still running after {secs}s; now job {id}; log {log}; use job wait {id}"),
                    );
                }
                let show = args.get("show").and_then(Value::as_bool).unwrap_or(false);
                if let (true, Some(full)) = (show, &o.full) {
                    let full = crate::secrets::redact(full);
                    hooks.ui(Ui::Show(first_lines(&full, SHOW_LINES)));
                    return (o.ok, format!("exit {}\n{}", o.code.unwrap_or(-1), self.tools.cap(&for_model(&full))));
                }
                // A file printed whole counts as read; a capped one was not seen whole.
                if o.ok && !o.text.contains("imp://out/") {
                    self.tools.mark_read(&perm::cat_paths(cmd));
                }
                (o.ok, format!("exit {}\n{}", o.code.unwrap_or(-1), o.text))
            }
            "read" => {
                let paths: Vec<String> = args
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                if paths.is_empty() {
                    return (false, "refused: read needs at least one path".into());
                }
                let offset = args.get("offset").and_then(Value::as_u64).map(|n| n as usize);
                let limit = args.get("limit").and_then(Value::as_u64).map(|n| n as usize);
                let depth = args.get("depth").and_then(Value::as_u64).map(|n| n as usize);
                let o = self.tools.read(&paths, offset, limit, depth);
                (o.ok, o.text)
            }
            "write" => {
                let path = args.get("path").and_then(Value::as_str).unwrap_or("");
                let content = args.get("content").and_then(Value::as_str).unwrap_or("");
                if let Err(why) = self.allowed_write(path, hooks) {
                    return (false, why.replace("{what}", "write"));
                }
                self.remember_original(path);
                let o = self.tools.write(path, content);
                (o.ok, o.text)
            }
            "edit" => {
                let path = args.get("path").and_then(Value::as_str).unwrap_or("");
                let mut pairs = Vec::new();
                if let Some(list) = args.get("edits").and_then(Value::as_array) {
                    for e in list {
                        let old = e.get("old").and_then(Value::as_str).unwrap_or("");
                        let new = e.get("new").and_then(Value::as_str).unwrap_or("");
                        pairs.push((old.to_string(), new.to_string()));
                    }
                }
                if pairs.is_empty() {
                    return (false, "refused: edit needs at least one old/new pair".into());
                }
                if let Err(why) = self.allowed_write(path, hooks) {
                    return (false, why.replace("{what}", "edit"));
                }
                self.remember_original(path);
                let o = self.tools.edit(path, &pairs);
                (o.ok, o.text)
            }
            "job" => {
                let op = args.get("op").and_then(Value::as_str).unwrap_or("list");
                match op {
                    "list" => {
                        let done = self.jobs.poll();
                        for (id, code, _) in done {
                            // The listing tells the model; its end needs no wake.
                            self.notify.remove(&id);
                            hooks.ui(Ui::Notice(format!("job {id} finished, exit {code}")));
                        }
                        (true, self.jobs.list())
                    }
                    "check" => {
                        let after = args.get("after_s").and_then(Value::as_u64).unwrap_or(0);
                        let job = args.get("id").and_then(Value::as_u64);
                        if !self.wakes {
                            (false, "refused: check-ins need the chat interface; use job wait in line mode".into())
                        } else if !(30..=86_400).contains(&after) {
                            (false, "refused: after_s is 30 to 86400 seconds".into())
                        } else if self.checkins.len() >= MAX_CHECKINS {
                            (false, format!("refused: {MAX_CHECKINS} check-ins are already pending"))
                        } else if job.is_some_and(|id| self.jobs.status(id).is_none()) {
                            (false, format!("refused: no job {}", job.unwrap_or(0)))
                        } else {
                            let now = std::time::Instant::now();
                            self.checkins.push((now + std::time::Duration::from_secs(after), job, now));
                            let what = job.map_or("the jobs".to_string(), |id| format!("job {id}"));
                            (true, format!("check-in on {what} in {}", crate::jobs::span(std::time::Duration::from_secs(after))))
                        }
                    }
                    "kill" => match args.get("id").and_then(Value::as_u64) {
                        Some(id) => (true, self.jobs.kill(id).unwrap_or_else(|e| e)),
                        None => (false, "refused: job kill needs an id".into()),
                    },
                    "wait" => match args.get("id").and_then(Value::as_u64) {
                        Some(id) => {
                            let secs = args
                                .get("timeout_s")
                                .and_then(Value::as_u64)
                                .unwrap_or(55)
                                .min(55);
                            let out = self.jobs.wait(id, std::time::Duration::from_secs(secs)).unwrap_or_else(|e| e);
                            if !out.contains("still running") {
                                self.notify.remove(&id);
                            }
                            (true, out)
                        }
                        None => (false, "refused: job wait needs an id".into()),
                    },
                    other => (false, format!("refused: unknown job op {other}")),
                }
            }
            "fetch" => self.fetch_tool(args, hooks),
            "search" => {
                if !self.cfg.provider.caps().search {
                    return (false, "search is not available for this provider".into());
                }
                (true, self.search_tool(args))
            }

            "memory" => {
                let op = args.get("op").and_then(Value::as_str).unwrap_or("search");
                match op {
                    "search" => {
                        let q = args.get("query").and_then(Value::as_str).unwrap_or("");
                        let hits = self.memory.search(q, 5);
                        if hits.is_empty() {
                            (true, format!("no notes match {q:?}"))
                        } else {
                            (
                                true,
                                hits.iter()
                                    .map(|(p, x)| format!("{}: {x}", p.display()))
                                    .collect::<Vec<_>>()
                                    .join("\n"),
                            )
                        }
                    }
                    "save" => {
                        let title = args.get("title").and_then(Value::as_str).unwrap_or("");
                        let body = args.get("body").and_then(Value::as_str).unwrap_or("");
                        if title.is_empty() || body.is_empty() {
                            return (false, "refused: memory save needs a title and a body".into());
                        }
                        let target = self
                            .memory
                            .dirs
                            .first()
                            .map(|d| d.join(format!("{}.md", crate::store::slugify(title))))
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        let yolo = self.cfg.live_permissions() == Permissions::Yolo;
                        if !yolo && !hooks.confirm(&format!("save note {target}")) {
                            return (false, "refused: the user declined to save this note".into());
                        }
                        match self.memory.save(title, body) {
                            Ok(p) => (true, format!("saved {}", p.display())),
                            Err(e) => (false, format!("refused: {e}")),
                        }
                    }
                    other => (false, format!("refused: unknown memory op {other}")),
                }
            }
            "goal" => {
                // The agent proposes; the user confirms. Nothing runs until a yes.
                if args.get("operation").and_then(Value::as_str) != Some("set") {
                    return (false, "refused: the goal tool supports only operation \"set\"".into());
                }
                let text = args.get("text").and_then(Value::as_str).unwrap_or("").trim().to_string();
                if text.is_empty() {
                    return (false, "refused: the goal text is empty".into());
                }
                if self.goal.is_some() {
                    return (false, "refused: a goal is already set; the user must clear it first".into());
                }
                if hooks.confirm(&format!("set this goal: {text}?")) {
                    self.start_goal(&text);
                    (true, format!("goal confirmed; it runs in rounds after this reply: {text}"))
                } else {
                    (false, "the user declined the goal; do not set it again unless the user asks".into())
                }
            }
            "skill_load" => {
                let name = args.get("name").and_then(Value::as_str).unwrap_or("");
                match self.skills.iter().find(|s| s.name == name) {
                    Some(s) => {
                        let body: String = s.body.chars().take(4096).collect();
                        if s.body.chars().count() > 4096 {
                            (true, format!("{body}\n[truncated at 4 KB]"))
                        } else {
                            (true, body)
                        }
                    }
                    None if self.cfg.skill_disabled(name) => {
                        (false, format!("refused: skill {name:?} is disabled"))
                    }
                    None => (
                        false,
                        format!(
                            "refused: no skill {name:?}; available: {}",
                            self.skills.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
                        ),
                    ),
                }
            }
            "ask" => {
                if let Some(list) = args.get("questions") {
                    let Some(list) = list.as_array().filter(|v| !v.is_empty()) else {
                        return (false, "refused: ask needs a nonempty questions array".into());
                    };
                    let questions: Vec<AskQuestion> = match list.iter().map(ask_question).collect() {
                        Ok(v) => v,
                        Err(e) => return (false, e),
                    };
                    match hooks.ask_many(&questions) {
                        Some(answers) if answers.len() == questions.len() && answers.iter().all(|v| !v.is_empty() && v.iter().all(|s| !s.is_empty())) =>
                            (true, json!({"answers": answers}).to_string()),
                        _ => (false, "refused: the user did not answer".into()),
                    }
                } else {
                    let q = match ask_question(args) { Ok(q) => q, Err(e) => return (false, e) };
                    match hooks.ask_choices(&q.question, &q.options, q.multiple) {
                        Some(answer) if !answer.is_empty() => (true, answer),
                        _ => (false, "refused: the user did not answer".into()),
                    }
                }
            }
            other => (false, format!("refused: unknown tool {other}")),
        }
    }

    fn search_tool(&mut self, args: &Value) -> String {
        let op = args.get("op").and_then(Value::as_str).unwrap_or("search");
        let url = match op {
            "search" => format!("{}/v1/tools/search", self.cfg.base_url),
            other => return format!("unknown search op {other}"),
        };
        let body = match op {
            "search" => {
                let q = args.get("query").and_then(Value::as_str).unwrap_or("");
                if q.is_empty() {
                    return "search needs a query".into();
                }
                serde_json::json!({
                    "query": q,
                    "count": args.get("count").and_then(Value::as_u64).unwrap_or(5).min(10),
                    "need_content": false
                })
            }
            _ => unreachable!("only search reaches here"),
        };

        let auth = format!("Authorization: Bearer {}", self.cfg.api_key);
        let headers = ["Content-Type: application/json", auth.as_str()];
        let mut raw = Vec::new();
        let resp = self.transport.post(&url, &headers, body.to_string().as_bytes(), &self.cancel, &mut |b| {
            if raw.len() < 2 * 1024 * 1024 {
                raw.extend_from_slice(b);
            }
        });
        match resp {
            Ok(200) => {}
            Ok(code) => return format!("{op} failed: HTTP {code}"),
            Err(e) => return format!("{op} failed: {e}"),
        }
        let text = String::from_utf8_lossy(&raw);
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            return "the search endpoint returned something unreadable".into();
        };

        match op {
            "search" => {
                let mut out = String::new();
                for item in v["results"].as_array().cloned().unwrap_or_default() {
                    let score = item["rerank_score"].as_f64().unwrap_or(0.0);
                    if score > 0.0 && score < 1.5 {
                        continue;
                    }
                    out.push_str(&format!(
                        "{}\n{}\n{}\n\n",
                        item["title"].as_str().unwrap_or(""),
                        item["url"].as_str().unwrap_or(""),
                        item["summary"].as_str().unwrap_or(item["snippet"].as_str().unwrap_or(""))
                    ));
                }
                if out.is_empty() {
                    "no results".into()
                } else {
                    self.tools.cap(&out)
                }
            }
            _ => unreachable!("only search reaches here"),
        }
    }

    /// One HTTP request: a GET reads freely; a write, or a read carrying the
    /// configured credentials, asks as the mode says.
    fn fetch_tool(&mut self, args: &Value, hooks: &mut dyn Hooks) -> (bool, String) {
        let url = args.get("url").and_then(Value::as_str).unwrap_or("").trim();
        if !crate::tools::fetchable(url) {
            return (false, "refused: fetch takes an http or https URL".into());
        }
        let method = args.get("method").and_then(Value::as_str).unwrap_or("GET").to_ascii_uppercase();
        if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
            return (false, format!("refused: fetch sends GET, POST, PUT, PATCH, or DELETE, not {method}"));
        }
        let host = perm::url_host(url);
        let auth = self.cfg.auth.iter().find(|(h, _)| *h == host).map(|(_, header)| header.clone());

        let mut headers: Vec<String> = Vec::new();
        if let Some(map) = args.get("headers").and_then(Value::as_object) {
            for (name, value) in map {
                let value = value.as_str().map_or_else(|| value.to_string(), str::to_string);
                if name.contains([':', '\r', '\n']) || value.contains(['\r', '\n']) {
                    return (false, format!("refused: header {name:?} is malformed"));
                }
                headers.push(format!("{name}: {value}"));
            }
        }
        // JSON goes as JSON, whether the model sent an object or its text;
        // other text as plain text. A header the model sets wins.
        let (body, kind): (Option<Vec<u8>>, &str) = match args.get("body") {
            None | Some(Value::Null) => (None, ""),
            Some(Value::String(text)) if serde_json::from_str::<Value>(text).is_ok_and(|v| v.is_object() || v.is_array()) => {
                (Some(text.clone().into_bytes()), "application/json")
            }
            Some(Value::String(text)) => (Some(text.clone().into_bytes()), "text/plain; charset=utf-8"),
            Some(json) => (Some(json.to_string().into_bytes()), "application/json"),
        };
        if body.is_some() && !headers.iter().any(|h| h.to_ascii_lowercase().starts_with("content-type:")) {
            headers.push(format!("Content-Type: {kind}"));
        }

        let mode = self.cfg.live_permissions();
        let what = describe("fetch", args);
        let with = if auth.is_some() { format!(" with your {host} credentials") } else { String::new() };
        let question = match (mode, method.as_str()) {
            (Permissions::Yolo, _) => None,
            (Permissions::Strict, _) => Some(format!("{what}{with}")),
            (Permissions::Normal | Permissions::Auto, "GET") if auth.is_some() => {
                (!self.auth_hosts.contains(&host)).then(|| format!("{what}{with} for this session"))
            }
            (Permissions::Normal | Permissions::Auto, "GET") => {
                (perm::classify_fetch(mode, url) == Decision::Ask).then(|| perm::fetch_question(url))
            }
            (Permissions::Normal | Permissions::Auto, _) => {
                let size = body.as_ref().map_or(String::new(), |b| format!(", a {} B body", b.len()));
                Some(format!("{what}{size}{with}"))
            }
        };
        if let Some(q) = question {
            if !hooks.confirm(&q) {
                return (false, format!("refused: the user declined {method} {host}"));
            }
            if method == "GET" && auth.is_some() {
                self.auth_hosts.insert(host.clone());
            }
        }

        if let Some(template) = &auth {
            match crate::secrets::expand(template) {
                Ok(header) => headers.push(header),
                Err(name) => return (false, format!("refused: the {host} credentials need ${name}, which is not set")),
            }
        }
        let refs: Vec<&str> = headers.iter().map(String::as_str).collect();
        // Credentials never follow a redirect: curl would forward the header.
        let follow = method == "GET" && auth.is_none();
        match crate::http::call(&method, url, &refs, body.as_deref(), follow, 15_000, 2 * 1024 * 1024) {
            Ok((reply, text)) => {
                let kind = reply.content_type.to_ascii_lowercase();
                let start = text.trim_start();
                let xml = kind.contains("xml") || kind.contains("rss") || kind.contains("atom") || start.starts_with("<?xml");
                let html = !xml && (kind.contains("html") || start.starts_with('<') && kind.is_empty());
                let text = if html {
                    crate::web::markdown(&text, url)
                } else if xml {
                    crate::web::feed(&text).unwrap_or(text)
                } else {
                    text
                };
                let kind = kind.split(';').next().unwrap_or("").trim().to_string();
                let mut head = format!("{} {kind}", reply.status);
                for h in &reply.headers {
                    head.push('\n');
                    head.push_str(h);
                }
                (reply.status < 400, self.tools.cap(&format!("{head}\n{text}")))
            }
            Err(e) => (false, format!("fetch failed: {e}")),
        }
    }

    /// Whether a write or edit of `path` may go ahead. The error is the
    /// tool result, with `{what}` for the tool's name.
    fn allowed_write(&mut self, path: &str, hooks: &mut dyn Hooks) -> Result<(), String> {
        let declined = || Err("refused: the user declined this {what}".to_string());
        // The jail refuses before anyone is asked.
        let resolved = self.tools.resolve(path).map_err(|e| format!("refused: {e}"))?;
        match crate::perm::self_config(&resolved, self.cfg.live_permissions()) {
            Some(crate::perm::Decision::Deny(why)) => {
                hooks.ui(Ui::Error(why.clone()));
                return Err(why);
            }
            Some(crate::perm::Decision::Ask) => {
                if !hooks.confirm(&format!("write {path}")) {
                    return declined();
                }
            }
            _ => {}
        }
        let root = self.tools.root.canonicalize().unwrap_or_else(|_| self.tools.root.clone());
        let scratch = crate::tools::scratch().and_then(|d| d.canonicalize().ok());
        let inside = resolved.starts_with(&root)
            || resolved.starts_with(&self.tools.root)
            || scratch.is_some_and(|d| resolved.starts_with(d));
        match perm::classify_write(inside, self.cfg.live_permissions()) {
            Decision::Allow => Ok(()),
            Decision::Ask if hooks.confirm(&format!("write {path}")) => Ok(()),
            Decision::Ask => declined(),
            Decision::Deny(why) => Err(why),
        }
    }

}

/// How a goal round's reply must end, so imp knows whether to go on.
const GOAL_ENDINGS: &str = "End your reply with a last line of GOAL MET and the evidence when the goal is met, \
or GOAL BLOCKED: <why> when you need the user. Otherwise do the next part of the work.";

/// A suggestion as the user would send it: one short line, quotes stripped.
fn clean_suggestion(text: &str) -> Option<String> {
    let t = text.trim();
    let t = ['"', '\'', '`', '“'].iter().fold(t, |t, q| t.strip_prefix(*q).unwrap_or(t));
    let t = ['"', '\'', '`', '”'].iter().fold(t, |t, q| t.strip_suffix(*q).unwrap_or(t)).trim();
    if t.is_empty() || t.contains('\n') || t.chars().count() > 60 {
        return None;
    }
    // ponytail: a question-word check; questions that open otherwise rely on the prompt.
    const ASKS: &[&str] = &[
        "what", "why", "how", "where", "when", "which", "who", "can", "could", "should", "would", "will",
        "is", "are", "do", "does", "did", "shall",
    ];
    let first = t.split_whitespace().next().unwrap_or("").to_lowercase();
    if ASKS.contains(&first.as_str()) && !t.ends_with('?') {
        return Some(format!("{}?", t.trim_end_matches('.')));
    }
    Some(t.to_string())
}

/// Index to keep from: the fourth-newest user message. `None` if too short.
pub fn keep_index(history: &[Message]) -> Option<usize> {
    let users: Vec<usize> = history
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == crate::api::Role::User)
        .map(|(i, _)| i)
        .collect();
    if users.len() < 5 {
        return None;
    }
    Some(users[users.len() - 4])
}

/// A 400 that means the request was too long, not that it was malformed.
pub fn is_context_overflow(status: u16, body: &str) -> bool {
    if status != 400 {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("context_length") || lower.contains("maximum context")
}

/// Durable facts the summariser was asked to emit.
pub fn facts_from_summary(summary: &str) -> Vec<String> {
    summary
        .lines()
        .filter_map(|l| l.trim().strip_prefix("MEMORY:").map(|s| s.trim().to_string()))
        .filter(|s| s.len() > 8)
        .take(3)
        .collect()
}

/// Feature tools ship only when the feature exists.
pub fn include_tool(name: &str, has_memory: bool, has_skills: bool) -> bool {
    match name {
        "memory" => has_memory,
        "skill_load" => has_skills,
        _ => true,
    }
}

pub fn summarise(text: &str) -> String {
    let flat = text.replace('\n', " ");
    let mut s: String = flat.chars().take(120).collect();
    if flat.chars().count() > 120 {
        s.push('…');
    }
    s
}

/// A URL as host and path: the query is noise in a step row.
fn short_url(url: &str) -> String {
    let url = flatten(url);
    let rest = url.split_once("://").map_or(url.as_str(), |(_, r)| r);
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let path = path.trim_end_matches('/');
    let mut out: String = path.chars().take(60).collect();
    if path.chars().count() > 60 || rest.len() > path.len() + 1 {
        out.push('…');
    }
    out
}

/// What a successful step brought back, in a few words.
pub fn gist(name: &str, out: &str) -> String {
    let size = |n: usize| {
        if n >= 1024 {
            format!("{} KB", n.div_ceil(1024))
        } else {
            format!("{n} B")
        }
    };
    let first = out.lines().next().unwrap_or("").trim();
    match name {
        "fetch" => {
            // `200 text/html` then the body.
            let (status, kind) = first.split_once(' ').unwrap_or((first, ""));
            let kind = kind.rsplit('/').next().unwrap_or(kind);
            // A capped body names its full size in the cap note.
            let body = out
                .split_once("... [")
                .and_then(|(_, n)| n.split(' ').next()?.parse().ok())
                .unwrap_or(out.len().saturating_sub(first.len() + 1));
            let items = out.lines().filter(|l| l.starts_with("- ")).count();
            let what = if (kind.contains("xml") || kind.contains("rss") || kind.contains("atom")) && items > 0 {
                format!("{items} items")
            } else {
                size(body)
            };
            format!("{status} {kind} · {what}")
        }
        "search" => {
            let n = out.split("\n\n").filter(|b| b.contains("://")).count();
            if n == 0 { "no results".into() } else { format!("{n} results") }
        }
        "read" => format!("{} lines", out.lines().count()),
        _ => summarise(first).chars().take(60).collect(),
    }
}

/// The whole text on one line. The renderer wraps it.
fn flatten(text: &str) -> String {
    text.replace('\n', " ").trim().to_string()
}

/// A tool row with the session directory left off its paths and the
/// scratch folder named `$IMP_TMP`.
pub fn relative(summary: String, root: &std::path::Path) -> String {
    let mut s = summary;
    if let Some(t) = crate::tools::scratch() {
        s = s.replace(&t.display().to_string(), "$IMP_TMP");
    }
    let root = root.display().to_string();
    if root.len() > 1 {
        s = s.replace(&format!("{}/", root.trim_end_matches('/')), "");
    }
    s
}

/// One short line for a tool call, in place of its JSON arguments.
pub fn describe(name: &str, args: &Value) -> String {
    let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or("");
    let lines = |key: &str| text(key).lines().count();
    match name {
        // A label reads better than a long pipeline; the command stays in Ctrl+E.
        "bash" if !text("description").trim().is_empty() => flatten(text("description")),
        "bash" => format!("run `{}`", flatten(text("command"))),
        "job" => match args.get("id").and_then(Value::as_u64) {
            Some(id) => format!("job {} {id}", text("op")),
            None => format!("job {}", text("op")),
        },
        "read" => {
            let paths: Vec<&str> = args
                .get("paths")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            format!("read {}", flatten(&paths.join(", ")))
        }
        "edit" => {
            let edits = args
                .get("edits")
                .and_then(Value::as_array)
                .map(|a| a.len())
                .unwrap_or(0);
            format!("edit {} ({edits} change{})", flatten(text("path")), if edits == 1 { "" } else { "s" })
        }
        "write" => format!(
            "write {} ({} lines)",
            flatten(text("path")),
            lines("content")
        ),
        "fetch" => match text("method").to_ascii_uppercase().as_str() {
            "" | "GET" => format!("fetch {}", short_url(text("url"))),
            method => format!("{method} {}", short_url(text("url"))),
        },
        "search" => format!("search {:?}", flatten(text("query"))),
        "memory" => {
            let what = if text("title").is_empty() {
                text("query")
            } else {
                text("title")
            };
            format!("memory {} {:?}", text("op"), flatten(what))
        }
        "skill_load" => format!("load skill {}", flatten(text("name"))),
        "ask" => format!("ask {:?}", flatten(text("question"))),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod subagent_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Agent, Log};

    #[test]
    fn steps_summarise_what_came_back() {
        assert_eq!(gist("fetch", &format!("200 application/json\n{}", "x".repeat(3000))), "200 json · 3 KB");
        assert_eq!(gist("fetch", "200 text/xml\n# BBC\n\n- a · l\n- b · l"), "200 xml · 2 items");
        assert_eq!(gist("fetch", "200 text/html\n# Hi"), "200 html · 4 B");
        assert_eq!(gist("fetch", "200 application/json\n{...\n... [19203 bytes total; x]\n}"), "200 json · 19 KB");
        assert_eq!(gist("search", "T\nhttps://a\ns\n\nU\nhttps://b\nt\n\n"), "2 results");
        assert_eq!(gist("bash", "exit 0\nhello"), "exit 0");
        assert_eq!(short_url("https://api.open-meteo.com/v1/forecast?latitude=51.3&x=1"), "api.open-meteo.com/v1/forecast…");
        assert_eq!(short_url("https://wttr.in/Woking"), "wttr.in/Woking");
        assert_eq!(describe("fetch", &serde_json::json!({"url": "https://wttr.in/"})), "fetch wttr.in");
    }

    #[test]
    fn tool_rows_drop_the_session_directory() {
        let root = std::path::Path::new("/home/m/shop");
        assert_eq!(relative("read /home/m/shop/src/cart.py, /etc/hosts".into(), root), "read src/cart.py, /etc/hosts");
        assert_eq!(relative("run `cd /home/m/shop`".into(), root), "run `cd /home/m/shop`");
    }

    fn tmp() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "imp-agent-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn roster() -> Vec<Agent> {
        let mut reviewer = Agent::builtin();
        reviewer.name = "reviewer".into();
        reviewer.description = "reviews".into();
        vec![Agent::builtin(), reviewer]
    }

    struct Null;
    impl Hooks for Null {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            false
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    /// Answers every confirmation with a fixed choice, for the goal tool.
    struct Answer2(bool);
    impl Hooks for Answer2 {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            self.0
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    #[test]
    fn a_goal_the_user_confirms_starts_round_one() {
        let mut s = session(crate::config::Config::default());
        let (ok, _) = s.run_tool("goal", &json!({"operation": "set", "text": "ship the parser"}), &mut Answer2(true));
        assert!(ok, "the confirmed goal was refused");
        let goal = s.goal.as_ref().expect("no goal was set");
        assert_eq!((goal.text.as_str(), goal.round), ("ship the parser", 1));
    }

    #[test]
    fn a_goal_the_user_declines_is_dropped_and_the_agent_is_told() {
        let mut s = session(crate::config::Config::default());
        let (ok, out) = s.run_tool("goal", &json!({"operation": "set", "text": "ship"}), &mut Answer2(false));
        assert!(!ok, "a declined goal reported success");
        assert!(s.goal.is_none(), "a declined goal was kept");
        assert!(out.contains("declined"), "the agent was not told: {out}");
    }

    #[test]
    fn the_agent_cannot_start_rounds_without_a_confirmed_goal() {
        let mut s = session(crate::config::Config::default());
        let _ = s.run_tool("goal", &json!({"operation": "set", "text": "ship"}), &mut Answer2(false));
        assert!(s.goal_next().is_err(), "a round started for a declined goal");
    }

    #[test]
    fn a_goal_is_saved_with_the_session_and_restored_paused() {
        let d = tmp();
        let log = crate::store::Log::create(&d, &d, "deepseek-flash");
        // A started session has a transcript file; `Log::open` needs one.
        std::fs::write(d.join(format!("{}.jsonl", log.id)), "").unwrap();
        let mut s = session(crate::config::Config::default());
        s.attach_log(log.clone());
        s.start_goal("ship the parser");
        s.save_goal();
        let mut back = session(crate::config::Config::default());
        back.resume_id(&d, &log.id).expect("resume");
        let goal = back.goal.as_ref().expect("the goal was not restored");
        assert_eq!((goal.text.as_str(), goal.round), ("ship the parser", 1));
        assert!(goal.paused, "a restored goal ran unasked");
    }

    fn session(cfg: crate::config::Config) -> Session {
        let d = tmp();
        Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        )
    }

    #[test]
    fn retry_cuts_the_history_at_the_typed_message() {
        let mut s = session(crate::config::Config::default());
        assert!(!s.retry("first"), "retried with nothing sent");
        let log = crate::store::Log::create(&tmp(), &s.tools.root, "deepseek-flash");
        s.attach_log(log.clone());
        s.say(Message::user("first"));
        s.say(Message::assistant("one"));
        s.say(Message::user("fix it"));
        s.say(Message::assistant("round one"));
        // A goal round's nudge is a user message the chat never shows.
        s.say(Message::user("keep going: round 2"));
        s.say(Message::assistant("a bad answer"));
        assert!(s.retry("fix it"));
        let kept: Vec<&str> = s.history.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(kept, vec!["first", "one"], "the cut missed the typed message");
        // A resumed session drops the same messages.
        let loaded: Vec<String> = log.load().into_iter().map(|m| m.content).collect();
        assert_eq!(loaded, vec!["first", "one"]);
        assert!(!s.retry("never sent"), "retried a message that is not there");
    }

    #[test]
    fn turn_carries_attachments_and_caps_them() {
        let (mut s, _fake) = scripted(vec![Script::Ok(text_reply("done"))]);
        for i in 0..crate::images::MAX_PER_MESSAGE {
            s.attach_image(Image {
                file: format!("{i}.png"),
                name: format!("{i}.png"),
                mime: "image/png".into(),
                bytes: 8,
            })
            .unwrap();
        }
        assert!(s
            .attach_image(Image {
                file: "extra.png".into(),
                name: "extra.png".into(),
                mime: "image/png".into(),
                bytes: 8,
            })
            .is_err());

        let mut hooks = Null;
        s.turn("look", &mut hooks).unwrap();
        let user = s
            .history
            .iter()
            .find(|m| m.role == crate::api::Role::User && m.content == "look")
            .unwrap();
        assert_eq!(user.images.len(), crate::images::MAX_PER_MESSAGE);
    }

    #[test]
    fn max_steps_pauses_after_the_last_batch() {
        let (mut s, _fake) = scripted(vec![
            Script::Ok(tool_reply("c1", "bash", "{\"command\":\"true\"}")),
            Script::Ok(tool_reply("c2", "bash", "{\"command\":\"true\"}")),
            Script::Ok(text_reply("never reached")),
        ]);
        s.cfg.max_steps = 2;
        let mut hooks = Null;
        assert!(s.turn("go", &mut hooks).is_ok());
        let calls = s
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::Assistant && !m.tool_calls.is_empty())
            .count();
        let results = s
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::Tool)
            .count();
        assert_eq!(calls, 2);
        assert_eq!(results, 2, "every call needs a result");
    }

    #[test]
    fn fork_switches_to_the_copy() {
        let mut s = session(crate::config::Config::default());
        let log = crate::store::Log::create(&tmp(), &s.tools.root, "deepseek-flash");
        s.attach_log(log);
        let before = s.log.as_ref().unwrap().id.clone();
        let (new_id, old_id) = s.fork().unwrap();
        assert_eq!(old_id, before);
        assert_eq!(s.log.as_ref().unwrap().id, new_id);
        assert_ne!(new_id, old_id);
    }

    #[test]
    fn in_dir_survives_a_quote_in_the_path() {
        let dir = std::env::temp_dir().join("imp it's here");
        std::fs::create_dir_all(&dir).unwrap();
        let out = std::process::Command::new("sh").arg("-c").arg(in_dir(&dir, "pwd")).output().unwrap();
        let got = String::from_utf8_lossy(&out.stdout);
        assert_eq!(std::fs::canonicalize(got.trim()).unwrap(), std::fs::canonicalize(&dir).unwrap());
    }

    #[test]
    fn edit_detail_shows_removed_and_added_lines() {
        let args = json!({"path": "x.rs", "edits": [{"old": "a\nb", "new": "c"}]});
        let detail = diff_detail("edit", &args);
        assert!(detail.contains("-a"), "{detail}");
        assert!(detail.contains("-b"), "{detail}");
        assert!(detail.contains("+c"), "{detail}");
        let trimmed = diff_detail("edit", &json!({"edits": [{"old": "a\nb\nc", "new": "a\nx\nc"}]}));
        assert_eq!(trimmed.trim_end(), " a\n-b\n+x\n c");
        let write = diff_detail("write", &json!({"path": "x", "content": "one\ntwo"}));
        assert_eq!(write, "+one\n+two");
        assert!(diff_detail("read", &json!({})).is_empty());
    }

    #[test]
    fn long_tool_descriptions_are_kept_whole() {
        let command = format!("echo {}", "word ".repeat(80));
        let text = describe("bash", &json!({"command": command}));
        assert!(text.chars().count() > 120, "{}", text.chars().count());
        assert!(!text.ends_with('…'), "{text}");

        let question = "why ".repeat(80);
        let ask = describe("ask", &json!({"question": question}));
        assert!(ask.chars().count() > 120);
        assert!(!ask.ends_with('…'), "{ask}");
    }

    #[test]
    fn describe_agent_names_model_thinking_and_tools() {
        let mut agent = Agent::builtin();
        agent.name = "reviewer".into();
        agent.tools = Some(vec!["read".into(), "bash".into()]);
        let text = describe_agent(&agent, &Config::default());
        assert!(text.contains("agent: reviewer"), "{text}");
        assert!(text.contains("model deepseek-flash"), "{text}");
        assert!(text.contains("thinking off"), "{text}");
        assert!(text.contains("tools read, bash"), "{text}");
    }

    #[test]
    fn set_root_reloads_and_records_the_directory() {
        let mut s = session(crate::config::Config::default());
        let dir = tmp();
        std::fs::write(
            dir.join("AGENTS.md"),
            "instructions from the new root",
        )
        .unwrap();
        let outside = tmp();
        std::fs::write(outside.join("secret.txt"), "x").unwrap();
        let log = crate::store::Log::create(&tmp(), &s.tools.root, "deepseek-flash");
        s.attach_log(log);

        s.set_root(dir.clone()).unwrap();

        assert_eq!(s.tools.root, dir);
        assert!(
            s.history
                .first()
                .map(|m| m.content.contains("instructions from the new root"))
                .unwrap_or(false),
            "prompt did not reload"
        );
        assert_eq!(s.log.as_ref().unwrap().meta_cwd().unwrap(), dir);
        let outside = outside.join("secret.txt").display().to_string();
        let out = s.tools.read(&[outside], None, None, None);
        assert!(!out.ok, "read escaped the new root: {}", out.text);
    }

    use crate::transport::{text_reply, tool_reply, Fake, Script};
    use std::sync::Arc;

    fn scripted(script: Vec<Script>) -> (Session, Arc<Fake>) {
        let fake = Arc::new(Fake::new(script));
        let mut s = session(crate::config::Config::default());
        s.transport = Box::new(fake.clone());
        s.retry_base = std::time::Duration::from_millis(1);
        (s, fake)
    }

    struct CancelOnTool(std::sync::Arc<AtomicBool>);
    impl Hooks for CancelOnTool {
        fn ui(&mut self, event: Ui) {
            if matches!(event, Ui::ToolStart { .. }) {
                self.0.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        fn confirm(&mut self, _: &str) -> bool {
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    #[test]
    fn tool_summaries_name_the_work() {
        let cases = [
            ("bash", json!({"command": "ls -la"}), "run `ls -la`"),
            (
                "bash",
                json!({"command": "find . | wc -l", "description": "count files"}),
                "count files",
            ),
            ("bash", json!({"command": "ls", "description": " "}), "run `ls`"),
            ("read", json!({"paths": ["a.rs", "b.rs"]}), "read a.rs, b.rs"),
            (
                "write",
                json!({"path": "x/s.md", "content": "a\nb\nc"}),
                "write x/s.md (3 lines)",
            ),
            (
                "edit",
                json!({"path": "a.rs", "edits": [{"old": "a", "new": "b"}]}),
                "edit a.rs (1 change)",
            ),
            ("fetch", json!({"url": "https://x/y.md"}), "fetch x/y.md"),
            ("search", json!({"op": "search", "query": "woking"}), "search \"woking\""),
            ("skill_load", json!({"name": "config"}), "load skill config"),
            ("ask", json!({"question": "where?"}), "ask \"where?\""),
        ];
        for (name, args, want) in cases {
            assert_eq!(describe(name, &args), want);
        }
    }

    #[test]
    fn a_disabled_skill_is_refused_by_name() {
        let (mut s, _fake) = scripted(vec![]);
        struct Quiet;
        impl Hooks for Quiet {
            fn ui(&mut self, _: Ui) {}
            fn confirm(&mut self, _: &str) -> bool {
                true
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
        }
        s.cfg.set("skill.config.enabled", "false").unwrap();
        s.skills = crate::store::load_skills(&s.cfg, &s.tools.root);
        let (ok, out) = s.run_tool("skill_load", &json!({"name": "config"}), &mut Quiet);
        assert!(!ok);
        assert_eq!(out, "refused: skill \"config\" is disabled");
    }

    #[test]
    fn a_refused_write_is_a_failure_with_a_reason() {
        let (mut s, _fake) = scripted(vec![]);
        struct Yes;
        impl Hooks for Yes {
            fn ui(&mut self, _: Ui) {}
            fn confirm(&mut self, _: &str) -> bool {
                true
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
        }
        let mut hooks = Yes;
        let (ok, out) = s.run_tool(
            "write",
            &serde_json::json!({"path": "/tmp/imp-outside.txt", "content": "x"}),
            &mut hooks,
        );
        assert!(!ok, "a refused write reported success: {out}");
        assert!(
            out.contains("outside the session directory"),
            "no reason given: {out}"
        );
    }

    #[test]
    fn the_width_is_sent_once_per_change() {
        let (mut s, _fake) = scripted(vec![
            Script::Ok(text_reply("one")),
            Script::Ok(text_reply("two")),
            Script::Ok(text_reply("three")),
        ]);
        crate::tui::SCREEN.store(80 << 16 | 24, std::sync::atomic::Ordering::Relaxed);
        s.cols.store(60, std::sync::atomic::Ordering::Relaxed);
        s.turn("a", &mut Null).unwrap();
        s.turn("b", &mut Null).unwrap();
        s.cols.store(100, std::sync::atomic::Ordering::Relaxed);
        s.turn("c", &mut Null).unwrap();
        let notes: Vec<&str> = s
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::System && m.content.contains("columns wide"))
            .map(|m| m.content.as_str())
            .collect();
        assert_eq!(notes.len(), 2, "notes: {notes:?}");
        assert!(notes[0].contains("60 columns wide and 24 rows high"), "first: {}", notes[0]);
        assert!(notes[1].contains("100"), "second: {}", notes[1]);
    }

    #[test]
    fn the_width_travels_with_the_session() {
        let d = tmp();
        let cols = std::sync::Arc::new(std::sync::atomic::AtomicU16::new(60));
        let s = Session::new(
            crate::config::Config::default(),
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            cols.clone(),
        );
        assert_eq!(
            s.cols.load(std::sync::atomic::Ordering::Relaxed),
            60,
            "the session did not see the width"
        );
        cols.store(0, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(s.cols.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[test]
    fn the_confirmation_wait_is_not_charged_to_the_tool() {
        let (mut s, _fake) = scripted(vec![
            Script::Ok(tool_reply("c1", "bash", "{\"command\":\"true\"}")),
            Script::Ok(text_reply("done")),
        ]);
        s.cfg.set_permissions(crate::config::Permissions::Strict);

        struct Slow {
            waited: std::time::Duration,
            ms: Option<u128>,
        }
        impl Hooks for Slow {
            fn ui(&mut self, event: Ui) {
                if let Ui::ToolEnd { ms, .. } = event {
                    self.ms = Some(ms);
                }
            }
            fn confirm(&mut self, _: &str) -> bool {
                let opened = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_millis(80));
                self.waited = opened.elapsed();
                true
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
            fn last_wait(&self) -> std::time::Duration {
                self.waited
            }
        }

        let mut hooks = Slow {
            waited: std::time::Duration::ZERO,
            ms: None,
        };
        s.turn("run true", &mut hooks).unwrap();
        let ms = hooks.ms.expect("a tool end");
        assert!(ms < 50, "the wait was charged to the tool: {ms} ms");
    }

    #[test]
    fn cancel_during_a_tool_call_stops_it() {
        let (mut s, _fake) = scripted(vec![
            Script::Ok(tool_reply("c1", "bash", "{\"command\":\"sleep 5\"}")),
            Script::Ok(text_reply("never reached")),
        ]);
        let flag = s.cancel.clone();
        let mut hooks = CancelOnTool(flag);
        s.turn("run it", &mut hooks).unwrap();
        let tool = s
            .history
            .iter()
            .find(|m| m.role == crate::api::Role::Tool)
            .expect("a tool result")
            .content
            .clone();
        assert!(tool.contains("cancelled"), "{tool}");
    }

    #[test]
    fn secrets_are_redacted_and_naming_one_asks() {
        use crate::secrets::{TEST_NAME, TEST_VALUE};
        let (mut s, _fake) = scripted(vec![
            Script::Ok(tool_reply("c1", "bash", &format!("{{\"command\":\"echo ${TEST_NAME}\"}}"))),
            Script::Ok(tool_reply("c2", "bash", &format!("{{\"command\":\"printf {TEST_VALUE}\"}}"))),
            Script::Ok(text_reply("done")),
        ]);
        s.turn("go", &mut Null).unwrap();
        let tools: Vec<&str> = s
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::Tool)
            .map(|m| m.content.as_str())
            .collect();
        assert!(tools[0].contains("declined"), "naming a secret ran unasked: {}", tools[0]);
        assert!(tools[1].contains(&format!("[redacted {TEST_NAME}]")), "{}", tools[1]);
        assert!(!s.history.iter().any(|m| m.content.contains(TEST_VALUE)), "the value reached history");
    }

    #[test]
    fn check_ins_come_due_with_the_jobs_state() {
        let (mut s, _fake) = scripted(vec![]);
        let (ok, out) = s.run_tool("job", &json!({"op": "check", "after_s": 60}), &mut Null);
        assert!(!ok && out.contains("need the chat"), "line mode accepted a check-in: {out}");
        s.wakes = true;
        s.cfg.set_permissions(Permissions::Yolo);
        let (ok, out) = s.run_tool("bash", &json!({"command": "printf 'copied 3 of 10'; sleep 5", "background": true}), &mut Null);
        assert!(ok, "{out}");
        let id: u64 = out.split_whitespace().nth(1).unwrap().parse().unwrap();
        let (ok, out) = s.run_tool("job", &json!({"op": "check", "after_s": 10}), &mut Null);
        assert!(!ok && out.contains("30 to 86400"), "{out}");
        let (ok, out) = s.run_tool("job", &json!({"op": "check", "id": id, "after_s": 600}), &mut Null);
        assert!(ok && out.contains("10m 0s"), "{out}");
        assert!(s.due().is_none(), "a check-in fired early");
        std::thread::sleep(std::time::Duration::from_millis(300));
        s.checkins[0].0 = std::time::Instant::now();
        let note = s.due().expect("no note when due");
        assert!(note.starts_with("[Check-in from imp, not the user.]"), "{note}");
        assert!(note.contains(&format!("job {id}")) && note.contains("last: copied 3 of 10"), "{note}");
        assert!(s.due().is_none(), "a check-in fired twice");
        for _ in 0..MAX_CHECKINS {
            assert!(s.run_tool("job", &json!({"op": "check", "after_s": 60}), &mut Null).0);
        }
        let (ok, out) = s.run_tool("job", &json!({"op": "check", "after_s": 60}), &mut Null);
        assert!(!ok && out.contains("already pending"), "{out}");
        let _ = s.jobs.kill(id);
    }

    #[test]
    fn a_notified_job_wakes_the_session_when_it_ends() {
        let (mut s, _fake) = scripted(vec![]);
        s.wakes = true;
        let (ok, out) = s.run_tool("bash", &json!({"command": "echo all 120 files copied", "background": true, "notify": true}), &mut Null);
        assert!(ok, "{out}");
        let quiet = s.run_tool("bash", &json!({"command": "true", "background": true}), &mut Null);
        assert!(quiet.0);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let note = s.due().expect("the end did not wake");
        assert!(note.contains("ended with exit 0") && note.contains("all 120 files copied"), "{note}");
        assert_eq!(note.matches("ended with exit").count(), 1, "the unnotified job woke too: {note}");
        assert!(s.history.iter().any(|m| m.role == crate::api::Role::System && m.content.contains("finished with exit 0")));
    }

    #[test]
    fn shown_output_reaches_the_user_and_text_reaches_the_model() {
        struct Shown(Vec<String>);
        impl Hooks for Shown {
            fn ui(&mut self, event: Ui) {
                if let Ui::Show(t) = event {
                    self.0.push(t);
                }
            }
            fn confirm(&mut self, _: &str) -> bool {
                false
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
        }
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Yolo);
        let mut hooks = Shown(Vec::new());
        let (ok, out) = s.run_tool("bash", &json!({"command": "printf '\\033[31mred\\033[0m chart\\n'", "show": true}), &mut hooks);
        assert!(ok, "{out}");
        assert_eq!(hooks.0, ["\u{1b}[31mred\u{1b}[0m chart\n"]);
        assert_eq!(out, "exit 0\nred chart\n");
        let (_, out) = s.run_tool("bash", &json!({"command": "printf '\\033[32m▀▀▀▀\\n▄▄▄▄\\033[0m\\n'", "show": true}), &mut hooks);
        assert_eq!(out, "exit 0\n[shown to the user: a 4×2 picture]");
        let (_, out) = s.run_tool("bash", &json!({"command": "printf '\\033[31mx\\033[0m'"}), &mut hooks);
        assert_eq!(hooks.0.len(), 2, "output was shown without show");
        assert!(out.contains("\u{1b}[31m"), "without show the model gets the output as before: {out:?}");
    }

    #[test]
    fn the_date_is_noted_once_a_day() {
        let (mut s, _fake) = scripted(vec![]);
        let notes = |s: &Session| s.history.iter().filter(|m| m.content.starts_with("Today is ")).count();
        s.note_date(20_778);
        s.note_date(20_778);
        assert_eq!(notes(&s), 1, "the same day was noted twice");
        assert!(s.history.iter().any(|m| m.content == "Today is Saturday 21 November 2026."));
        s.note_date(20_779);
        assert_eq!(notes(&s), 2, "a new day was not noted");
        assert!(s.history.iter().any(|m| m.content == "Today is Sunday 22 November 2026."));
    }

    #[test]
    fn a_cancelled_request_is_not_retried() {
        let (mut s, fake) = scripted(vec![Script::Cancel, Script::Ok(text_reply("retried"))]);
        struct Notes(Vec<String>);
        impl Hooks for Notes {
            fn ui(&mut self, event: Ui) {
                if let Ui::Info(t) | Ui::Notice(t) | Ui::Error(t) = event {
                    self.0.push(t);
                }
            }
            fn confirm(&mut self, _: &str) -> bool {
                false
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
        }
        let mut hooks = Notes(Vec::new());
        s.turn("hello", &mut hooks).unwrap();
        assert_eq!(fake.attempts(), 1, "the cancelled request was sent again");
        assert_eq!(hooks.0, vec!["cancelled".to_string()]);
        assert!(!s.history.iter().any(|m| m.content == "retried"));
    }

    #[test]
    fn a_cancelled_turn_stops_before_the_next_request() {
        let (mut s, fake) = scripted(vec![
            Script::Ok(tool_reply("c1", "bash", "{\"command\":\"true\"}")),
            Script::Ok(text_reply("never reached")),
        ]);
        struct CancelAfterFirstStep(std::sync::Arc<AtomicBool>, usize);
        impl Hooks for CancelAfterFirstStep {
            fn ui(&mut self, event: Ui) {
                if matches!(event, Ui::ToolEnd { .. }) {
                    self.1 += 1;
                    if self.1 == 1 {
                        self.0.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
            fn confirm(&mut self, _: &str) -> bool {
                true
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
        }
        let mut hooks = CancelAfterFirstStep(s.cancel.clone(), 0);
        s.turn("run it", &mut hooks).unwrap();
        assert_eq!(fake.attempts(), 1, "the cancelled turn sent another request");
        assert!(
            !s.history.iter().any(|m| m.content.contains("never reached")),
            "the cancelled turn kept the second answer"
        );
    }

    #[test]
    fn prior_reasoning_is_sent_back() {
        let mut history = vec![Message::user("hi")];
        let mut assistant = Message::assistant("hello");
        assistant.reasoning = "because".into();
        history.push(assistant);
        let opts = RequestOptions {
            model: "deepseek-flash".into(),
            max_tokens: 10,
            thinking: true,
            temperature: None,
            effort: None,
        };
        let mut adapter = crate::providers::adapter(
            crate::providers::Provider::DeepSeek,
            "https://api.deepseek.com",
            "k",
        );
        let body = adapter.wire(&opts, &history, &[]).body;
        assert_eq!(body["messages"][1]["reasoning_content"], "because");
    }

    #[test]
    fn a_restricted_agent_loses_tools() {
        let mut reviewer = Agent::builtin();
        reviewer.name = "reviewer".into();
        reviewer.tools = Some(vec!["read".into()]);
        let mut s = session(crate::config::Config::default());
        s.apply_agent(reviewer);
        let names: Vec<String> = s
            .active_tools()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap_or("").to_string())
            .collect();
        assert!(names.contains(&"read".to_string()));
        assert!(!names.contains(&"bash".to_string()));
    }

    #[test]
    fn a_provider_without_search_omits_the_tool() {
        let mut cfg = crate::config::Config::default();
        cfg.set_provider(crate::providers::Provider::OpenAi);
        let mut s = session(cfg);
        let names: Vec<String> = s
            .active_tools()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap_or("").to_string())
            .collect();
        assert!(!names.contains(&"search".to_string()), "{names:?}");
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"fetch".to_string()), "fetch must work on every provider");

        let (ok, out) = s.run_tool("search", &json!({"op": "search", "query": "x"}), &mut Null);
        assert!(!ok, "{out}");
        assert!(out.contains("not available"), "{out}");
    }

    #[test]
    fn retries_once_then_succeeds() {
        let (mut s, fake) = scripted(vec![
            Script::Fails(429, "slow down".into()),
            Script::Ok(text_reply("done")),
        ]);
        s.turn("hi", &mut Null).unwrap();
        assert_eq!(fake.attempts(), 2);
        let seen: Vec<String> = s
            .history
            .iter()
            .map(|m| format!("{}:{:?}", m.role.as_str(), m.content))
            .collect();
        assert!(
            s.history.iter().any(|m| m.content == "done"),
            "{}",
            seen.join(" | ")
        );
    }

    /// Collects retry countdowns.
    struct Waits(Vec<(u64, u32)>);
    impl Hooks for Waits {
        fn ui(&mut self, event: Ui) {
            if let Ui::Retry { secs, attempt, .. } = event {
                self.0.push((secs, attempt));
            }
        }
        fn confirm(&mut self, _: &str) -> bool {
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    #[test]
    fn retries_back_off_until_success_or_five() {
        let (mut s, fake) = scripted(vec![
            Script::Fails(429, "slow".into()),
            Script::Transport("connection reset".into()),
            Script::Fails(503, "busy".into()),
            Script::Ok(text_reply("done")),
        ]);
        let mut waits = Waits(Vec::new());
        s.turn("hi", &mut waits).unwrap();
        assert_eq!(fake.attempts(), 4);
        let attempts: Vec<u32> = waits.0.iter().filter(|(secs, _)| *secs == 0).map(|(_, a)| *a).collect();
        assert_eq!(attempts, vec![1, 2, 3], "one wait per retry");
        let (mut s, fake) = scripted((0..7).map(|_| Script::Fails(500, "down".into())).collect());
        assert!(s.turn("hi", &mut Null).is_err());
        assert_eq!(fake.attempts(), 6, "the first try and five retries");
    }

    #[test]
    fn retry_after_sets_the_wait() {
        let (mut s, _fake) = scripted(vec![Script::Wait(429, 1), Script::Ok(text_reply("done"))]);
        let mut waits = Waits(Vec::new());
        let start = std::time::Instant::now();
        s.turn("hi", &mut waits).unwrap();
        assert!(start.elapsed() >= std::time::Duration::from_millis(900), "Retry-After was ignored");
        assert_eq!(waits.0.first(), Some(&(1, 1)));
    }

    #[test]
    fn esc_ends_a_wait() {
        let (mut s, fake) = scripted(vec![Script::Wait(429, 30), Script::Ok(text_reply("done"))]);
        let cancel = s.cancel.clone();
        let stop = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let start = std::time::Instant::now();
        s.turn("hi", &mut Null).unwrap();
        stop.join().unwrap();
        assert!(start.elapsed() < std::time::Duration::from_secs(2), "the wait outlived Esc");
        assert_eq!(fake.attempts(), 1, "a request was sent after Esc");
    }

    #[test]
    fn terminal_error_does_not_retry() {
        let (mut s, fake) = scripted(vec![Script::Fails(400, "bad request".into())]);
        let err = s.turn("hi", &mut Null).unwrap_err();
        assert_eq!(fake.attempts(), 1);
        assert!(err.contains("400"), "{err}");
    }

    #[test]
    fn context_overflow_truncates_and_retries() {
        let (mut s, fake) = scripted(vec![
            Script::Fails(
                400,
                "This model's maximum context length is 65536 tokens".into(),
            ),
            Script::Ok(text_reply("ok")),
        ]);
        for _ in 0..10 {
            s.history.push(Message::tool("c", "y".repeat(4000)));
        }
        s.turn("hi", &mut Null).unwrap();
        assert_eq!(fake.attempts(), 2);
        assert!(s.history.iter().any(|m| m.content.contains("[dropped")));
    }


    #[derive(Default)]
    struct Counting(usize);
    impl Hooks for Counting {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            self.0 += 1;
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    #[test]
    fn a_goal_runs_until_met_blocked_or_out_of_rounds() {
        let (mut s, _fake) = scripted(vec![
            Script::Ok(text_reply("Fixed one test; two remain.")),
            Script::Ok(text_reply("I will write GOAL MET when done.\nStill working.")),
            Script::Ok(text_reply("All pass.\n**GOAL MET** 13 tests OK")),
        ]);
        s.cfg.goal_rounds = 5;
        let first = s.start_goal("all tests pass");
        assert!(first.contains("all tests pass") && first.contains("GOAL MET"));
        s.turn(&first, &mut Null).unwrap();
        let next = s.goal_next().unwrap();
        assert!(next.contains("Round 2 of 5"), "{next}");
        s.turn(&next, &mut Null).unwrap();
        let next = s.goal_next().expect("a mention mid-reply ended the goal");
        s.turn(&next, &mut Null).unwrap();
        assert_eq!(s.goal_next(), Err("goal met".into()));
        assert!(s.goal.is_none());

        let (mut s, _fake) = scripted(vec![Script::Ok(text_reply("GOAL BLOCKED: needs the API key"))]);
        let first = s.start_goal("deploy");
        s.turn(&first, &mut Null).unwrap();
        assert_eq!(s.goal_next(), Err("goal blocked: needs the API key".into()));

        let (mut s, _fake) = scripted(vec![Script::Ok(text_reply("a")), Script::Ok(text_reply("b"))]);
        s.cfg.goal_rounds = 2;
        let first = s.start_goal("x");
        s.turn(&first, &mut Null).unwrap();
        let next = s.goal_next().unwrap();
        s.turn(&next, &mut Null).unwrap();
        assert_eq!(s.goal_next(), Err("goal not met after 2 rounds".into()));

        let args = json!({"command": "npm publish"}).to_string();
        let (mut s, _fake) = scripted(vec![
            Script::Ok(crate::transport::tool_reply("c1", "bash", &args)),
            Script::Ok(text_reply("could not publish")),
        ]);
        let first = s.start_goal("publish");
        s.turn(&first, &mut Null).unwrap();
        assert_eq!(s.goal_next(), Err("goal stopped: a confirmation was declined".into()));
    }

    #[test]
    fn every_request_counts_toward_the_task() {
        let args = json!({"command": "true"}).to_string();
        let (mut s, _fake) = scripted(vec![
            Script::Ok(crate::transport::tool_reply("c1", "bash", &args)),
            Script::Ok(crate::transport::tool_reply("c2", "bash", &args)),
            Script::Ok(text_reply("done")),
        ]);
        s.turn("go", &mut Null).unwrap();
        assert_eq!(s.task.steps, 2);
        // Three requests of the fake's 8 hit, 2 miss, 5 out each.
        assert_eq!((s.task.hit, s.task.miss, s.task.out), (24, 6, 15), "{:?}", s.task);
    }

    #[test]
    fn suggestions_are_one_short_line() {
        assert_eq!(clean_suggestion(" \"yes, run them\"\n").as_deref(), Some("yes, run them"));
        assert_eq!(clean_suggestion(""), None);
        assert_eq!(clean_suggestion("a\nb"), None);
        assert_eq!(clean_suggestion(&"x".repeat(61)), None);
        assert_eq!(clean_suggestion("should I commit this").as_deref(), Some("should I commit this?"));
        assert_eq!(clean_suggestion("Should I commit this.").as_deref(), Some("Should I commit this?"));
        assert_eq!(clean_suggestion("what next?").as_deref(), Some("what next?"));
        assert_eq!(clean_suggestion("run the tests").as_deref(), Some("run the tests"));
    }

    #[test]
    fn steered_messages_join_the_turn_before_the_next_request() {
        let (mut s, _fake) = scripted(vec![Script::Ok(text_reply("done"))]);
        s.steer.lock().unwrap().push("use pnpm, not npm".into());
        s.turn("install it", &mut Null).unwrap();
        let users: Vec<&str> = s.history.iter().filter(|m| m.role == crate::api::Role::User).map(|m| m.content.as_str()).collect();
        assert_eq!(users, vec!["install it", "use pnpm, not npm"]);
        assert!(s.steer.lock().unwrap().is_empty());
    }

    #[test]
    fn a_held_turn_makes_no_request_until_released_or_stopped() {
        // No scripted reply: any request would fail the turn.
        let (mut s, _fake) = scripted(vec![]);
        s.hold.store(true, std::sync::atomic::Ordering::SeqCst);
        let cancel = s.cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        struct Notes(Vec<String>);
        impl Hooks for Notes {
            fn ui(&mut self, event: Ui) {
                if let Ui::Info(t) | Ui::Notice(t) | Ui::Error(t) = event {
                    self.0.push(t);
                }
            }
            fn confirm(&mut self, _: &str) -> bool {
                false
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
        }
        let mut hooks = Notes(Vec::new());
        s.turn("go", &mut hooks).unwrap();
        assert_eq!(hooks.0, vec!["cancelled".to_string()]);
    }

    #[test]
    fn the_first_version_of_each_changed_file_is_kept() {
        let (mut s, _fake) = scripted(vec![]);
        let root = s.tools.root.clone();
        std::fs::write(root.join("calc.py"), "a - b\n").unwrap();
        std::fs::write(root.join("big.bin"), vec![0xffu8; 16]).unwrap();
        let big = std::fs::File::create(root.join("large.txt")).unwrap();
        big.set_len(2 * 1024 * 1024).unwrap();
        let edit = json!({"path": "calc.py", "edits": [{"old": "a - b", "new": "a + b"}]});
        assert!(s.run_tool("edit", &edit, &mut Null).0);
        let again = json!({"path": "calc.py", "edits": [{"old": "a + b", "new": "a * b"}]});
        assert!(s.run_tool("edit", &again, &mut Null).0);
        s.run_tool("write", &json!({"path": "notes.md", "content": "new"}), &mut Null);
        s.remember_original("big.bin");
        s.remember_original("large.txt");
        let found = |name: &str| s.originals.iter().find(|(p, _)| p.ends_with(name)).map(|(_, b)| b.clone());
        assert_eq!(found("calc.py"), Some(Before::Text("a - b\n".into())), "the second edit replaced the first version");
        assert_eq!(found("notes.md"), Some(Before::Missing));
        assert_eq!(found("big.bin"), Some(Before::Untracked));
        assert_eq!(found("large.txt"), Some(Before::Untracked));
        assert_eq!(s.originals.len(), 4);
    }

    #[test]
    fn session_changes_are_net_per_file() {
        let (mut s, _fake) = scripted(vec![]);
        let root = s.tools.root.clone();
        std::fs::write(root.join("calc.py"), "def add(a, b):\n    return a - b\n").unwrap();
        let e = |old: &str, new: &str| json!({"path": "calc.py", "edits": [{"old": old, "new": new}]});
        assert!(s.run_tool("edit", &e("a - b", "a + b"), &mut Null).0);
        assert!(s.run_tool("edit", &e("a + b", "a * b"), &mut Null).0);
        s.run_tool("write", &json!({"path": "notes.md", "content": "one\ntwo\n"}), &mut Null);
        std::fs::write(root.join("same.txt"), "x\n").unwrap();
        s.remember_original("same.txt");
        let changes = s.session_changes();
        let names: Vec<&str> = changes.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["calc.py", "notes.md"], "an unchanged file was listed");
        assert_eq!((changes[0].adds, changes[0].dels, changes[0].ext.as_str()), (1, 1, "py"));
        assert!(changes[0].rows.contains(&"-    return a - b".to_string()) && changes[0].rows.contains(&"+    return a * b".to_string()));
        assert_eq!((changes[1].adds, changes[1].dels), (2, 0));
    }

    #[test]
    fn git_diff_output_is_split_by_file() {
        let text = "diff --git a/src/main.rs b/src/main.rs\nindex 1..2 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    let x = 1;\n+    let x = 2;\n@@ -9 +9 @@\n-a\n+b\ndiff --git a/notes.txt b/notes.txt\nnew file mode 100644\n--- /dev/null\n+++ b/notes.txt\n@@ -0,0 +1 @@\n+hello\ndiff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\n";
        let files = parse_git_diff(text);
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["src/main.rs", "notes.txt", "logo.png"]);
        assert_eq!((files[0].ext.as_str(), files[0].adds, files[0].dels), ("rs", 2, 2));
        assert_eq!(files[0].rows, vec![" fn main() {", "-    let x = 1;", "+    let x = 2;", "⋯", "-a", "+b"]);
        assert_eq!((files[1].adds, files[1].rows.len()), (1, 1));
        assert!(files[2].untracked);
    }

    #[test]
    fn git_changes_run_in_the_session_directory() {
        let (s, _fake) = scripted(vec![]);
        let root = s.tools.root.clone();
        let git = |args: &[&str]| std::process::Command::new("git").args(args).current_dir(&root).output().unwrap();
        if !git(&["init", "-q"]).status.success() {
            return;
        }
        std::fs::write(root.join("a.txt"), "one\n").unwrap();
        git(&["add", "a.txt"]);
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "x"]);
        std::fs::write(root.join("a.txt"), "two\n").unwrap();
        let files = s.git_changes("").unwrap();
        assert_eq!((files.len(), files[0].name.as_str(), files[0].adds, files[0].dels), (1, "a.txt", 1, 1));
        git(&["add", "a.txt"]);
        assert!(s.git_changes("").unwrap().is_empty(), "staged changes showed without --staged");
        assert_eq!(s.git_changes("--staged").unwrap().len(), 1);
    }

    #[test]
    fn ask_takes_labelled_options_and_bare_strings() {
        let new = ask_options(&json!({"options": [{"label": "Rust", "description": "fast"}, {"label": "Go"}]}));
        assert_eq!(new, vec![("Rust".to_string(), "fast".to_string()), ("Go".to_string(), String::new())]);
        let old = ask_options(&json!({"options": ["yes", "no"]}));
        assert_eq!(old, vec![("yes".to_string(), String::new()), ("no".to_string(), String::new())]);
        // A front-end without descriptions gets the labels.
        struct Labels(Vec<String>);
        impl Hooks for Labels {
            fn ui(&mut self, _: Ui) {}
            fn confirm(&mut self, _: &str) -> bool {
                false
            }
            fn ask(&mut self, _: &str, options: &[String]) -> Option<String> {
                self.0 = options.to_vec();
                Some(options[0].clone())
            }
        }
        let (mut s, _fake) = scripted(vec![]);
        let mut hooks = Labels(Vec::new());
        let args = json!({"question": "which?", "options": [{"label": "Rust", "description": "fast"}, "Go"]});
        assert_eq!(s.run_tool("ask", &args, &mut hooks), (true, "Rust".to_string()));
        assert_eq!(hooks.0, vec!["Rust", "Go"]);
    }

    #[test]
    fn ask_batch_returns_ordered_structured_answers_and_preserves_legacy_result() {
        struct Choices;
        impl Hooks for Choices {
            fn ui(&mut self, _: Ui) {}
            fn confirm(&mut self, _: &str) -> bool { false }
            fn ask(&mut self, question: &str, options: &[String]) -> Option<String> {
                if question.ends_with("language?") { Some(options[1].clone()) } else { Some("custom answer".into()) }
            }
        }
        let (mut s, _fake) = scripted(vec![]);
        let args = json!({"questions": [
            {"header": "Language", "question": "language?", "options": [{"label": "Go"}, {"label": "Rust"}]},
            {"header": "Other", "question": "other?"}
        ]});
        let (ok, out) = s.run_tool("ask", &args, &mut Choices);
        assert!(ok, "{out}");
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), json!({"answers": [vec!["Rust"], vec!["custom answer"]]}));
        assert_eq!(s.run_tool("ask", &json!({"question":"language?", "options":["Go", "Rust"]}), &mut Choices), (true, "Rust".into()));
    }

    #[test]
    fn ask_batch_rejects_empty_and_malformed_questions_without_prompting() {
        struct NoPrompt;
        impl Hooks for NoPrompt {
            fn ui(&mut self, _: Ui) {}
            fn confirm(&mut self, _: &str) -> bool { false }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> { panic!("prompt opened for invalid batch") }
        }
        let (mut s, _fake) = scripted(vec![]);
        for questions in [
            json!([]),
            json!([{"header":"missing question"}]),
            json!([{"question":"ok?", "options": [1,2]}]),
            json!([{"question":"ok?", "options": ["1", "2", "3", "4", "5", "6", "7"]}]),
        ] {
            let out = s.run_tool("ask", &json!({"questions":questions}), &mut NoPrompt);
            assert!(!out.0, "invalid batch accepted: {out:?}");
        }
    }

    #[test]
    fn dismissed_batch_does_not_run_later_calls_in_the_same_response() {
        struct Dismiss { started: Vec<String> }
        impl Hooks for Dismiss {
            fn ui(&mut self, event: Ui) {
                if let Ui::ToolStart { name, .. } = event { self.started.push(name); }
            }
            fn confirm(&mut self, _: &str) -> bool { true }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> { None }
        }
        let mut response = tool_reply("c1", "ask", &json!({"questions":[{"question":"continue?"}]}).to_string());
        response.insert(2, format!("data: {}\n\n", json!({"choices":[{"index":0,"delta":{"tool_calls":[{
            "index":1,"id":"c2","type":"function","function":{"name":"bash","arguments":"{\"command\":\"true\"}"}
        }]},"finish_reason":null}]})));
        let (mut s, _fake) = scripted(vec![Script::Ok(response), Script::Ok(text_reply("unused"))]);
        let mut hooks = Dismiss { started: Vec::new() };
        s.turn("go", &mut hooks).unwrap();
        assert_eq!(hooks.started, ["ask"]);
        assert!(s.history.iter().any(|m| m.role == crate::api::Role::Tool && m.content.contains("user did not answer")));
    }

    #[test]
    fn a_foreground_command_asks_then_takes_the_terminal() {
        struct Screen {
            asked: usize,
            yes: bool,
            ran: Option<String>,
            chat: bool,
        }
        impl Hooks for Screen {
            fn ui(&mut self, _: Ui) {}
            fn confirm(&mut self, _: &str) -> bool {
                self.asked += 1;
                self.yes
            }
            fn confirm_always(&mut self, what: &str) -> Answer {
                if self.confirm(what) { Answer::Yes } else { Answer::No }
            }
            fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
                None
            }
            fn run_attached(&mut self, command: &str) -> Option<i32> {
                self.ran = Some(command.to_string());
                Some(0)
            }
            fn can_foreground(&self) -> bool {
                self.chat
            }
        }
        let run = |mode: Permissions, yes: bool, chat: bool, cmd: &str| {
            let (mut s, _fake) = scripted(vec![]);
            s.cfg.set_permissions(mode);
            let mut h = Screen { asked: 0, yes, ran: None, chat };
            let out = s.run_tool("bash", &json!({"command": cmd, "foreground": true}), &mut h);
            (out, h.asked, h.ran)
        };
        // Asks even for a safe command, then hands over and reports the code.
        let ((ok, text), asked, ran) = run(Permissions::Normal, true, true, "ls");
        assert!(ok && text == "exit 0\n(ran in the foreground; output not captured)", "{text}");
        assert_eq!(asked, 1);
        assert!(ran.unwrap().contains("ls"));
        // Strict asks too; yolo does not.
        assert_eq!(run(Permissions::Strict, true, true, "htop").1, 1);
        assert_eq!(run(Permissions::Yolo, false, true, "htop").1, 0);
        // Declined: nothing runs.
        let ((ok, text), _, ran) = run(Permissions::Normal, false, true, "htop");
        assert!(!ok && text.starts_with("refused: the user declined") && ran.is_none());
        // The deny list still holds; line mode refuses.
        let ((ok, _), asked, _) = run(Permissions::Normal, true, true, "shutdown now");
        assert!(!ok && asked == 0);
        let ((ok, text), _, ran) = run(Permissions::Normal, true, false, "htop");
        assert!(!ok && text == "refused: foreground needs the chat interface" && ran.is_none());
    }

    #[test]
    fn a_suggestion_leaves_the_history_alone() {
        let (mut s, _fake) = scripted(vec![
            Script::Ok(text_reply("Shall I run the tests?")),
            Script::Ok(text_reply("yes, run them")),
        ]);
        s.cfg.suggest = true;
        s.turn("fix it", &mut Null).unwrap();
        assert!(s.wants_suggestion(), "a plain turn should be followed by a suggestion");
        let before = s.history.len();
        let (text, _) = s.suggest().unwrap();
        assert_eq!(text, "yes, run them");
        assert_eq!(s.history.len(), before);
    }

    fn session_with_log(script: Vec<Script>) -> (Session, Arc<Fake>, std::path::PathBuf) {
        let (mut s, fake) = scripted(script);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("imp-summary-{}-{nanos}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        s.attach_log(crate::store::Log::create(&dir, &dir, "deepseek-flash"));
        s.cfg.provider = crate::providers::Provider::DeepSeek;
        (s, fake, dir)
    }

    #[test]
    fn a_long_first_message_gets_one_summary_title() {
        let long = "Can you please look at why the rsync test keeps failing on the Pi when the network is slow";
        let (mut s, _fake, dir) = session_with_log(vec![
            Script::Ok(text_reply("Done.")),
            Script::Ok(text_reply("\"Flaky rsync test on slow network.\"")),
        ]);
        s.turn(long, &mut Null).unwrap();
        let (title, _) = s.title_summary().expect("no summary title");
        assert_eq!(title, "Flaky rsync test on slow network");
        // The worker stores the title; a stored summary is never asked for again.
        s.log.as_ref().unwrap().retitle(&title).unwrap();
        assert!(s.title_summary().is_none(), "a second summary was requested");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_short_first_message_asks_for_no_summary() {
        let (mut s, _fake, dir) = session_with_log(vec![Script::Ok(text_reply("Done."))]);
        s.turn("fix the parser", &mut Null).unwrap();
        assert!(s.title_summary().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_summary_keeps_the_cut_title() {
        let long = "Can you please look at why the rsync test keeps failing on the Pi when the network is slow";
        let (mut s, _fake, dir) = session_with_log(vec![
            Script::Ok(text_reply("Done.")),
            Script::Fails(429, "slow down".into()),
            Script::Fails(429, "slow down".into()),
            Script::Fails(429, "slow down".into()),
        ]);
        s.turn(long, &mut Null).unwrap();
        assert!(s.title_summary().is_none(), "a failed request produced a title");
        assert_eq!(s.log.as_ref().unwrap().meta().title, "look at why the rsync test keeps…");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cat_read_lets_a_write_overwrite() {
        let (mut s, _fake) = scripted(vec![]);
        std::fs::write(s.tools.root.join("notes.md"), "old\n").unwrap();
        let (ok, out) = s.run_tool("write", &json!({"path": "notes.md", "content": "new"}), &mut Null);
        assert!(!ok && out.contains("before overwriting"), "{out}");
        let (ok, _) = s.run_tool("bash", &json!({"command": "cat -n notes.md"}), &mut Null);
        assert!(ok);
        let (ok, out) = s.run_tool("write", &json!({"path": "notes.md", "content": "new"}), &mut Null);
        assert!(ok, "{out}");
    }

    #[test]
    fn jailed_writes_are_refused_without_asking() {
        let (mut s, _fake) = scripted(vec![]);
        let mut hooks = Counting::default();
        let outside = std::env::temp_dir().join(format!("imp-outside-{}.txt", std::process::id()));
        let (ok, out) = s.run_tool(
            "edit",
            &json!({"path": outside.display().to_string(), "edits": [{"old": "a", "new": "b"}]}),
            &mut hooks,
        );
        assert!(!ok);
        assert!(out.contains("outside the session directory"), "{out}");
        let (ok, out) = s.run_tool("write", &json!({"path": "inside.txt", "content": "x"}), &mut hooks);
        assert!(ok, "{out}");
        assert_eq!(hooks.0, 0, "a prompt appeared");
    }

    struct Yes;
    impl Hooks for Yes {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    // Built by concatenation so tooling that greps commands for the word does
    // not treat this test as a request to run one.
    fn privileged_command() -> String {
        format!("{} -n true", ["su", "do"].concat())
    }

    fn privileged_turn(allow: bool, confirm: bool) -> String {
        let d = tmp();
        let mut cfg = crate::config::Config::default();
        cfg.allow_sudo = allow;
        let args = serde_json::json!({ "command": privileged_command() }).to_string();
        let fake = std::sync::Arc::new(crate::transport::Fake::new(vec![
            crate::transport::Script::Ok(crate::transport::tool_reply("c1", "bash", &args)),
            crate::transport::Script::Ok(crate::transport::text_reply("done")),
        ]));
        let mut s = Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        s.transport = Box::new(fake);
        if confirm {
            let _ = s.turn("try", &mut Yes);
        } else {
            let _ = s.turn("try", &mut Null);
        }
        s.history
            .iter()
            .find(|m| m.role == crate::api::Role::Tool)
            .map(|m| m.content.clone())
            .unwrap_or_default()
    }

    struct Attach(bool);
    impl Hooks for Attach {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
        fn run_attached(&mut self, _: &str) -> Option<i32> {
            self.0 = true;
            Some(7)
        }
    }

    /// One HTTP response from a throwaway local server, for fetch tests.
    fn serve_once(response: &'static str) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{addr}/data")
    }

    /// A local server that answers each request with the request itself.
    fn echo_server(times: usize) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for _ in 0..times {
                let Ok((mut stream, _)) = listener.accept() else { return };
                stream.set_read_timeout(Some(std::time::Duration::from_millis(300))).ok();
                let mut got = Vec::new();
                let mut buf = [0u8; 4096];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    got.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&got);
                    if let Some((head, body)) = text.split_once("\r\n\r\n") {
                        let want: usize = head
                            .lines()
                            .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0)))
                            .unwrap_or(0);
                        if body.len() >= want {
                            break;
                        }
                    }
                }
                let body = String::from_utf8_lossy(&got).into_owned();
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn credentials_go_to_their_host_only_and_never_show() {
        use crate::secrets::{TEST_NAME, TEST_VALUE};
        std::env::set_var(TEST_NAME, TEST_VALUE);
        let api = echo_server(2);
        let other = echo_server(1);
        let host = api.trim_start_matches("http://").to_string();
        let calls = [
            Script::Ok(tool_reply("c1", "fetch", &json!({"url": format!("{api}/user")}).to_string())),
            Script::Ok(tool_reply("c2", "fetch", &json!({"url": format!("{api}/repos")}).to_string())),
            Script::Ok(tool_reply("c3", "fetch", &json!({"url": format!("{other}/?u={host}")}).to_string())),
            Script::Ok(text_reply("done")),
        ];
        let (mut s, _fake) = scripted(calls.into());
        s.cfg.set_permissions(Permissions::Normal);
        s.cfg.auth.push((host.clone(), format!("Authorization: Bearer ${TEST_NAME}")));
        let mut hooks = CountingHooks::default();
        s.turn("go", &mut hooks).unwrap();
        assert_eq!(hooks.confirms, 1, "a credentialed read asked other than once per host");
        let results: Vec<&str> = s.history.iter().filter(|m| m.role == crate::api::Role::Tool).map(|m| m.content.as_str()).collect();
        let shown = format!("Authorization: Bearer [redacted {TEST_NAME}]");
        assert!(results[0].contains(&shown), "{}", results[0]);
        assert!(results[1].contains(&shown), "{}", results[1]);
        assert!(!results[2].to_ascii_lowercase().contains("authorization"), "sent to another host: {}", results[2]);
        assert!(!s.history.iter().any(|m| m.content.contains(TEST_VALUE)), "the key reached history");
    }

    #[test]
    fn writes_ask_every_time_and_send_json() {
        let api = echo_server(2);
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Normal);
        let mut hooks = CountingHooks::default();
        let args = json!({"url": format!("{api}/items"), "method": "post", "body": {"title": "x"}});
        let (ok, out) = s.run_tool("fetch", &args, &mut hooks);
        assert!(ok, "{out}");
        assert!(out.contains("POST /items ") && out.contains("Content-Type: application/json") && out.ends_with("{\"title\":\"x\"}"), "{out}");
        let text = json!({"url": format!("{api}/items"), "method": "PUT", "body": "{\"title\":\"y\"}"});
        let (_, out) = s.run_tool("fetch", &text, &mut hooks);
        assert!(out.contains("PUT /items ") && out.contains("Content-Type: application/json"), "JSON text not sent as JSON: {out}");
        assert_eq!(hooks.confirms, 2, "a second write did not ask");
        let (ok, out) = s.run_tool("fetch", &json!({"url": api, "method": "TRACE"}), &mut hooks);
        assert!(!ok && out.contains("not TRACE"), "{out}");
    }

    #[test]
    fn credentials_never_follow_a_redirect_and_need_their_variable() {
        use crate::secrets::{TEST_NAME, TEST_VALUE};
        std::env::set_var(TEST_NAME, TEST_VALUE);
        let api = serve_once("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/stolen\r\nConnection: close\r\n\r\n");
        let host = crate::perm::url_host(&api);
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Yolo);
        s.cfg.auth.push((host.clone(), format!("X-Key: ${TEST_NAME}")));
        let (ok, out) = s.run_tool("fetch", &json!({"url": api}), &mut Null);
        assert!(ok, "{out}");
        assert!(out.starts_with("302 ") && out.contains("Location: http://127.0.0.1:9/stolen"), "{out}");
        s.cfg.auth[0].1 = "X-Key: $IMP_SURELY_UNSET_VAR".into();
        let (ok, out) = s.run_tool("fetch", &json!({"url": api}), &mut Null);
        assert!(!ok && out.contains("$IMP_SURELY_UNSET_VAR"), "{out}");
    }

    #[test]
    fn fetch_returns_status_type_and_body() {
        let url = serve_once(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"moon\":0.92}",
        );
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Yolo);
        let (ok, out) = s.run_tool("fetch", &json!({"url": url}), &mut Null);
        assert!(ok, "{out}");
        assert_eq!(out, "200 application/json\n{\"moon\":0.92}");

        let page = serve_once(
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<title>Gone</title><p>No such page</p>",
        );
        let (ok, out) = s.run_tool("fetch", &json!({"url": page}), &mut Null);
        assert!(!ok, "{out}");
        assert!(out.starts_with("404 text/html\n# Gone\n\nNo such page"), "{out}");

        let (ok, out) = s.run_tool("fetch", &json!({"url": "file:///etc/passwd"}), &mut Null);
        assert!(!ok && out.contains("http or https"), "{out}");
    }

    #[test]
    fn reading_the_web_does_not_ask_in_normal() {
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Normal);
        let mut hooks = CountingHooks::default();
        let a = serve_once("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\none");
        assert!(s.run_tool("fetch", &json!({"url": a}), &mut hooks).0);
        let b = serve_once("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\ntwo");
        let out = s.run_tool("bash", &json!({"command": format!("curl -s {b} | head -c 2")}), &mut hooks);
        assert!(out.0, "{}", out.1);
        assert_eq!(hooks.confirms, 0, "a plain read asked");
        let long = format!("{a}?q={}", "x".repeat(400));
        let _ = s.run_tool("fetch", &json!({"url": long}), &mut hooks);
        assert_eq!(hooks.confirms, 1, "a long URL did not ask");
        let _ = s.run_tool("bash", &json!({"command": format!("curl -X POST {b}")}), &mut hooks);
        assert_eq!(hooks.confirms, 2, "a post did not ask");

        let (mut strict, _f) = scripted(vec![]);
        strict.cfg.set_permissions(Permissions::Strict);
        let mut hooks = CountingHooks::default();
        let _ = strict.run_tool("fetch", &json!({"url": "http://127.0.0.1:9/"}), &mut hooks);
        let _ = strict.run_tool("fetch", &json!({"url": "http://127.0.0.1:9/"}), &mut hooks);
        assert_eq!(hooks.confirms, 2, "strict did not ask every time");
    }

    #[test]
    fn a_refused_privilege_command_never_reaches_the_terminal() {
        let d = tmp();
        let cfg = crate::config::Config::default();
        assert!(!cfg.allow_sudo);
        let args = serde_json::json!({ "command": privileged_command() }).to_string();
        let fake = std::sync::Arc::new(crate::transport::Fake::new(vec![
            crate::transport::Script::Ok(crate::transport::tool_reply("c1", "bash", &args)),
            crate::transport::Script::Ok(crate::transport::text_reply("done")),
        ]));
        let mut s = Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        s.transport = Box::new(fake);
        let mut hooks = Attach(false);
        s.turn("try", &mut hooks).unwrap();
        assert!(!hooks.0, "ran attached without allow_sudo");
        let tool = s
            .history
            .iter()
            .find(|m| m.role == crate::api::Role::Tool)
            .unwrap();
        assert!(tool.content.contains("refused"), "{}", tool.content);
    }

    #[test]
    fn a_privilege_command_is_handed_to_the_terminal() {
        let d = tmp();
        let mut cfg = crate::config::Config::default();
        cfg.allow_sudo = true;
        let args = serde_json::json!({ "command": privileged_command() }).to_string();
        let fake = std::sync::Arc::new(crate::transport::Fake::new(vec![
            crate::transport::Script::Ok(crate::transport::tool_reply("c1", "bash", &args)),
            crate::transport::Script::Ok(crate::transport::text_reply("done")),
        ]));
        let mut s = Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        s.transport = Box::new(fake);
        let mut hooks = Attach(false);
        s.turn("try", &mut hooks).unwrap();
        assert!(hooks.0, "the hook was not used");
        let tool = s
            .history
            .iter()
            .find(|m| m.role == crate::api::Role::Tool)
            .unwrap();
        assert!(tool.content.contains("exit 7"), "{}", tool.content);
        assert!(tool.content.contains("attached"), "{}", tool.content);
    }

    #[test]
    fn privilege_commands_need_the_setting() {
        let refused = privileged_turn(false, false);
        assert!(refused.contains("allow_sudo"), "{refused}");
    }

    #[test]
    fn privilege_commands_can_be_declined() {
        let declined = privileged_turn(true, false);
        assert!(declined.contains("declined"), "{declined}");
    }

    #[test]
    fn an_approved_privilege_command_runs() {
        let ran = privileged_turn(true, true);
        assert!(!ran.contains("allow_sudo"), "{ran}");
        assert!(!ran.contains("declined"), "{ran}");
    }

    #[test]
    fn a_declined_memory_write_changes_nothing() {
        let d = tmp();
        let memory_dir = d.join("memory");
        let (mut s, _fake) = scripted(vec![
            Script::Ok(tool_reply(
                "c1",
                "memory",
                "{\"op\":\"save\",\"title\":\"note\",\"body\":\"a fact\"}",
            )),
            Script::Ok(text_reply("declined")),
        ]);
        s.memory = crate::store::Memory {
            dirs: vec![memory_dir.clone()],
        };
        s.cfg.suggest = true;
        s.turn("save it", &mut Null).unwrap();
        assert!(!memory_dir.exists() || std::fs::read_dir(&memory_dir).unwrap().count() == 0);
        assert!(!s.wants_suggestion(), "a suggestion would follow a decline");
    }

    #[test]
    fn yolo_saves_notes_without_asking() {
        for (mode, asks) in [(Permissions::Yolo, 0), (Permissions::Normal, 1)] {
            let d = tmp();
            let memory_dir = d.join("memory");
            let (mut s, _fake) = scripted(vec![
                Script::Ok(tool_reply(
                    "c1",
                    "memory",
                    "{\"op\":\"save\",\"title\":\"note\",\"body\":\"a fact\"}",
                )),
                Script::Ok(text_reply("saved")),
            ]);
            s.memory = crate::store::Memory {
                dirs: vec![memory_dir.clone()],
            };
            s.cfg.set_permissions(mode);
            let mut hooks = CountingHooks::default();
            s.turn("save it", &mut hooks).unwrap();
            assert_eq!(hooks.confirms, asks, "{mode:?}");
            assert_eq!(std::fs::read_dir(&memory_dir).map(|r| r.count()).unwrap_or(0), 1, "{mode:?}");
        }
    }

    #[test]
    fn second_overflow_is_a_clean_error() {
        let (mut s, fake) = scripted(vec![
            Script::Fails(400, "context_length_exceeded".into()),
            Script::Fails(400, "context_length_exceeded".into()),
        ]);
        for _ in 0..10 {
            s.history.push(Message::tool("c", "y".repeat(4000)));
        }
        let err = s.turn("hi", &mut Null).unwrap_err();
        assert_eq!(fake.attempts(), 2);
        assert!(err.contains("400"), "{err}");
    }

    #[test]
    fn tool_call_round_trip() {
        let (mut s, fake) = scripted(vec![
            Script::Ok(tool_reply("call_1", "bash", "{\"command\":\"echo hi\"}")),
            Script::Ok(text_reply("done")),
        ]);
        s.turn("run echo", &mut Null).unwrap();
        assert_eq!(fake.attempts(), 2);
        let tool = s
            .history
            .iter()
            .find(|m| m.role == crate::api::Role::Tool)
            .expect("tool result");
        assert!(tool.content.contains("hi"), "{}", tool.content);
        assert!(fake.last_body().contains("call_1"));
    }

    #[test]
    fn compaction_merges_a_summary_and_keeps_four_exchanges() {
        let (mut s, fake) = scripted(vec![Script::Ok(text_reply(
            "[compacted]\nDecisions: shipped v1.",
        ))]);
        s.history.push(Message::system("base"));
        for i in 0..6 {
            s.history.push(Message::user(format!("q{i}")));
            s.history.push(Message::assistant(format!("a{i}")));
        }
        s.compact(&mut Null, None).unwrap();
        assert_eq!(fake.attempts(), 1);
        assert!(s
            .history
            .iter()
            .any(|m| m.role == crate::api::Role::System && m.content.contains("[compacted]")));
        assert_eq!(
            s.history
                .iter()
                .filter(|m| m.role == crate::api::Role::User)
                .count(),
            4
        );
        assert_eq!(s.history[0].content, "base");
    }

    #[test]
    fn estimator_crosses_the_thresholds() {
        let small = vec![Message::user("hi")];
        assert!(crate::api::estimate_tokens(&small) < 45_000);
        let big = vec![Message::user("x".repeat(300_000))];
        assert!(crate::api::estimate_tokens(&big) > 45_000);
        assert!(crate::api::estimate_tokens(&big) > 60_000);
    }

    #[test]
    fn prune_stubs_old_tool_results() {
        let mut cfg = crate::config::Config::default();
        cfg.prune_at = 45_000;
        let mut s = session(cfg);
        for _ in 0..30 {
            s.history.push(Message::tool("c", "y".repeat(8000)));
        }
        s.history.push(Message::user("now"));
        let before = crate::api::estimate_tokens(&s.history);
        let pruned = s.prune(&mut Null);
        assert!(pruned > 0);
        assert!(crate::api::estimate_tokens(&s.history) < before / 2);
        assert!(s.history.iter().any(|m| m.content.contains("[pruned")));
    }

    #[test]
    fn compaction_keeps_the_last_four_exchanges() {
        let mut history = vec![Message::system("base")];
        for i in 0..6 {
            history.push(Message::user(format!("q{i}")));
            history.push(Message::assistant(format!("a{i}")));
        }
        let keep = keep_index(&history).unwrap();
        assert_eq!(history[keep].content, "q2");
        let kept_users = history[keep..]
            .iter()
            .filter(|m| m.role == crate::api::Role::User)
            .count();
        assert_eq!(kept_users, 4);
        assert!(keep_index(&history[..5]).is_none());
    }

    /// Collects the info lines the compaction reports.
    #[derive(Default)]
    struct Notes(Vec<String>);
    impl Hooks for Notes {
        fn ui(&mut self, event: Ui) {
            if let Ui::Info(text) = event {
                self.0.push(text);
            }
        }
        fn confirm(&mut self, _: &str) -> bool {
            false
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    fn six_exchanges(s: &mut Session) {
        s.history.push(Message::system("base"));
        for i in 0..6 {
            s.history.push(Message::user(format!("q{i}")));
            s.history.push(Message::assistant(format!("a{i}")));
        }
    }

    #[test]
    fn focus_line_goes_before_the_transcript() {
        let (mut s, fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        s.compact(&mut Null, Some("keep the auth decisions")).unwrap();
        let body = fake.sent.lock().unwrap()[0].clone();
        let focus = body.find("Focus: keep the auth decisions").expect(&body);
        let transcript = body.find("user: q0").expect(&body);
        assert!(focus < transcript, "focus is after the transcript:\n{body}");
    }

    #[test]
    fn no_focus_sends_no_focus_line() {
        let (mut s, fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        s.compact(&mut Null, None).unwrap();
        let body = fake.sent.lock().unwrap()[0].clone();
        assert!(body.contains("Summarise this conversation"), "{body}");
        assert!(!body.contains("Focus:"), "{body}");
    }

    #[test]
    fn blank_focus_counts_as_no_focus() {
        let (mut s, fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        s.compact(&mut Null, Some("   ")).unwrap();
        let body = fake.sent.lock().unwrap()[0].clone();
        assert!(!body.contains("Focus:"), "{body}");
    }

    #[test]
    fn long_focus_is_trimmed_cut_and_reported() {
        let (mut s, fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        let long = format!("  {}  ", "x".repeat(600));
        let mut notes = Notes::default();
        s.compact(&mut notes, Some(&long)).unwrap();
        let body = fake.sent.lock().unwrap()[0].clone();
        assert!(body.contains(&format!("Focus: {}", "x".repeat(500))), "{body}");
        assert!(!body.contains(&"x".repeat(501)), "focus was not cut");
        assert!(
            notes.0.iter().any(|t| t.contains("focus cut to 500 characters")),
            "{:?}",
            notes.0
        );
    }

    #[test]
    fn focus_with_a_newline_stays_one_line() {
        let (mut s, fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        s.compact(&mut Null, Some("keep the auth\nand the tests")).unwrap();
        let body = fake.sent.lock().unwrap()[0].clone();
        assert!(body.contains("Focus: keep the auth and the tests"), "{body}");
    }

    #[test]
    fn short_focus_is_not_reported_as_cut() {
        let (mut s, _fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        let mut notes = Notes::default();
        s.compact(&mut notes, Some("keep the auth decisions")).unwrap();
        assert!(!notes.0.iter().any(|t| t.contains("focus cut")), "{:?}", notes.0);
    }

    #[test]
    fn focused_compaction_keeps_the_last_four_exchanges() {
        let (mut s, _fake) = scripted(vec![Script::Ok(text_reply("[compacted]\nok"))]);
        six_exchanges(&mut s);
        s.compact(&mut Null, Some("keep the auth decisions")).unwrap();
        let users: Vec<&str> = s
            .history
            .iter()
            .filter(|m| m.role == crate::api::Role::User)
            .map(|m| m.content.as_str())
            .collect();
        assert_eq!(users, ["q2", "q3", "q4", "q5"]);
    }

    #[test]
    fn automatic_compaction_sends_no_focus() {
        let (mut s, fake) = scripted(vec![
            Script::Ok(text_reply("[compacted]\nok")),
            Script::Ok(text_reply("done")),
        ]);
        six_exchanges(&mut s);
        s.cfg.compact_at = 1;
        s.turn("go", &mut Null).unwrap();
        let first = fake.sent.lock().unwrap()[0].clone();
        assert!(first.contains("Summarise this conversation"), "{first}");
        assert!(!first.contains("Focus:"), "{first}");
    }

    #[test]
    fn switching_agents_appends_without_rewriting_the_prefix() {
        let mut s = session(crate::config::Config::default());
        s.history.push(Message::system("base"));
        let prefix = s.history[0].to_json().to_string();
        let mut reviewer = Agent::builtin();
        reviewer.name = "reviewer".into();
        reviewer.instructions = "review things".into();
        s.switch_agent(reviewer);
        assert_eq!(s.history[0].to_json().to_string(), prefix);
        let last = s.history.last().unwrap();
        assert_eq!(last.role, crate::api::Role::System);
        assert!(last.content.contains("supersede"));
    }

    #[test]
    fn detects_context_overflow_only_on_400() {
        assert!(is_context_overflow(
            400,
            r#"{"error":{"message":"This model's maximum context length is 65536"}}"#
        ));
        assert!(is_context_overflow(400, "context_length_exceeded"));
        assert!(!is_context_overflow(400, "invalid argument"));
        assert!(!is_context_overflow(500, "context_length_exceeded"));
    }

    #[test]
    fn extracts_at_most_three_facts() {
        let summary = "Did work.\nMEMORY: prefers jj over git\nMEMORY: uses make check\nMEMORY: no trailing lockfile\nMEMORY: fourth fact here\nMEMORY: short";
        let facts = facts_from_summary(summary);
        assert_eq!(facts.len(), 3);
        assert_eq!(facts[0], "prefers jj over git");
    }

    #[test]
    fn feature_tools_are_conditional() {
        assert!(!include_tool("memory", false, true));
        assert!(include_tool("memory", true, false));
        assert!(!include_tool("skill_load", true, false));
        assert!(include_tool("bash", false, false));
    }

    #[test]
    fn last_event_reads_the_newest() {
        let d = tmp();
        let log = Log::create(&d.join("sessions"), &d, "deepseek-flash");
        log.event("agent", "reviewer");
        log.event("agent", "default");
        assert_eq!(log.last_event("agent").as_deref(), Some("default"));
        assert_eq!(log.last_event("nope"), None);
    }

    #[test]
    fn resume_restores_the_active_agent() {
        let d = tmp();
        let sessions = d.join("sessions");
        let log = Log::create(&sessions, &d, "deepseek-flash");
        log.event("agent", "reviewer");

        let mut session = Session::new(
            crate::config::Config::default(),
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        session.resume_id(&sessions, &log.id).unwrap();
        assert_eq!(session.agent.name, "reviewer");
    }

    #[test]
    fn a_saved_default_resumes_as_build() {
        let d = tmp();
        let sessions = d.join("sessions");
        let log = Log::create(&sessions, &d, "deepseek-flash");
        log.event("agent", "default");
        let mut start = crate::store::builtin_agents().swap_remove(1);
        start.name = "plan".into();
        let mut session = Session::new(
            crate::config::Config::default(),
            d.clone(),
            d.join("out"),
            String::new(),
            start,
            crate::store::builtin_agents(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        session.resume_id(&sessions, &log.id).unwrap();
        assert_eq!(session.agent.name, "build");
    }

    #[test]
    fn resume_ignores_orphan_tool_results() {
        let d = tmp();
        let sessions = d.join("sessions");
        let log = Log::create(&sessions, &d, "deepseek-flash");
        log.append(&Message::tool("missing-call", "orphan"));
        log.append(&Message::user("hello"));
        let mut session = Session::new(
            crate::config::Config::default(),
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        session.resume_id(&sessions, &log.id).unwrap();
        assert!(session.history.iter().all(|m| m.tool_call_id.is_none()));
    }
    #[derive(Default)]
    struct CountingHooks {
        confirms: usize,
    }

    impl Hooks for CountingHooks {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            self.confirms += 1;
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    /// Answers `session` (or `always` when `save`) when offered, and counts
    /// what it was asked.
    #[derive(Default)]
    struct AlwaysHooks {
        offered: usize,
        plain: usize,
        save: bool,
    }

    impl Hooks for AlwaysHooks {
        fn ui(&mut self, _: Ui) {}
        fn confirm(&mut self, _: &str) -> bool {
            self.plain += 1;
            true
        }
        fn confirm_always(&mut self, _: &str) -> Answer {
            self.offered += 1;
            if self.save { Answer::Always } else { Answer::Session }
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    #[test]
    fn always_skips_the_same_shape_for_the_session() {
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Normal);
        let mut h = AlwaysHooks::default();
        let run = |s: &mut Session, h: &mut AlwaysHooks, cmd: &str| {
            s.run_tool("bash", &json!({"command": cmd}), h);
        };
        run(&mut s, &mut h, "printf a");
        assert_eq!((h.offered, h.plain), (0, 0), "printf is safe-listed");
        run(&mut s, &mut h, "touch imp-always-x");
        run(&mut s, &mut h, "touch imp-always-x");
        assert_eq!(h.offered, 1, "the same shape asked again");
        run(&mut s, &mut h, "touch imp-always-y");
        assert_eq!(h.offered, 2, "another argument did not ask");
        run(&mut s, &mut h, "touch imp-always-x && true");
        assert_eq!((h.offered, h.plain), (2, 1), "a compound command offered always");
        s.reset();
        run(&mut s, &mut h, "touch imp-always-x");
        assert_eq!(h.offered, 3, "always outlived /new");

        let (mut strict, _f) = scripted(vec![]);
        strict.cfg.set_permissions(Permissions::Strict);
        let mut h = AlwaysHooks::default();
        run(&mut strict, &mut h, "touch imp-always-x");
        run(&mut strict, &mut h, "touch imp-always-x");
        assert_eq!((h.offered, h.plain), (0, 2), "strict offered always");
    }

    #[test]
    fn a_revoked_allow_asks_again() {
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Normal);
        let mut h = AlwaysHooks::default();
        let run = |s: &mut Session, h: &mut AlwaysHooks, cmd: &str| {
            s.run_tool("bash", &json!({"command": cmd}), h);
        };
        let shape = crate::perm::shape("touch imp-revoked-x").expect("a simple shape");
        run(&mut s, &mut h, "touch imp-revoked-x");
        assert_eq!(h.offered, 1);
        s.revoke_allow(&shape, false).unwrap();
        run(&mut s, &mut h, "touch imp-revoked-x");
        assert_eq!(h.offered, 2, "a revoked allow did not ask again");
    }

    #[test]
    fn always_is_saved_for_every_session_but_not_strict() {
        let (mut s, _fake) = scripted(vec![]);
        s.cfg.set_permissions(Permissions::Normal);
        let mut h = AlwaysHooks { save: true, ..Default::default() };
        let run = |s: &mut Session, h: &mut AlwaysHooks, cmd: &str| {
            s.run_tool("bash", &json!({"command": cmd}), h);
        };
        run(&mut s, &mut h, "touch imp-saved-x");
        assert_eq!(h.offered, 1);
        assert!(s.cfg.allowed("touch imp-saved-x"), "not in the allow list");
        s.reset();
        run(&mut s, &mut h, "touch imp-saved-x");
        assert_eq!(h.offered, 1, "always did not outlive /new");

        let other_cfg = s.cfg.detached();
        assert!(other_cfg.allowed("touch imp-saved-x"), "another session lost it");
        s.cfg.set_permissions(Permissions::Strict);
        run(&mut s, &mut h, "touch imp-saved-x");
        assert_eq!(h.plain, 1, "an allow line skipped strict");
    }

    #[test]
    fn a_live_mode_change_applies_to_the_next_tool_call() {
        let d = tmp();
        let mut cfg = crate::config::Config::default();
        cfg.set_permissions(Permissions::Normal);
        let mut s = Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        let mut hooks = CountingHooks::default();

        // A second clone of the config stands in for the App's copy.
        let ui = s.cfg.clone();
        ui.mode
            .store(Permissions::Yolo.code(), std::sync::atomic::Ordering::Relaxed);
        let args = json!({"command": "mkdir imp-yolo-live"});
        let (ok, out) = s.run_tool("bash", &args, &mut hooks);
        assert!(ok, "{out}");
        assert_eq!(hooks.confirms, 0, "yolo still asked for confirmation");

        s.cfg.set_permissions(Permissions::Normal);
        let args = json!({"command": "mkdir imp-yolo-ask"});
        let (ok, _) = s.run_tool("bash", &args, &mut hooks);
        assert!(ok);
        assert_eq!(hooks.confirms, 1, "normal did not ask");
        let _ = std::fs::remove_dir(d.join("imp-yolo-live"));
        let _ = std::fs::remove_dir(d.join("imp-yolo-ask"));
    }

    #[test]
    fn yolo_toggles_back_to_the_launch_mode() {
        let d = tmp();
        let mut cfg = crate::config::Config::default();
        cfg.set_permissions(Permissions::Strict);
        let mut s = Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            roster(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        assert!(s.tools.jail, "strict starts with the jail");
        s.set_permissions(Permissions::Yolo);
        assert_eq!(s.cfg.permissions, Permissions::Yolo);
        assert!(!s.tools.jail, "yolo left the jail on");
        s.set_permissions(Permissions::Strict);
        assert_eq!(s.cfg.permissions, Permissions::Strict, "did not restore strict");
        assert!(s.tools.jail, "the jail did not come back");
    }

}

/// Whether the session title should be summarised: the first message is
/// longer than a cut title, the title is still the cut one (not renamed and
/// not summarised yet), and the setting allows it for this provider.
pub fn title_wanted(setting: crate::config::TitleSummary, provider: crate::providers::Provider, first: &str, stored: &str) -> bool {
    use crate::config::TitleSummary;
    use crate::providers::Provider;
    if first.chars().count() <= 40 || stored != crate::store::cut_title(first) {
        return false;
    }
    match setting {
        TitleSummary::Off => false,
        TitleSummary::On => true,
        TitleSummary::Auto => !matches!(provider, Provider::Ollama | Provider::Custom),
    }
}

/// The request for a summary: the first message alone, no history, no tools.
pub fn title_messages(first: &str) -> Vec<Message> {
    vec![
        Message::system("Write a title for the request below in at most six words. Reply with only the title."),
        Message::user(first),
    ]
}

/// A reply as a title: quotes and end punctuation stripped, at most six words.
pub fn title_from_reply(text: &str) -> Option<String> {
    let cleaned: String = text
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '.')
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ");
    (!cleaned.is_empty()).then_some(cleaned)
}

#[cfg(test)]
mod title_summary_tests {
    use super::*;
    use crate::config::TitleSummary;
    use crate::providers::Provider;

    const LONG: &str = "Can you please look at why the rsync test keeps failing on the Pi when the network is slow";

    #[test]
    fn a_long_first_message_wants_a_summary_on_a_hosted_provider() {
        let stored = crate::store::cut_title(LONG);
        assert!(title_wanted(TitleSummary::Auto, Provider::DeepSeek, LONG, &stored));
    }

    #[test]
    fn a_short_message_never_wants_one() {
        assert!(!title_wanted(TitleSummary::On, Provider::DeepSeek, "fix the parser", "fix the parser"));
    }

    #[test]
    fn a_rename_or_an_earlier_summary_is_kept() {
        assert!(!title_wanted(TitleSummary::On, Provider::DeepSeek, LONG, "fix sync"));
        assert!(!title_wanted(TitleSummary::On, Provider::DeepSeek, LONG, "Flaky rsync test on slow network"));
    }

    #[test]
    fn auto_is_off_for_local_and_custom_endpoints_and_on_or_off_is_explicit() {
        let stored = crate::store::cut_title(LONG);
        assert!(!title_wanted(TitleSummary::Auto, Provider::Ollama, LONG, &stored));
        assert!(!title_wanted(TitleSummary::Auto, Provider::Custom, LONG, &stored));
        assert!(title_wanted(TitleSummary::On, Provider::Ollama, LONG, &stored));
        assert!(!title_wanted(TitleSummary::Off, Provider::DeepSeek, LONG, &stored));
    }

    #[test]
    fn the_request_holds_only_the_first_message() {
        let messages = title_messages(LONG);
        assert_eq!(messages.len(), 2, "history or other text went in");
        assert_eq!(messages[1].content, LONG);
    }

    #[test]
    fn a_reply_becomes_a_clean_title_of_at_most_six_words() {
        assert_eq!(
            title_from_reply("\"Flaky rsync test on slow network.\"").as_deref(),
            Some("Flaky rsync test on slow network")
        );
        assert_eq!(
            title_from_reply("one two three four five six seven eight").as_deref(),
            Some("one two three four five six")
        );
        assert_eq!(title_from_reply("  \"\"  "), None);
    }
}

#[cfg(test)]
mod notice_tests {
    use super::*;
    use crate::config::Permissions;

    struct Notices(Vec<String>);
    impl Hooks for Notices {
        fn ui(&mut self, event: Ui) {
            if let Ui::Notice(text) = event {
                self.0.push(text);
            }
        }
        fn confirm(&mut self, _: &str) -> bool {
            false
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }

    #[test]
    fn a_finished_job_is_reported_as_a_notice() {
        let mut cfg = crate::config::Config::default();
        cfg.set_permissions(Permissions::Yolo);
        let d = std::env::temp_dir().join(format!("imp-notice-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let mut s = Session::new(
            cfg,
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            Vec::new(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        );
        let (ok, out) = s.run_tool("bash", &serde_json::json!({"command": "true", "background": true}), &mut Notices(Vec::new()));
        assert!(ok, "{out}");
        let id: u64 = out.split_whitespace().nth(1).unwrap().parse().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));
        let mut hooks = Notices(Vec::new());
        s.run_tool("job", &serde_json::json!({"op": "list"}), &mut hooks);
        assert!(
            hooks.0.iter().any(|t| t == &format!("job {id} finished, exit 0")),
            "no notice for the finished job: {:?}",
            hooks.0
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[cfg(test)]
mod goal_state_tests {
    use super::*;

    fn bare() -> Session {
        let d = std::env::temp_dir().join(format!("imp-goal-state-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::create_dir_all(&d);
        Session::new(
            crate::config::Config::default(),
            d.clone(),
            d.join("out"),
            String::new(),
            Agent::builtin(),
            Vec::new(),
            std::sync::Arc::new(std::sync::atomic::AtomicU16::new(0)),
        )
    }

    #[test]
    fn pausing_keeps_the_goal_and_its_round() {
        let mut s = bare();
        s.start_goal("fix the parser");
        s.goal.as_mut().unwrap().round = 2;
        s.pause_goal();
        let g = s.goal.clone().expect("pausing cleared the goal");
        assert!(g.paused);
        assert_eq!((g.text.as_str(), g.round, g.rounds), ("fix the parser", 2, 5));
    }

    #[test]
    fn resuming_restarts_at_the_saved_round_without_advancing() {
        let mut s = bare();
        s.start_goal("fix the parser");
        s.goal.as_mut().unwrap().round = 2;
        s.pause_goal();
        let msg = s.resume_goal().expect("resume failed");
        assert!(msg.contains("Round 2 of 5"), "{msg}");
        let g = s.goal.clone().unwrap();
        assert!(!g.paused);
        assert_eq!(g.round, 2, "resuming advanced the round");
    }

    #[test]
    fn clear_removes_the_goal_and_resume_without_a_goal_is_an_error() {
        let mut s = bare();
        s.start_goal("x");
        s.clear_goal();
        assert!(s.goal.is_none());
        assert!(s.resume_goal().is_err());
    }

    #[test]
    fn edit_changes_the_text_and_keeps_the_round() {
        let mut s = bare();
        s.start_goal("fix the parser");
        s.goal.as_mut().unwrap().round = 2;
        s.edit_goal("fix the lexer").unwrap();
        let g = s.goal.clone().unwrap();
        assert_eq!((g.text.as_str(), g.round), ("fix the lexer", 2));
    }
}
