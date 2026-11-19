//! Anthropic messages and SSE.
//!
//! System text moves to the top-level `system` field, tools become
//! `input_schema` definitions, tool results become `tool_result` blocks, and
//! thinking deltas arrive as reasoning rather than answer text.

use crate::api::{Event, Message, RequestOptions, Role};
use crate::providers::{Adapter, Wire};
use serde_json::{json, Value};

pub struct Anthropic {
    base_url: String,
    key: String,
    prompt: u64,
    cache_hit: u64,
    cache_miss: u64,
    calls: usize,
}

impl Anthropic {
    pub fn new(base_url: &str, key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            key: key.to_string(),
            prompt: 0,
            cache_hit: 0,
            cache_miss: 0,
            calls: 0,
        }
    }
}

impl Adapter for Anthropic {
    fn wire(&mut self, opts: &RequestOptions, history: &[Message], tools: &[Value]) -> Wire {
        self.prompt = 0;
        self.cache_hit = 0;
        self.cache_miss = 0;
        self.calls = 0;

        let (system, messages) = translate(history);
        let mut body = json!({
            "model": opts.model,
            "max_tokens": opts.max_tokens,
            "messages": messages,
            "stream": true,
        });
        let map = body.as_object_mut().expect("object");
        if let Some(system) = system {
            map.insert("system".into(), json!(system));
        }
        if let Some(t) = opts.temperature {
            map.insert("temperature".into(), json!(t));
        }
        let defs = tool_defs(tools);
        if !defs.is_empty() {
            map.insert("tools".into(), Value::Array(defs));
        }

        Wire {
            url: format!("{}/v1/messages", self.base_url),
            headers: vec![
                "Content-Type: application/json".into(),
                format!("x-api-key: {}", self.key),
                "anthropic-version: 2023-06-01".into(),
                "Accept: text/event-stream".into(),
            ],
            body,
        }
    }

    fn event(&mut self, payload: &str) -> Vec<Event> {
        let mut out = Vec::new();
        let Ok(v) = serde_json::from_str::<Value>(payload) else {
            return out;
        };
        match v.get("type").and_then(Value::as_str).unwrap_or("") {
            "message_start" => {
                let Some(message) = v.get("message") else {
                    return out;
                };
                if let Some(model) = message.get("model").and_then(Value::as_str) {
                    out.push(Event::Model(model.to_string()));
                }
                if let Some(u) = message.get("usage") {
                    let input = count(u, "input_tokens");
                    let created = count(u, "cache_creation_input_tokens");
                    self.cache_hit = count(u, "cache_read_input_tokens");
                    self.cache_miss = input + created;
                    self.prompt = input + created + self.cache_hit;
                }
            }
            "content_block_start" => {
                let Some(block) = v.get("content_block") else {
                    return out;
                };
                if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    self.calls += 1;
                    out.push(Event::ToolCall {
                        index: index(&v),
                        id: text(block, "id"),
                        name: text(block, "name"),
                        arguments: Some(String::new()),
                    });
                }
            }
            "content_block_delta" => {
                let Some(delta) = v.get("delta") else {
                    return out;
                };
                match delta.get("type").and_then(Value::as_str) {
                    Some("text_delta") => {
                        if let Some(t) = delta.get("text").and_then(Value::as_str) {
                            if !t.is_empty() {
                                out.push(Event::Content(t.to_string()));
                            }
                        }
                    }
                    Some("thinking_delta") => {
                        if let Some(t) = delta.get("thinking").and_then(Value::as_str) {
                            if !t.is_empty() {
                                out.push(Event::Reasoning(t.to_string()));
                            }
                        }
                    }
                    Some("input_json_delta") => {
                        if let Some(t) = delta.get("partial_json").and_then(Value::as_str) {
                            out.push(Event::ToolCall {
                                index: index(&v),
                                id: None,
                                name: None,
                                arguments: Some(t.to_string()),
                            });
                        }
                    }
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(reason) = v
                    .get("delta")
                    .and_then(|d| d.get("stop_reason"))
                    .and_then(Value::as_str)
                {
                    out.push(Event::Finish(finish(reason).to_string()));
                }
                if let Some(completion) = v
                    .get("usage")
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(Value::as_u64)
                {
                    out.push(Event::Usage {
                        prompt: self.prompt,
                        completion,
                        cache_hit: self.cache_hit,
                        cache_miss: self.cache_miss,
                    });
                }
            }
            "message_stop" => out.push(Event::Done),
            _ => {}
        }
        out
    }
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

fn count(u: &Value, key: &str) -> u64 {
    u.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn index(v: &Value) -> usize {
    v.get("index").and_then(Value::as_u64).unwrap_or(0) as usize
}

/// Internal finish values the session loop already understands.
fn finish(reason: &str) -> &'static str {
    match reason {
        "tool_use" => "tool_calls",
        "max_tokens" => "length",
        "refusal" => "content_filter",
        _ => "stop",
    }
}

/// Split system text out and turn each message into content blocks.
fn translate(history: &[Message]) -> (Option<String>, Vec<Value>) {
    let system = history
        .iter()
        .filter(|m| m.role == Role::System)
        .map(|m| m.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let system = if system.is_empty() { None } else { Some(system) };

    let mut messages = Vec::new();
    for m in history {
        let mut parts = Vec::new();
        match m.role {
            Role::System => continue,
            Role::User => {
                if !m.content.is_empty() {
                    parts.push(json!({"type": "text", "text": m.content}));
                }
                for image in &m.images {
                    let path = crate::images::dir().join(&image.file);
                    if let Ok(bytes) = std::fs::read(&path) {
                        parts.push(json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": image.mime,
                                "data": crate::api::base64(&bytes),
                            }
                        }));
                    }
                }
                if !parts.is_empty() {
                    messages.push(json!({"role": "user", "content": parts}));
                }
            }
            Role::Assistant => {
                if !m.content.is_empty() {
                    parts.push(json!({"type": "text", "text": m.content}));
                }
                for call in &m.tool_calls {
                    parts.push(json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.name,
                        "input": input(&call.arguments),
                    }));
                }
                if !parts.is_empty() {
                    messages.push(json!({"role": "assistant", "content": parts}));
                }
            }
            Role::Tool => {
                let id = m.tool_call_id.clone().unwrap_or_default();
                messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": id,
                        "content": m.content,
                    }],
                }));
            }
        }
    }
    (system, messages)
}

