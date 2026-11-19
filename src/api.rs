//! Wire types, request building, SSE parsing, and token estimation.
//!
//! The API facts this encodes are in design.md: reasoning must round-trip when
//! tools are present, tool-call deltas are keyed by index, and the final chunk
//! carries usage.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub file: String,
    pub name: String,
    pub mime: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub reasoning: String,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
    pub images: Vec<Image>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, text)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, text)
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, text)
    }

    pub fn tool(call_id: impl Into<String>, text: impl Into<String>) -> Self {
        let mut m = Self::new(Role::Tool, text);
        m.tool_call_id = Some(call_id.into());
        m
    }

    fn new(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: text.into(),
            reasoning: String::new(),
            tool_calls: Vec::new(),
            tool_call_id: None,
            images: Vec::new(),
        }
    }

    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("role".into(), json!(self.role.as_str()));
        if self.images.is_empty() {
            map.insert("content".into(), json!(self.content));
        } else {
            let mut parts = vec![json!({"type": "text", "text": self.content})];
            for image in &self.images {
                let path = crate::images::dir().join(&image.file);
                if let Ok(bytes) = std::fs::read(&path) {
                    parts.push(json!({
                        "type": "image_url",
                        "image_url": { "url": format!("data:{};base64,{}", image.mime, base64(&bytes)) }
                    }));
                }
            }
            map.insert("content".into(), Value::Array(parts));
        }
        if !self.reasoning.is_empty() && self.role == Role::Assistant {
            // Required whenever tools are present; omitting it is a 400.
            map.insert("reasoning_content".into(), json!(self.reasoning));
        }
        if let Some(id) = &self.tool_call_id {
            map.insert("tool_call_id".into(), json!(id));
        }
        if !self.tool_calls.is_empty() {
            let calls: Vec<Value> = self
                .tool_calls
                .iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "type": "function",
                        "function": { "name": c.name, "arguments": c.arguments }
                    })
                })
                .collect();
            map.insert("tool_calls".into(), Value::Array(calls));
        }
        if !self.images.is_empty() {
            let images: Vec<Value> = self
                .images
                .iter()
                .map(|i| json!({"file": i.file, "name": i.name, "mime": i.mime, "bytes": i.bytes}))
                .collect();
            map.insert("images".into(), Value::Array(images));
        }
        Value::Object(map)
    }

    pub fn from_json(v: &Value) -> Option<Self> {
        let role = match v.get("role")?.as_str()? {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => return None,
        };
        let content = match v.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Array(parts)) => parts
                .iter()
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        };
        let mut msg = Self::new(role, content);
        if let Some(images) = v.get("images").and_then(Value::as_array) {
            for image in images {
                let file = image.get("file").and_then(Value::as_str)?;
                let mime = image.get("mime").and_then(Value::as_str)?;
                msg.images.push(Image {
                    file: file.to_string(),
                    name: image
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or(file)
                        .to_string(),
                    mime: mime.to_string(),
                    bytes: image.get("bytes").and_then(Value::as_u64).unwrap_or(0),
                });
            }
        }
        msg.reasoning = v
            .get("reasoning_content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        msg.tool_call_id = v
            .get("tool_call_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(calls) = v.get("tool_calls").and_then(Value::as_array) {
            for c in calls {
                let Some(f) = c.get("function") else { continue };
                msg.tool_calls.push(ToolCall {
                    id: c.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                    name: f.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                    arguments: f
                        .get("arguments")
                        .and_then(Value::as_str)
                        .unwrap_or("{}")
                        .to_string(),
                });
            }
        }
        Some(msg)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retry {
    Once,
    Never,
}

#[derive(Debug, Clone)]
pub struct Failure {
    pub message: String,
    pub retry: Retry,
}

impl Failure {
    pub fn terminal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry: Retry::Never,
        }
    }

    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry: Retry::Once,
        }
    }
}

/// Map a finish reason plus HTTP status to an outcome.
pub fn classify(status: u16, finish: Option<&str>, body_hint: &str) -> Result<(), Failure> {
    if status == 429 || (500..600).contains(&status) || status == 408 {
        return Err(Failure::retryable(format!("HTTP {status}: {body_hint}")));
    }
    if status != 200 {
        return Err(Failure::terminal(format!("HTTP {status}: {body_hint}")));
    }
    match finish {
        None | Some("stop") | Some("tool_calls") => Ok(()),
        Some("aborted") | Some("insufficient_system_resource") => {
            Err(Failure::retryable(format!("upstream: {finish:?}")))
        }
        Some("length") => Err(Failure::terminal(
            "the reply hit the output limit; retry with a larger max_tokens",
        )),
        Some("content_filter") => Err(Failure::terminal("the response was filtered")),
        Some(other) => Err(Failure::terminal(format!("unknown finish reason {other:?}"))),
    }
}

