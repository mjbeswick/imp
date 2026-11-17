//! The turn loop: request, stream, run tools, repeat.

use crate::api::{self, Accum, Message, RequestOptions, SseParser, ToolCall};
use crate::config::Config;
use crate::perm::{self, Decision};
use crate::tools::Tools;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

pub enum Ui {
    Content(String),
    Reasoning(String),
    ToolStart { name: String, args: String },
    ToolEnd { ok: bool, ms: u128, preview: String },
    Info(String),
    Error(String),
}

const MAX_STEPS: usize = 24;

pub fn schema() -> Vec<Value> {
    vec![
        json!({"type":"function","function":{"name":"bash","description":"Run a shell command. Output is capped; larger output is written to a file whose path is returned.","parameters":{"type":"object","properties":{"command":{"type":"string","description":"Command line, run with sh -c."}},"required":["command"]}}}),
        json!({"type":"function","function":{"name":"read","description":"Read one or more files with line numbers.","parameters":{"type":"object","properties":{"paths":{"type":"array","items":{"type":"string"}},"offset":{"type":"integer"},"limit":{"type":"integer"}},"required":["paths"]}}}),
        json!({"type":"function","function":{"name":"edit","description":"Replace exact text in a file. Every old string must match exactly once; all edits apply or none do.","parameters":{"type":"object","properties":{"path":{"type":"string"},"edits":{"type":"array","items":{"type":"object","properties":{"old":{"type":"string"},"new":{"type":"string"}},"required":["old","new"]}}},"required":["path","edits"]}}}),
        json!({"type":"function","function":{"name":"write","description":"Create or replace a file.","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}}),
        json!({"type":"function","function":{"name":"ask","description":"Ask the user a question. Offer options for a choice; omit them for free text. Use only when blocked on a decision you cannot make yourself.","parameters":{"type":"object","properties":{"question":{"type":"string"},"options":{"type":"array","items":{"type":"string"},"description":"Two to six short choices."}},"required":["question"]}}}),
    ]
}

pub struct Session {
    pub cfg: Config,
    pub tools: Tools,
    pub history: Vec<Message>,
    pub cancel: AtomicBool,
}

impl Session {
    pub fn new(cfg: Config, root: std::path::PathBuf, out_dir: std::path::PathBuf, system: String) -> Self {
        let mut history = Vec::new();
        if !system.is_empty() {
            history.push(Message::system(system));
        }
        Self {
            cfg,
            tools: Tools::new(root, out_dir),
            history,
            cancel: AtomicBool::new(false),
        }
    }

    fn request(&mut self, ui: &mut dyn FnMut(Ui)) -> Result<Accum, String> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let opts = RequestOptions {
                model: self.cfg.model.clone(),
                max_tokens: self.cfg.max_tokens,
                thinking: self.cfg.thinking,
                temperature: Some(0.2),
            };
            let body = api::build_request(&opts, &self.history, &schema());
            let url = format!("{}/chat/completions", self.cfg.base_url);
            let auth = format!("Authorization: Bearer {}", self.cfg.api_key);
            let headers = ["Content-Type: application/json", auth.as_str(), "Accept: text/event-stream"];

            let mut parser = SseParser::default();
            let mut acc = Accum::default();
            let status;
            let mut raw_error = String::new();
            let mut raw = Vec::new();

            let result = crate::http::post_stream(
                &url,
                &headers,
                body.to_string().as_bytes(),
                |bytes| {
                    if raw.len() < 4096 {
                        raw.extend_from_slice(bytes);
                    }
                    for ev in parser.push(bytes) {
                        match &ev {
                            api::Event::Content(t) => ui(Ui::Content(t.clone())),
                            api::Event::Reasoning(t) => ui(Ui::Reasoning(t.clone())),
                            _ => {}
                        }
                        acc.apply(ev);
                    }
                },
            );

            match result {
                Ok(r) if r.status == 200 => {
                    status = r.status;
                }
                Ok(r) => {
                    status = r.status;
                    let body = String::from_utf8_lossy(&raw).to_string();
                    raw_error = body.chars().take(300).collect();
                }
                Err(e) => {
                    if attempt == 1 {
                        ui(Ui::Info(format!("retrying after: {e}")));
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        continue;
                    }
                    return Err(e);
                }
            }

            match api::classify(status, acc.finish.as_deref(), &raw_error) {
                Ok(()) if status == 200 => return Ok(acc),
                Ok(()) => {}
                Err(f) => {
                    if f.retry == api::Retry::Once && attempt == 1 {
                        ui(Ui::Info(format!("retrying: {}", f.message)));
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        continue;
                    }
                    return Err(f.message);
                }
            }
            if attempt == 1 && acc.finish.is_none() && acc.content.is_empty() {
                ui(Ui::Info("empty stream; retrying".into()));
                continue;
            }
            return Ok(acc);
        }
    }

    /// One user turn: stream, run tools, repeat until the model answers.
    pub fn turn(
        &mut self,
        user: &str,
        ui: &mut dyn FnMut(Ui),
        confirm: &mut dyn FnMut(&str) -> bool,
    ) -> Result<(), String> {
        self.history.push(Message::user(user));

        for step in 0..MAX_STEPS {
            self.cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            let acc = self.request(ui)?;

            let mut assistant = Message::assistant(acc.content.clone());
            assistant.reasoning = acc.reasoning.clone();
            assistant.tool_calls = acc.tool_calls();
            let calls = assistant.tool_calls.clone();
            self.history.push(assistant);

            if calls.is_empty() {
                return Ok(());
            }
            if step == MAX_STEPS - 1 {
                let msg = format!("stopped after {MAX_STEPS} tool steps");
                ui(Ui::Error(msg.clone()));
                return Err(msg);
            }

            for call in calls {
                let out = self.dispatch(&call, ui, confirm);
                self.history.push(Message::tool(call.id.clone(), out));
            }
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        call: &ToolCall,
        ui: &mut dyn FnMut(Ui),
        confirm: &mut dyn FnMut(&str) -> bool,
    ) -> String {
        let args: Value = match serde_json::from_str(&call.arguments) {
            Ok(v) => v,
            Err(e) => return format!("bad arguments: {e}"),
        };
        ui(Ui::ToolStart {
            name: call.name.clone(),
            args: summarise(&call.arguments),
        });

        let out = self.run_tool(&call.name, &args, ui, confirm);
        ui(Ui::ToolEnd {
            ok: !out.starts_with("refused")
                && !out.starts_with("cancelled")
                && !out.starts_with("bad arguments"),
            ms: 0,
            preview: summarise(&out),
        });
        out
    }

    fn run_tool(
        &mut self,
        name: &str,
        args: &Value,
        ui: &mut dyn FnMut(Ui),
        confirm: &mut dyn FnMut(&str) -> bool,
    ) -> String {
        match name {
            "bash" => {
                let Some(cmd) = args.get("command").and_then(Value::as_str) else {
                    return "bash needs a command".into();
                };
                match perm::classify_command(cmd, self.cfg.permissions, self.cfg.allow_sudo) {
                    Decision::Deny(why) => return why,
                    Decision::Ask => {
                        if !confirm(cmd) {
                            return "the user declined to run this command".into();
                        }
                    }
                    Decision::Allow => {}
                }
                let o = self.tools.bash(cmd, &self.cancel);
                format!("{}exit {}\n{}", if o.ok { "" } else { "" }, o.code.unwrap_or(-1), o.text)
            }
            "read" => {
                let paths: Vec<String> = args
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                if paths.is_empty() {
                    return "read needs at least one path".into();
                }
                let offset = args.get("offset").and_then(Value::as_u64).map(|n| n as usize);
                let limit = args.get("limit").and_then(Value::as_u64).map(|n| n as usize);
                self.tools.read(&paths, offset, limit).text
            }
            "write" => {
                let path = args.get("path").and_then(Value::as_str).unwrap_or("");
                let content = args.get("content").and_then(Value::as_str).unwrap_or("");
                if !self.allowed_write(path, confirm) {
                    return "the user declined this write".into();
                }
                self.tools.write(path, content).text
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
                    return "edit needs at least one old/new pair".into();
                }
                if !self.allowed_write(path, confirm) {
                    return "the user declined this edit".into();
                }
                self.tools.edit(path, &pairs).text
            }
            "ask" => {
                let q = args.get("question").and_then(Value::as_str).unwrap_or("");
                let opts: Vec<String> = args
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                if opts.len() > 6 {
                    return "ask accepts at most six options".into();
                }
                ui(Ui::Info(format!("question: {q}")));
                if confirm(q) {
                    "the user answered in the interface".into()
                } else {
                    "the user did not answer".into()
                }
            }
            other => format!("unknown tool {other}"),
        }
    }

    fn allowed_write(&mut self, path: &str, confirm: &mut dyn FnMut(&str) -> bool) -> bool {
        let inside = self
            .tools
            .resolve(path)
            .map(|p| p.starts_with(&self.tools.root))
            .unwrap_or(false);
        match perm::classify_write(inside, self.cfg.permissions) {
            Decision::Allow => true,
            Decision::Ask => confirm(&format!("write {path}")),
            Decision::Deny(_) => false,
        }
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
