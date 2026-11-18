//! The turn loop: request, stream, run tools, repeat.

use crate::api::{self, Accum, Message, RequestOptions, SseParser, ToolCall};
use crate::config::Config;
use crate::perm::{self, Decision};
use crate::jobs::Jobs;
use crate::transport::{Live, Transport};
use crate::store::{Agent, Log, Memory, Skill};
use crate::tools::Tools;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

pub trait Hooks {
    fn ui(&mut self, event: Ui);
    fn confirm(&mut self, what: &str) -> bool;
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
        ms: u128,
    },
    Info(String),
    Error(String),
}

const MAX_STEPS: usize = 24;

pub fn schema() -> Vec<Value> {
    vec![
        json!({"type":"function","function":{"name":"bash","description":"Run a shell command. Output is capped; larger output is written to a file whose path is returned.","parameters":{"type":"object","properties":{"command":{"type":"string","description":"Command line, run with sh -c."},"background":{"type":"boolean","description":"Start it as a job and return an id immediately."}},"required":["command"]}}}),
        json!({"type":"function","function":{"name":"job","description":"List background jobs, wait for one, or kill one.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["list","wait","kill"]},"id":{"type":"integer"},"timeout_s":{"type":"integer","description":"For wait; up to 55 seconds."}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"read","description":"Read one or more files with line numbers.","parameters":{"type":"object","properties":{"paths":{"type":"array","items":{"type":"string"}},"offset":{"type":"integer"},"limit":{"type":"integer"}},"required":["paths"]}}}),
        json!({"type":"function","function":{"name":"edit","description":"Replace exact text in a file. Every old string must match exactly once; all edits apply or none do.","parameters":{"type":"object","properties":{"path":{"type":"string"},"edits":{"type":"array","items":{"type":"object","properties":{"old":{"type":"string"},"new":{"type":"string"}},"required":["old","new"]}}},"required":["path","edits"]}}}),
        json!({"type":"function","function":{"name":"write","description":"Create or replace a file.","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}}),
        json!({"type":"function","function":{"name":"search","description":"Search the web, or fetch a page as markdown.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["search","fetch"]},"query":{"type":"string"},"url":{"type":"string"},"count":{"type":"integer"}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"memory","description":"Search stored notes, or save a new one.","parameters":{"type":"object","properties":{"op":{"type":"string","enum":["search","save"]},"query":{"type":"string"},"title":{"type":"string"},"body":{"type":"string"}},"required":["op"]}}}),
        json!({"type":"function","function":{"name":"skill_load","description":"Load the full instructions of a skill by name.","parameters":{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}}}),
        json!({"type":"function","function":{"name":"ask","description":"Ask the user a question. Offer options for a choice; omit them for free text. Use only when blocked on a decision you cannot make yourself.","parameters":{"type":"object","properties":{"question":{"type":"string"},"options":{"type":"array","items":{"type":"string"},"description":"Two to six short choices."}},"required":["question"]}}}),
    ]
}

pub struct Session {
    pub cfg: Config,
    pub tools: Tools,
    pub history: Vec<Message>,
    pub cancel: std::sync::Arc<AtomicBool>,
    pub cols: std::sync::Arc<std::sync::atomic::AtomicU16>,
    last_cols: Option<u16>,
    pub agent: Agent,
    pub roster: Vec<Agent>,
    pub jobs: Jobs,
    pub job_dir: std::path::PathBuf,
    pub log: Option<Log>,
    pub last_usage: (u64, u64, u64),
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
        let jail = crate::perm::jail_enabled(cfg.permissions);
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
            agent,
            roster,
            jobs: Jobs::default(),
            job_dir,
            log: None,
            last_usage: (0, 0, 0),
            transport: Box::new(Live),
            memory,
            skills,
        }
    }

    pub fn attach_log(&mut self, log: Log) {
        self.log = Some(log);
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
            let body = api::build_request(&opts, &self.history, &tools);
            let url = format!("{}/chat/completions", self.cfg.base_url);
            let auth = format!("Authorization: Bearer {}", self.cfg.api_key);
            let headers = ["Content-Type: application/json", auth.as_str(), "Accept: text/event-stream"];

            let mut parser = SseParser::default();
            let mut acc = Accum::default();
            let status;
            let mut raw_error = String::new();
            let mut raw = Vec::new();

            let result = self.transport.post(
                &url,
                &headers,
                body.to_string().as_bytes(),
                &mut |bytes| {
                    if raw.len() < 4096 {
                        raw.extend_from_slice(bytes);
                    }
                    for ev in parser.push(bytes) {
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
                    if attempt == 1 {
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

            self.last_usage = (acc.cache_hit, acc.cache_miss, acc.completion_tokens);
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
            if hooks.confirm(&format!("save note `{title}`")) {
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
        let body = api::build_request(&opts, messages, &[]);
        let url = format!("{}/chat/completions", self.cfg.base_url);
        let auth = format!("Authorization: Bearer {}", self.cfg.api_key);
        let headers = ["Content-Type: application/json", auth.as_str()];
        let mut parser = SseParser::default();
        let mut acc = Accum::default();
        let result = self.transport.post(&url, &headers, body.to_string().as_bytes(), &mut |b| {
            for ev in parser.push(b) {
                acc.apply(ev);
            }
        });
        match result {
            Ok(200) => Ok(acc.content),
            Ok(code) => Err(format!("summariser failed: HTTP {code}")),
            Err(e) => Err(e),
        }
    }

    /// One user turn: stream, run tools, repeat until the model answers.
    pub fn turn(&mut self, user: &str, hooks: &mut dyn Hooks) -> Result<(), String> {
        self.cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        self.say(Message::user(user));

        for step in 0..MAX_STEPS {
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
                let note = format!(
                    "Background job {id} finished with exit {code}; output: {}",
                    path.display()
                );
                hooks.ui(Ui::Info(note.clone()));
                self.say(Message::system(note));
            }
            self.cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let acc = self.request(hooks)?;

            let mut assistant = Message::assistant(acc.content.clone());
            assistant.reasoning = acc.reasoning.clone();
            assistant.tool_calls = acc.tool_calls();
            let calls = assistant.tool_calls.clone();
            self.say(assistant);

            if calls.is_empty() {
                return Ok(());
            }
            if step == MAX_STEPS - 1 {
                let msg = format!("stopped after {MAX_STEPS} tool steps");
                hooks.ui(Ui::Error(msg.clone()));
                return Err(msg);
            }

            for call in calls {
                let out = self.dispatch(&call, hooks);
                self.say(Message::tool(call.id.clone(), out));
            }
        }
        Ok(())
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
        let spent = start.elapsed().saturating_sub(hooks.last_wait());
        hooks.ui(Ui::ToolEnd {
            ok,
            preview: summarise(&out),
            output: out.clone(),
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
                    perm::shell_touches_self_config(cmd, self.cfg.permissions)
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
                if cmd.split_whitespace().next() == Some("sudo") {
                    if let Some(code) = hooks.run_attached(cmd) {
                        return (code == 0, format!("exit {code}\n(ran attached to the terminal)"));
                    }
                }
                match perm::classify_command(cmd, self.cfg.permissions, self.cfg.allow_sudo) {
                    Decision::Deny(why) => return (false, why),
                    Decision::Ask => {
                        if !hooks.confirm(&format!("run `{cmd}`")) {
                            return (false, "refused: the user declined to run this command".into());
                        }
                    }
                    Decision::Allow => {}
                }
                if args.get("background").and_then(Value::as_bool).unwrap_or(false) {
                    return match self.jobs.start(cmd, &self.job_dir) {
                        Ok(id) => {
                            let _ = self.jobs.poll();
                            (
                                true,
                                format!(
                                    "job {id} started in the background; output: {}",
                                    self.job_dir.join(format!("job-{id}.log")).display()
                                ),
                            )
                        }
                        Err(e) => (false, format!("refused: could not start job: {e}")),
                    };
                }
                let o = self.tools.bash(cmd, &self.cancel);
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
                let o = self.tools.read(&paths, offset, limit);
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
                            hooks.ui(Ui::Info(format!("job {id} finished, exit {code}")));
                        }
                        (true, self.jobs.list())
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
                            (
                                true,
                                self.jobs
                                    .wait(id, std::time::Duration::from_secs(secs))
                                    .unwrap_or_else(|e| e),
                            )
                        }
                        None => (false, "refused: job wait needs an id".into()),
                    },
                    other => (false, format!("refused: unknown job op {other}")),
                }
            }
            "search" => (true, self.search_tool(args)),
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
                        if !hooks.confirm(&format!("save note {target}")) {
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
            "fetch" => format!("{}/v1/tools/fetch", self.cfg.base_url),
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
            _ => {
                let u = args.get("url").and_then(Value::as_str).unwrap_or("");
                if u.is_empty() {
                    return "fetch needs a url".into();
                }
                serde_json::json!({ "url": u })
            }
        };

        let auth = format!("Authorization: Bearer {}", self.cfg.api_key);
        let headers = ["Content-Type: application/json", auth.as_str()];
        let mut raw = Vec::new();
        let resp = self.transport.post(&url, &headers, body.to_string().as_bytes(), &mut |b| {
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
            _ => {
                let title = v["title"].as_str().unwrap_or("");
                let content = v["content"].as_str().unwrap_or("");
                if content.is_empty() {
                    return "the page had no readable content".into();
                }
                let outline = outline_of(content);
                let packed = format!("{title}\n\n{content}");
                format!("outline: {outline}\n\n{}", self.tools.cap(&packed))
            }
        }
    }

    fn allowed_write(&mut self, path: &str, hooks: &mut dyn Hooks) -> bool {
        if let Ok(resolved) = self.tools.resolve(path) {
            match crate::perm::self_config(&resolved, self.cfg.permissions) {
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
        match perm::classify_write(inside, self.cfg.permissions) {
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

fn outline_of(markdown: &str) -> String {
    let mut heads: Vec<&str> = markdown
        .lines()
        .filter(|l| l.starts_with('#'))
        .map(|l| l.trim_start_matches('#').trim())
        .filter(|l| !l.is_empty())
        .take(12)
        .collect();
    if heads.is_empty() {
        return "(no headings)".into();
    }
    heads.dedup();
    heads.join(" · ")
}

pub fn summarise(text: &str) -> String {
    let flat = text.replace('\n', " ");
    let mut s: String = flat.chars().take(120).collect();
    if flat.chars().count() > 120 {
        s.push('…');
    }
    s
}

/// One short line for a tool call, in place of its JSON arguments.
pub fn describe(name: &str, args: &Value) -> String {
    let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or("");
    let lines = |key: &str| text(key).lines().count();
    match name {
        "bash" => format!("run `{}`", summarise(text("command"))),
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
            format!("read {}", summarise(&paths.join(", ")))
        }
        "edit" => {
            let edits = args
                .get("edits")
                .and_then(Value::as_array)
                .map(|a| a.len())
                .unwrap_or(0);
            format!("edit {} ({edits} changes)", summarise(text("path")))
        }
        "write" => format!(
            "write {} ({} lines)",
            summarise(text("path")),
            lines("content")
        ),
        "search" => {
            if text("op") == "fetch" {
                format!("fetch {}", summarise(text("url")))
            } else {
                format!("search {:?}", summarise(text("query")))
            }
        }
        "memory" => {
            let what = if text("title").is_empty() {
                text("query")
            } else {
                text("title")
            };
            format!("memory {} {:?}", text("op"), summarise(what))
        }
        "skill_load" => format!("load skill {}", summarise(text("name"))),
        "ask" => format!("ask {:?}", summarise(text("question"))),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Agent, Log};

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
            (
                "search",
                json!({"op": "fetch", "url": "https://x/y.md"}),
                "fetch https://x/y.md",
            ),
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
        s.cfg.permissions = crate::config::Permissions::Strict;

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
        let body = api::build_request(&opts, &history, &[]);
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
}