/// One SSE event, after parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Content(String),
    Reasoning(String),
    ToolCall {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    Finish(String),
    Model(String),
    Usage {
        prompt: u64,
        completion: u64,
        cache_hit: u64,
        cache_miss: u64,
    },
    Done,
}

/// Incremental SSE splitter. Feed it bytes and an adapter, get events back.
#[derive(Default)]
pub struct SseParser {
    buf: Vec<u8>,
}

impl SseParser {
    pub fn push(&mut self, bytes: &[u8], adapter: &mut dyn crate::providers::Adapter) -> Vec<Event> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&line);
            let text = text.trim_end_matches(['\r', '\n']);
            if text.is_empty() || text.starts_with(':') {
                continue;
            }
            let Some(payload) = text.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload == "[DONE]" {
                out.push(Event::Done);
                continue;
            }
            out.extend(adapter.event(payload));
        }
        out
    }
}

/// Assembles one streamed response.
#[derive(Debug, Default)]
pub struct Accum {
    pub content: String,
    pub reasoning: String,
    pub calls: BTreeMap<usize, ToolCall>,
    pub finish: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_hit: u64,
    pub cache_miss: u64,
}

impl Accum {
    pub fn apply(&mut self, ev: Event) {
        match ev {
            Event::Content(t) => self.content.push_str(&t),
            Event::Reasoning(t) => self.reasoning.push_str(&t),
            Event::ToolCall {
                index,
                id,
                name,
                arguments,
            } => {
                let entry = self.calls.entry(index).or_default();
                if let Some(id) = id {
                    entry.id = id;
                }
                if let Some(name) = name {
                    entry.name = name;
                }
                if let Some(args) = arguments {
                    entry.arguments.push_str(&args);
                }
            }
            Event::Finish(f) => self.finish = Some(f),
            Event::Model(m) => self.model = Some(m),
            Event::Usage {
                prompt,
                completion,
                cache_hit,
                cache_miss,
            } => {
                self.prompt_tokens = prompt;
                self.completion_tokens = completion;
                self.cache_hit = cache_hit;
                self.cache_miss = cache_miss;
            }
            Event::Done => {}
        }
    }

    /// Calls in index order; arguments default to `{}` when the model sent none.
    pub fn tool_calls(&self) -> Vec<ToolCall> {
        self.calls
            .values()
            .cloned()
            .map(|mut c| {
                if c.arguments.trim().is_empty() {
                    c.arguments = "{}".into();
                }
                c
            })
            .collect()
    }
}

pub struct RequestOptions {
    pub model: String,
    pub max_tokens: u32,
    pub thinking: bool,
    pub temperature: Option<f32>,
}

/// Rough wire-size estimate: four bytes per token plus per-message overhead.
pub fn estimate_tokens(history: &[Message]) -> u64 {
    let mut bytes = 0usize;
    for m in history {
        bytes += m.content.len() + m.reasoning.len() + 24;
        for c in &m.tool_calls {
            bytes += c.name.len() + c.arguments.len() + 24;
        }
    }
    (bytes / 4) as u64
}

