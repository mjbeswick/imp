//! Gemini `streamGenerateContent` requests and SSE.
//!
//! System text becomes `systemInstruction`, messages become `contents`, tool
//! results become `functionResponse` parts, and `usageMetadata` fills the
//! token counters.

use crate::api::{Event, Message, RequestOptions, Role};
use crate::providers::{Adapter, Wire};
use serde_json::{json, Value};

pub struct Gemini {
    base_url: String,
    key: String,
    calls: usize,
}

impl Gemini {
    pub fn new(base_url: &str, key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            key: key.to_string(),
            calls: 0,
        }
    }
}

impl Adapter for Gemini {
    fn wire(&mut self, opts: &RequestOptions, history: &[Message], tools: &[Value]) -> Wire {
        self.calls = 0;
        let mut body = json!({"contents": contents(history)});
        let map = body.as_object_mut().expect("object");

        let system = history
            .iter()
            .filter(|m| m.role == Role::System)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        if !system.is_empty() {
            map.insert(
                "systemInstruction".into(),
                json!({"parts": [{"text": system}]}),
            );
        }

        let mut config = serde_json::Map::new();
        if let Some(t) = opts.temperature {
            config.insert("temperature".into(), json!(t));
        }
        config.insert("maxOutputTokens".into(), json!(opts.max_tokens));
        map.insert("generationConfig".into(), Value::Object(config));

        let defs = tool_defs(tools);
        if !defs.is_empty() {
            map.insert("tools".into(), json!([{"functionDeclarations": defs}]));
        }

        Wire {
            url: format!(
                "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
                self.base_url, opts.model
            ),
            headers: vec![
                "Content-Type: application/json".into(),
                format!("x-goog-api-key: {}", self.key),
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

        if let Some(model) = v.get("modelVersion").and_then(Value::as_str) {
            out.push(Event::Model(model.to_string()));
        }
        if let Some(u) = v.get("usageMetadata") {
            let prompt = count(u, "promptTokenCount");
            let cache_hit = count(u, "cachedContentTokenCount");
            out.push(Event::Usage {
                prompt,
                completion: count(u, "candidatesTokenCount") + count(u, "thoughtsTokenCount"),
                cache_hit,
                cache_miss: prompt.saturating_sub(cache_hit),
            });
        }

        let Some(candidate) = v
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
        else {
            return out;
        };

        if let Some(parts) = candidate
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(Value::as_array)
        {
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        if part.get("thought").and_then(Value::as_bool).unwrap_or(false) {
                            out.push(Event::Reasoning(text.to_string()));
                        } else {
                            out.push(Event::Content(text.to_string()));
                        }
                    }
                }
                if let Some(call) = part.get("functionCall") {
                    let index = self.calls;
                    self.calls += 1;
                    out.push(Event::ToolCall {
                        index,
                        id: Some(format!("gemini-{index}")),
                        name: call.get("name").and_then(Value::as_str).map(str::to_string),
                        arguments: Some(
                            call.get("args").cloned().unwrap_or_else(|| json!({})).to_string(),
                        ),
                    });
                }
            }
        }

        if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
            out.push(Event::Finish(finish(reason, self.calls > 0).to_string()));
        }
        out
    }
}

fn count(u: &Value, key: &str) -> u64 {
    u.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Internal finish values the session loop already understands.
fn finish(reason: &str, called: bool) -> &'static str {
    match reason {
        "MAX_TOKENS" => "length",
        "SAFETY" | "RECITATION" | "PROHIBITED_CONTENT" | "BLOCKLIST" => "content_filter",
        _ if called => "tool_calls",
        _ => "stop",
    }
}

/// Chat messages as `contents`; tool results are user turns answering a call.
fn contents(history: &[Message]) -> Vec<Value> {
    let mut names: Vec<(String, String)> = Vec::new();
    for m in history {
        for call in &m.tool_calls {
            names.push((call.id.clone(), call.name.clone()));
        }
    }
    let name_for = |id: &str| {
        names
            .iter()
            .find(|(call_id, _)| call_id == id)
            .map(|(_, name)| name.clone())
            .unwrap_or_default()
    };

    let mut contents = Vec::new();
    for m in history {
        let mut parts = Vec::new();
        match m.role {
            Role::System => continue,
            Role::User => {
                if !m.content.is_empty() {
                    parts.push(json!({"text": m.content}));
                }
                for image in &m.images {
                    let path = crate::images::dir().join(&image.file);
                    if let Ok(bytes) = std::fs::read(&path) {
                        parts.push(json!({
                            "inlineData": {
                                "mimeType": image.mime,
                                "data": crate::api::base64(&bytes),
                            }
                        }));
                    }
                }
                if !parts.is_empty() {
                    contents.push(json!({"role": "user", "parts": parts}));
                }
            }
            Role::Assistant => {
                if !m.content.is_empty() {
                    parts.push(json!({"text": m.content}));
                }
                for call in &m.tool_calls {
                    parts.push(json!({
                        "functionCall": {
                            "name": call.name,
                            "args": serde_json::from_str::<Value>(&call.arguments)
                                .unwrap_or_else(|_| json!({})),
                        }
                    }));
                }
                if !parts.is_empty() {
                    contents.push(json!({"role": "model", "parts": parts}));
                }
            }
            Role::Tool => {
                let name = name_for(m.tool_call_id.as_deref().unwrap_or(""));
                contents.push(json!({
                    "role": "user",
                    "parts": [{"functionResponse": {
                        "name": name,
                        "response": {"result": m.content},
                    }}],
                }));
            }
        }
    }
    contents
}