fn input(arguments: &str) -> Value {
    serde_json::from_str(arguments).unwrap_or_else(|_| json!({}))
}

/// OpenAI-shaped tool schemas become Anthropic tool definitions.
fn tool_defs(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .filter_map(|t| {
            let f = t.get("function")?;
            let mut def = json!({
                "name": f.get("name")?,
                "input_schema": f.get("parameters").cloned().unwrap_or_else(|| json!({"type": "object"})),
            });
            if let Some(desc) = f.get("description") {
                def["description"] = desc.clone();
            }
            Some(def)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Accum, SseParser};
    use crate::providers::Provider;

    fn feed(events: &[&str]) -> Accum {
        let mut adapter = crate::providers::adapter(Provider::Anthropic, "", "k");
        let mut parser = SseParser::default();
        let mut acc = Accum::default();
        for event in events {
            for ev in parser.push(event.as_bytes(), adapter.as_mut()) {
                acc.apply(ev);
            }
        }
        acc
    }

    fn sse(name: &str, data: Value) -> String {
        format!("event: {name}\ndata: {data}\n\n")
    }

    #[test]
    fn text_stream_parses() {
        let acc = feed(&[
            &sse(
                "message_start",
                json!({"type": "message_start", "message": {"model": "claude-sonnet-4-5", "usage": {"input_tokens": 9, "cache_read_input_tokens": 4}}}),
            ),
            &sse(
                "content_block_delta",
                json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}),
            ),
            &sse(
                "content_block_delta",
                json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " world"}}),
            ),
            &sse(
                "message_delta",
                json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 3}}),
            ),
            &sse("message_stop", json!({"type": "message_stop"})),
        ]);
        assert_eq!(acc.content, "Hello world");
        assert_eq!(acc.finish.as_deref(), Some("stop"));
        assert_eq!(acc.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(acc.prompt_tokens, 13);
        assert_eq!(acc.cache_hit, 4);
        assert_eq!(acc.completion_tokens, 3);
    }

    #[test]
    fn tool_stream_becomes_a_call() {
        let acc = feed(&[
            &sse(
                "content_block_start",
                json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use", "id": "toolu_1", "name": "bash"}}),
            ),
            &sse(
                "content_block_delta",
                json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "{\"command\":"}}),
            ),
            &sse(
                "content_block_delta",
                json!({"type": "content_block_delta", "index": 1, "delta": {"type": "input_json_delta", "partial_json": "\"ls\"}"}}),
            ),
            &sse(
                "message_delta",
                json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 8}}),
            ),
        ]);
        let calls = acc.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "toolu_1");
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].arguments, "{\"command\":\"ls\"}");
        assert_eq!(acc.finish.as_deref(), Some("tool_calls"));
    }

    #[test]
    fn thinking_is_reasoning_not_text() {
        let acc = feed(&[&sse(
            "content_block_delta",
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "hmm"}}),
        )]);
        assert_eq!(acc.reasoning, "hmm");
        assert!(acc.content.is_empty());
    }

    #[test]
    fn wire_moves_system_and_maps_tools() {
        let mut history = vec![Message::system("be terse"), Message::user("hi")];
        let mut assistant = Message::assistant("");
        assistant.tool_calls.push(crate::api::ToolCall {
            id: "toolu_1".into(),
            name: "bash".into(),
            arguments: "{\"command\":\"ls\"}".into(),
        });
        history.push(assistant);
        history.push(Message::tool("toolu_1", "a.rs"));

        let mut adapter = crate::providers::adapter(Provider::Anthropic, "https://api.anthropic.com", "k");
        let tools = vec![json!({"type": "function", "function": {"name": "bash", "description": "run", "parameters": {"type": "object"}}})];
        let w = adapter.wire(
            &RequestOptions {
                model: "claude-sonnet-4-5".into(),
                max_tokens: 100,
                thinking: true,
                temperature: Some(0.2),
            },
            &history,
            &tools,
        );
        assert_eq!(w.url, "https://api.anthropic.com/v1/messages");
        assert!(w.headers.iter().any(|h| h == "x-api-key: k"));
        assert!(w.headers.iter().any(|h| h == "anthropic-version: 2023-06-01"));
        assert_eq!(w.body["system"], "be terse");
        assert!(w.body.get("thinking").is_none());
        assert_eq!(w.body["tools"][0]["input_schema"]["type"], "object");
        let last = w.body["messages"].as_array().unwrap().last().unwrap();
        assert_eq!(last["content"][0]["type"], "tool_result");
        assert_eq!(last["content"][0]["tool_use_id"], "toolu_1");
        assert_eq!(
            w.body["messages"][1]["content"][0]["type"],
            "tool_use"
        );
    }
}
