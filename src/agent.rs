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
    fn ask(&mut self, question: &str, options: &[String]) -> Option<String>;

    /// Time the last confirmation kept the user, which is not the tool's cost.
    fn last_wait(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }

    /// Run a command attached to the terminal, for prompts that need a tty.
    /// `None` means the front-end cannot hand the terminal over.
    fn run_attached(&mut self, _command: &str) -> Option<i32> {
        None
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
    Error(String),
    /// Output shown to the user with its colours; the model gets text.
    Show(String),
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

pub fn schema() -> Vec<Value> {
    vec![
        json!({"type":"function","function":{"name":"bash","description":"Run a shell command. Output is capped; larger output is written to a file whose path is returned.","parameters":{"type":"object","properties":{"command":{"type":"string","description":"Command line, run with sh -c."},"background":{"type":"boolean","description":"Start it as a job and return an id immediately."},"delay_s":{"type":"integer","description":"Start it as a job after this many seconds, up to 86400."},"notify":{"type":"boolean","description":"With background or delay_s: wake you when the job ends."},"description":{"type":"string","description":"A few words on what it does, shown to the user instead of a long command."},"show":{"type":"boolean","description":"Show the output to the user with its colours, for pictures, charts, and diagrams; you get the text, or only the size of a picture."}},"required":["command"]}}}),
        json!({"type":"function","function":{"name":"job","description":"List background jobs with progress, wait for one, kill one, or check back later: check wakes you after after_s seconds with the job's state, so you can report progress while the user is away.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["list","wait","kill","check"]},"id":{"type":"integer"},"timeout_s":{"type":"integer","description":"For wait; up to 55 seconds."},"after_s":{"type":"integer","description":"For check; 30 to 86400 seconds."}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"read","description":"Read files with line numbers, or list a directory. Add several paths in one call.","parameters":{"type":"object","properties":{"paths":{"type":"array","items":{"type":"string"}},"offset":{"type":"integer"},"limit":{"type":"integer"},"depth":{"type":"integer","description":"Directory levels, 1 to 3. Default 1."}},"required":["paths"]}}}),
        json!({"type":"function","function":{"name":"edit","description":"Replace exact text in a file. Every old string must match exactly once; all edits apply or none do.","parameters":{"type":"object","properties":{"path":{"type":"string"},"edits":{"type":"array","items":{"type":"object","properties":{"old":{"type":"string"},"new":{"type":"string"}},"required":["old","new"]}}},"required":["path","edits"]}}}),
        json!({"type":"function","function":{"name":"write","description":"Create or replace a file.","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}}),
        json!({"type":"function","function":{"name":"fetch","description":"One HTTP request from this machine, including the local network. GET by default; HTML comes back as markdown, feeds as a list, JSON and text as they are. Configured credentials are added for their host by imp; never put keys in headers.","parameters":{"type":"object","properties":{"url":{"type":"string"},"method":{"type":"string","enum":["GET","POST","PUT","PATCH","DELETE"]},"headers":{"type":"object","description":"Extra request headers, name to value."},"body":{"description":"Text, or JSON sent as application/json."}},"required":["url"]}}}),
        json!({"type":"function","function":{"name":"search","description":"Search the web; results are summaries with URLs. Read a page with fetch.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["search"]},"query":{"type":"string"},"count":{"type":"integer"}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"memory","description":"Search stored notes, or save a new one.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["search","save"]},"query":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"skill_load","description":"Load the full instructions of a skill by name.","parameters":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}}),
        json!({"type":"function","function":{"name":"ask","description":"Ask the user a question. Offer options for a choice; omit them for free text. Use only when blocked on a decision you cannot make yourself.","parameters":{"type":"object","properties":{"question":{"type":"string"},"options":{"type":"array","items":{"type":"string"},"description":"Two to six short choices."}},"required":["question"]}}}),
    ]
}

fn diff_detail(name: &str, args: &Value) -> String {
    let mut lines: Vec<String> = Vec::new();
    match name {
        "edit" => {
            for edit in args.get("edits").and_then(Value::as_array).into_iter().flatten() {
                let old = edit.get("old").and_then(Value::as_str).unwrap_or("");
                let new = edit.get("new").and_then(Value::as_str).unwrap_or("");
                for line in old.lines() {
                    lines.push(format!("-{line}"));
                }
                for line in new.lines() {
                    lines.push(format!("+{line}"));
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
    pub last_model: Option<String>,
    pub attachments: Vec<Image>,
    pub transport: Box<dyn Transport>,
    pub memory: Memory,
    pub skills: Vec<Skill>,
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
            last_model: None,
            attachments: Vec::new(),
            transport: Box::new(Live),
            memory,
            skills,
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
        self.resume(messages);
        let agent = log.last_event("agent");
        self.attach_log(log);
        if let Some(name) = agent {
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

    fn say(&mut self, message: Message) {
        if let Some(log) = &self.log {
            log.append(&message);
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
                    if attempt == 1 && !self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        hooks.ui(Ui::Info(format!("retrying after: {e}")));
                        std::thread::sleep(std::time::Duration::from_millis(800));
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
            self.last_model = acc.model.clone();
            match api::classify(status, acc.finish.as_deref(), &raw_error) {
                Ok(()) if status == 200 => return Ok(acc),
                Ok(()) => {}
                Err(f) => {
                    if f.retry == api::Retry::Once && attempt == 1 {
                        hooks.ui(Ui::Info(format!("retrying: {}", f.message)));
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        continue;
                    }
                    return Err(f.message);
                }
            }
            if attempt == 1 && acc.finish.is_none() && acc.content.is_empty() {
                hooks.ui(Ui::Info("empty stream; retrying".into()));
                continue;
            }
            return Ok(acc);
        }
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

    /// Summarise everything except the last four exchanges.
    pub fn compact(&mut self, hooks: &mut dyn Hooks) -> Result<(), String> {
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

        let prompt = format!(
            "Summarise this conversation for another model that must continue the work.\n\
             Cover: decisions made, files touched, open threads, user preferences, unresolved errors.\n\
             Then add up to three lines starting with MEMORY: for durable facts worth saving.\n\
             Be terse. 400 tokens maximum.\n\n{transcript}"
        );
        let summary = self.complete_once(&[
            Message::system("You compress conversations without losing decisions."),
            Message::user(prompt),
        ])?;

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

    fn complete_once(&self, messages: &[Message]) -> Result<String, String> {
        let opts = RequestOptions {
            model: self.cfg.model.clone(),
            max_tokens: 400,
            thinking: false,
            temperature: Some(0.0),
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
            Ok(200) => Ok(acc.content),
            Ok(code) => Err(format!("summariser failed: HTTP {code}")),
            Err(e) => Err(e),
        }
    }

    /// One user turn: stream, run tools, repeat until the model answers.
    pub fn turn(&mut self, user: &str, hooks: &mut dyn Hooks) -> Result<(), String> {
        self.cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        self.note_width();
        self.note_date(crate::prompt::day_number());
        let mut message = Message::user(user);
        message.images = std::mem::take(&mut self.attachments);
        self.say(message);

        let max_steps = self.cfg.max_steps.max(1);
        for step in 0..max_steps {
            if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
                hooks.ui(Ui::Info("cancelled".into()));
                return Ok(());
            }
            self.prune(hooks);
            if crate::api::estimate_tokens(&self.history) >= self.cfg.compact_at as u64 {
                if let Err(e) = self.compact(hooks) {
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
                hooks.ui(Ui::Info(note.clone()));
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
                let out = self.dispatch(&call, hooks);
                self.say(Message::tool(call.id.clone(), out));
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

    fn dispatch(&mut self, call: &ToolCall, hooks: &mut dyn Hooks) -> String {
        let args: Value = match serde_json::from_str(&call.arguments) {
            Ok(v) => v,
            Err(e) => return format!("bad arguments: {e}"),
        };
        hooks.ui(Ui::ToolStart {
            name: call.name.clone(),
            summary: describe(&call.name, &args),
            args: call.arguments.clone(),
        });

        let start = std::time::Instant::now();
        let (ok, out) = self.run_tool(&call.name, &args, hooks);
        // No secret value reaches the conversation, the log, or the screen.
        let out = crate::secrets::redact(&out);
        let spent = start.elapsed().saturating_sub(hooks.last_wait());
        let detail = if ok { diff_detail(&call.name, &args) } else { String::new() };
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
                    other => other,
                };
                // A plain curl GET shares the per-host fetch approval.
                let get = perm::plain_get(cmd).filter(|_| !matches!(decision, Decision::Deny(_)) && named.is_none());
                if let Some(url) = get {
                    if perm::classify_fetch(mode, &url) == Decision::Ask && !hooks.confirm(&perm::fetch_question(&url)) {
                        return (false, format!("refused: the user declined fetching from {}", perm::url_host(&url)));
                    }
                } else {
                    match decision {
                        Decision::Deny(why) => return (false, why),
                        Decision::Ask => {
                            // `always` only for one simple command in normal.
                            let shape = perm::shape(cmd).filter(|_| mode == Permissions::Normal);
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
                            match (answer, shape) {
                                (Answer::No, _) => {
                                    return (false, "refused: the user declined to run this command".into())
                                }
                                (Answer::Session, Some(k)) => {
                                    self.always.insert(k);
                                }
                                (Answer::Always, Some(k)) => {
                                    if let Err(e) = self.cfg.allow_always(&k) {
                                        hooks.ui(Ui::Error(format!("allow {k} not saved: {e}")));
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
                    if let Some(code) = hooks.run_attached(cmd) {
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
                if !self.allowed_write(path, hooks) {
                    return (false, "refused: the user declined this write".into());
                }
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
                if !self.allowed_write(path, hooks) {
                    return (false, "refused: the user declined this edit".into());
                }
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
                            hooks.ui(Ui::Info(format!("job {id} finished, exit {code}")));
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
                let q = args.get("question").and_then(Value::as_str).unwrap_or("");
                let opts: Vec<String> = args
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                if opts.len() > 6 {
                    return (false, "refused: ask accepts at most six options".into());
                }
                match hooks.ask(q, &opts) {
                    Some(answer) if !answer.is_empty() => (true, answer),
                    _ => (false, "refused: the user did not answer".into()),
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
            (Permissions::Normal, "GET") if auth.is_some() => {
                (!self.auth_hosts.contains(&host)).then(|| format!("{what}{with} for this session"))
            }
            (Permissions::Normal, "GET") => {
                (perm::classify_fetch(mode, url) == Decision::Ask).then(|| perm::fetch_question(url))
            }
            (Permissions::Normal, _) => {
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

    fn allowed_write(&mut self, path: &str, hooks: &mut dyn Hooks) -> bool {
        if let Ok(resolved) = self.tools.resolve(path) {
            match crate::perm::self_config(&resolved, self.cfg.live_permissions()) {
                Some(crate::perm::Decision::Deny(why)) => return {
                    hooks.ui(Ui::Error(why));
                    false
                },
                Some(crate::perm::Decision::Ask) => {
                    if !hooks.confirm(&format!("write {path}")) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        let inside = self
            .tools
            .resolve(path)
            .map(|p| p.starts_with(&self.tools.root))
            .unwrap_or(false);
        match perm::classify_write(inside, self.cfg.live_permissions()) {
            Decision::Allow => true,
            Decision::Ask => hooks.confirm(&format!("write {path}")),
            Decision::Deny(_) => false,
        }
    }

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
            format!("edit {} ({edits} changes)", flatten(text("path")))
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
    fn edit_detail_shows_removed_and_added_lines() {
        let args = json!({"path": "x.rs", "edits": [{"old": "a\nb", "new": "c"}]});
        let detail = diff_detail("edit", &args);
        assert!(detail.contains("-a"), "{detail}");
        assert!(detail.contains("-b"), "{detail}");
        assert!(detail.contains("+c"), "{detail}");
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
                "edit a.rs (1 changes)",
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
                if let Ui::Info(t) | Ui::Error(t) = event {
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
        s.turn("save it", &mut Null).unwrap();
        assert!(!memory_dir.exists() || std::fs::read_dir(&memory_dir).unwrap().count() == 0);
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
        s.compact(&mut Null).unwrap();
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