/// OpenAI-shaped tool schemas become Gemini function declarations.
fn tool_defs(tools: &[Value]) -> Vec<Value> {
    tools
        .iter()
        .filter_map(|t| {
            let f = t.get("function")?;
            let mut def = json!({
                "name": f.get("name")?,
                "parameters": f.get("parameters").cloned().unwrap_or_else(|| json!({"type": "object"})),
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

    fn feed(payloads: &[Value]) -> Accum {
        let mut adapter = crate::providers::adapter(Provider::Gemini, "", "k");
        let mut parser = SseParser::default();
        let mut acc = Accum::default();
        for payload in payloads {
            let line = format!("data: {payload}\n\n");
            for ev in parser.push(line.as_bytes(), adapter.as_mut()) {
                acc.apply(ev);
            }
        }
        acc
    }

    #[test]
    fn text_and_usage_parse() {
        let acc = feed(&[
            json!({"candidates": [{"content": {"parts": [{"text": "Hello"}]}}]}),
            json!({
                "candidates": [{"content": {"parts": [{"text": " world"}]}, "finishReason": "STOP"}],
                "usageMetadata": {"promptTokenCount": 20, "candidatesTokenCount": 4, "cachedContentTokenCount": 6},
            }),
        ]);
        assert_eq!(acc.content, "Hello world");
        assert_eq!(acc.finish.as_deref(), Some("stop"));
        assert_eq!(acc.prompt_tokens, 20);
        assert_eq!(acc.completion_tokens, 4);
        assert_eq!(acc.cache_hit, 6);
        assert_eq!(acc.cache_miss, 14);
    }

    #[test]
    fn function_call_becomes_a_tool_call() {
        let acc = feed(&[json!({
            "candidates": [{
                "content": {"parts": [{"functionCall": {"name": "bash", "args": {"command": "ls"}}}]},
                "finishReason": "STOP",
            }],
        })]);
        let calls = acc.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].arguments, "{\"command\":\"ls\"}");
        assert!(!calls[0].id.is_empty());
        assert_eq!(acc.finish.as_deref(), Some("tool_calls"));
    }

    #[test]
    fn thought_parts_become_reasoning() {
        let acc = feed(&[json!({
            "candidates": [{"content": {"parts": [{"text": "hmm", "thought": true}]}}],
        })]);
        assert_eq!(acc.reasoning, "hmm");
        assert!(acc.content.is_empty());
    }

    #[test]
    fn wire_uses_the_gemini_endpoint_and_shapes() {
        let mut history = vec![Message::system("be terse"), Message::user("hi")];
        let mut assistant = Message::assistant("");
        assistant.tool_calls.push(crate::api::ToolCall {
            id: "gemini-0".into(),
            name: "bash".into(),
            arguments: "{\"command\":\"ls\"}".into(),
        });
        history.push(assistant);
        history.push(Message::tool("gemini-0", "a.rs"));

        let mut adapter = crate::providers::adapter(Provider::Gemini, "https://generativelanguage.googleapis.com", "k");
        let tools = vec![json!({"type": "function", "function": {"name": "bash", "parameters": {"type": "object"}}})];
        let w = adapter.wire(
            &RequestOptions {
                model: "gemini-2.5-pro".into(),
                max_tokens: 100,
                thinking: true,
                temperature: Some(0.2),
            },
            &history,
            &tools,
        );
        assert_eq!(
            w.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:streamGenerateContent?alt=sse"
        );
        assert!(w.headers.iter().any(|h| h == "x-goog-api-key: k"));
        assert_eq!(w.body["systemInstruction"]["parts"][0]["text"], "be terse");
        assert_eq!(w.body["generationConfig"]["maxOutputTokens"], 100);
        assert!(w.body.get("thinking").is_none());
        assert!(w.body.get("system").is_none());
        assert_eq!(w.body["tools"][0]["functionDeclarations"][0]["name"], "bash");
        let contents = w.body["contents"].as_array().unwrap();
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[2]["parts"][0]["functionResponse"]["name"], "bash");
    }
}