pub(crate) fn base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(parts: &[&str]) -> Accum {
        let mut adapter = crate::providers::adapter(
            crate::providers::Provider::DeepSeek,
            "https://api.deepseek.com",
            "",
        );
        let mut p = SseParser::default();
        let mut a = Accum::default();
        for part in parts {
            for ev in p.push(part.as_bytes(), adapter.as_mut()) {
                a.apply(ev);
            }
        }
        a
    }

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
    }

    #[test]
    fn image_records_round_trip() {
        let v = serde_json::json!({
            "role": "user",
            "content": "look",
            "images": [{"file": "abc.png", "name": "shot.png", "mime": "image/png", "bytes": 24}]
        });
        let message = Message::from_json(&v).unwrap();
        assert_eq!(message.content, "look");
        assert_eq!(message.images.len(), 1);
        assert_eq!(message.images[0].name, "shot.png");
        assert_eq!(message.images[0].mime, "image/png");
    }

    #[test]
    fn served_model_is_kept() {
        let a = feed(&[
            "data: {\"model\":\"deepseek-flash-2026-11\",\"choices\":[]}\n\n",
        ]);
        assert_eq!(a.model.as_deref(), Some("deepseek-flash-2026-11"));
    }

    #[test]
    fn plain_text_stream() {
        let a = feed(&[
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2,\"prompt_cache_hit_tokens\":5,\"prompt_cache_miss_tokens\":2}}\n\n",
            "data: [DONE]\n\n",
        ]);
        assert_eq!(a.content, "Hello world");
        assert_eq!(a.finish.as_deref(), Some("stop"));
        assert_eq!(a.prompt_tokens, 7);
        assert_eq!(a.cache_hit, 5);
    }

    #[test]
    fn split_across_chunks() {
        let a = feed(&[
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hel",
            "lo\"},\"finish_reason\":null}]}\n\n",
        ]);
        assert_eq!(a.content, "Hello");
    }

    #[test]
    fn tool_call_arguments_accumulate_by_index() {
        let a = feed(&[
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"bash\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"\",\"arguments\":\"{\\\"command\\\":\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"ls\\\"}\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
        ]);
        let calls = a.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].arguments, "{\"command\":\"ls\"}");
        assert_eq!(a.finish.as_deref(), Some("tool_calls"));
    }

    #[test]
    fn parallel_calls_stay_separate() {
        let a = feed(&[
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"a\",\"function\":{\"name\":\"read\",\"arguments\":\"{}\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":1,\"id\":\"b\",\"function\":{\"name\":\"bash\",\"arguments\":\"{}\"}}]},\"finish_reason\":null}]}\n\n",
        ]);
        let calls = a.tool_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "read");
        assert_eq!(calls[1].name, "bash");
    }

    #[test]
    fn keep_alive_comments_are_ignored() {
        let a = feed(&[": keep-alive\n\n", "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"x\"},\"finish_reason\":null}]}\n\n"]);
        assert_eq!(a.content, "x");
    }

    #[test]
    fn reasoning_is_separate() {
        let a = feed(&[
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"think\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"answer\"},\"finish_reason\":null}]}\n\n",
        ]);
        assert_eq!(a.reasoning, "think");
        assert_eq!(a.content, "answer");
    }

    #[test]
    fn classification_maps_failures() {
        assert!(classify(200, Some("stop"), "").is_ok());
        assert_eq!(classify(429, None, "slow").unwrap_err().retry, Retry::Once);
        assert_eq!(
            classify(200, Some("insufficient_system_resource"), "").unwrap_err().retry,
            Retry::Once
        );
        assert_eq!(
            classify(200, Some("length"), "").unwrap_err().retry,
            Retry::Never
        );
        assert!(classify(400, None, "bad").is_err());
    }

    #[test]
    fn assistant_json_keeps_reasoning_and_calls() {
        let mut m = Message::assistant("hi");
        m.reasoning = "because".into();
        m.tool_calls.push(ToolCall {
            id: "c1".into(),
            name: "bash".into(),
            arguments: "{}".into(),
        });
        let v = m.to_json();
        assert_eq!(v["reasoning_content"], "because");
        assert_eq!(v["tool_calls"][0]["function"]["name"], "bash");
        let back = Message::from_json(&v).unwrap();
        assert_eq!(back.reasoning, "because");
        assert_eq!(back.tool_calls[0].id, "c1");
    }

    #[test]
    fn tool_message_has_call_id() {
        let v = Message::tool("c1", "ok").to_json();
        assert_eq!(v["role"], "tool");
        assert_eq!(v["tool_call_id"], "c1");
    }

    #[test]
    fn thinking_off_omits_temperature_when_none() {
        let opts = RequestOptions {
            model: "deepseek-flash".into(),
            max_tokens: 10,
            thinking: true,
            temperature: Some(0.2),
        };
        let mut adapter = crate::providers::adapter(
            crate::providers::Provider::DeepSeek,
            "https://api.deepseek.com",
            "k",
        );
        let wire = adapter.wire(&opts, &[Message::user("hi")], &[]);
        let body = wire.body;
        assert_eq!(body["thinking"]["type"], "enabled");
        assert!(body.get("temperature").is_none());
        assert!(body.get("tools").is_none());
        assert_eq!(wire.url, "https://api.deepseek.com/chat/completions");
        assert!(wire.headers.iter().any(|h| h == "Authorization: Bearer k"));
    }

    #[test]
    fn estimate_grows_with_content() {
        let small = estimate_tokens(&[Message::user("hi")]);
        let big = estimate_tokens(&[Message::user("x".repeat(4000))]);
        assert!(big > small + 900);
    }
}
