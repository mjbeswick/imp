//! OpenAI-compatible requests and streams.
//!
//! DeepSeek, OpenAI, Ollama, and custom hosts share this dialect. Capabilities
//! decide which optional fields go out and which usage counters come back.

use crate::api::{Event, Message, RequestOptions};
use crate::providers::{Adapter, Caps, Thinking, TokenKey, Wire};
use serde_json::{json, Value};

pub struct OpenAi {
    base_url: String,
    key: String,
    caps: Caps,
}

impl OpenAi {
    pub fn new(base_url: &str, key: &str, caps: Caps) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            key: key.to_string(),
            caps,
        }
    }

    /// One message in wire shape. DeepSeek keeps reasoning and image
    /// bookkeeping; stricter hosts get neither unknown field.
    fn message(&self, m: &Message) -> Value {
        let mut v = m.to_json();
        if !self.caps.reasoning {
            if let Some(map) = v.as_object_mut() {
                map.remove("reasoning_content");
                map.remove("images");
            }
        }
        v
    }
}

impl Adapter for OpenAi {
    fn wire(&mut self, opts: &RequestOptions, history: &[Message], tools: &[Value]) -> Wire {
        let messages: Vec<Value> = history.iter().map(|m| self.message(m)).collect();
        let mut body = json!({
            "model": opts.model,
            "messages": messages,
            "stream": true,
        });
        let map = body.as_object_mut().expect("object");
        let token_key = match self.caps.tokens {
            TokenKey::MaxCompletionTokens => "max_completion_tokens",
            _ => "max_tokens",
        };
        map.insert(token_key.into(), json!(opts.max_tokens));
        match self.caps.thinking {
            Thinking::DeepSeek => {
                map.insert(
                    "thinking".into(),
                    json!({ "type": if opts.thinking { "enabled" } else { "disabled" } }),
                );
                if opts.thinking {
                    map.insert("reasoning_effort".into(), json!("low"));
                } else if let Some(t) = opts.temperature {
                    map.insert("temperature".into(), json!(t));
                }
            }
            Thinking::Effort => {
                if opts.thinking {
                    map.insert("reasoning_effort".into(), json!("low"));
                } else if let Some(t) = opts.temperature {
                    map.insert("temperature".into(), json!(t));
                }
            }
            Thinking::None => {
                if let Some(t) = opts.temperature {
                    map.insert("temperature".into(), json!(t));
                }
            }
        }
        if !tools.is_empty() {
            map.insert("tools".into(), Value::Array(tools.to_vec()));
        }

        let mut headers = vec!["Content-Type: application/json".to_string()];
        if !self.key.is_empty() {
            headers.push(format!("Authorization: Bearer {}", self.key));
        }
        headers.push("Accept: text/event-stream".into());
        Wire {
            url: format!("{}/chat/completions", self.base_url),
            headers,
            body,
        }
    }

    fn event(&mut self, payload: &str) -> Vec<Event> {
        parse_chunk(payload, self.caps.cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Provider;

    fn opts(thinking: bool) -> RequestOptions {
        RequestOptions {
            model: "deepseek-flash".into(),
            max_tokens: 10,
            thinking,
            temperature: Some(0.2),
        }
    }

    fn wire(provider: Provider, key: &str, history: &[Message]) -> Wire {
        let mut adapter = OpenAi::new("https://api.test/v1", key, provider.caps());
        adapter.wire(&opts(false), history, &[])
    }

    #[test]
    fn deepseek_body_is_unchanged() {
        let w = wire(Provider::DeepSeek, "k", &[Message::user("hi")]);
        assert_eq!(
            w.body,
            json!({
                "model": "deepseek-flash",
                "messages": [{"role": "user", "content": "hi"}],
                "stream": true,
                "max_tokens": 10,
                "thinking": {"type": "disabled"},
                "temperature": 0.2f32,
            })
        );
        assert_eq!(w.url, "https://api.test/v1/chat/completions");
        assert!(w.headers.iter().any(|h| h == "Authorization: Bearer k"));
    }

    #[test]
    fn openai_gets_its_token_key_and_no_deepseek_fields() {
        let mut assistant = Message::assistant("hi");
        assistant.reasoning = "because".into();
        let w = wire(Provider::OpenAi, "k", &[assistant]);
        assert_eq!(w.body["max_completion_tokens"], 10);
        assert!(w.body.get("max_tokens").is_none());
        assert!(w.body.get("thinking").is_none());
        assert!(w.body["messages"][0].get("reasoning_content").is_none());
    }

    #[test]
    fn thinking_effort_replaces_the_deepseek_object() {
        let mut adapter = OpenAi::new("https://api.openai.com/v1", "k", Provider::OpenAi.caps());
        let w = adapter.wire(&opts(true), &[Message::user("hi")], &[]);
        assert_eq!(w.body["reasoning_effort"], "low");
        assert!(w.body.get("thinking").is_none());
        assert!(w.body.get("temperature").is_none());
    }

    #[test]
    fn a_local_host_sends_no_auth_header() {
        let w = wire(Provider::Ollama, "", &[Message::user("hi")]);
        assert!(!w.headers.iter().any(|h| h.starts_with("Authorization")));
        assert!(w.headers.iter().any(|h| h == "Content-Type: application/json"));
    }

    #[test]
    fn usage_without_cache_counts_is_zero() {
        let mut adapter = OpenAi::new("", "", Provider::OpenAi.caps());
        let events = adapter.event(
            r#"{"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#,
        );
        let usage = events
            .iter()
            .find_map(|e| match e {
                Event::Usage { prompt, completion, cache_hit, cache_miss } => {
                    Some((*prompt, *completion, *cache_hit, *cache_miss))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(usage, (10, 5, 0, 0));
    }
}

/// One `choices[].delta` chunk plus any usage the final chunk carries.
fn parse_chunk(payload: &str, cache: bool) -> Vec<Event> {
    let mut out = Vec::new();
    let Ok(v) = serde_json::from_str::<Value>(payload) else {
        return out;
    };

    if let Some(model) = v.get("model").and_then(Value::as_str) {
        if !model.is_empty() {
            out.push(Event::Model(model.to_string()));
        }
    }

    if let Some(usage) = v.get("usage").filter(|u| !u.is_null()) {
        let count = |field: &str| usage.get(field).and_then(Value::as_u64).unwrap_or(0);
        out.push(Event::Usage {
            prompt: count("prompt_tokens"),
            completion: count("completion_tokens"),
            cache_hit: if cache {
                count("prompt_cache_hit_tokens")
            } else {
                0
            },
            cache_miss: if cache {
                count("prompt_cache_miss_tokens")
            } else {
                0
            },
        });
    }

    let Some(choice) = v.get("choices").and_then(Value::as_array).and_then(|a| a.first()) else {
        return out;
    };

    if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
        out.push(Event::Finish(reason.to_string()));
    }

    let Some(delta) = choice.get("delta") else {
        return out;
    };
    if let Some(text) = delta.get("content").and_then(Value::as_str) {
        if !text.is_empty() {
            out.push(Event::Content(text.to_string()));
        }
    }
    if let Some(text) = delta.get("reasoning_content").and_then(Value::as_str) {
        if !text.is_empty() {
            out.push(Event::Reasoning(text.to_string()));
        }
    }
    if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            let index = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
            let id = call
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let func = call.get("function");
            let name = func
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let arguments = func
                .and_then(|f| f.get("arguments"))
                .and_then(Value::as_str)
                .map(str::to_string);
            out.push(Event::ToolCall {
                index,
                id,
                name,
                arguments,
            });
        }
    }
    out
}

