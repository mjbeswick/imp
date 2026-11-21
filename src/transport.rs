//! The one seam between the agent loop and the network.

#[cfg(test)]
use std::sync::Mutex;

pub trait Transport: Send {
    /// POST `body`, feeding response bytes to `on_chunk` as they arrive.
    fn post(
        &self,
        url: &str,
        headers: &[&str],
        body: &[u8],
        cancel: &std::sync::atomic::AtomicBool,
        on_chunk: &mut dyn FnMut(&[u8]),
    ) -> Result<u16, String>;
}

pub struct Live;

impl Transport for Live {
    fn post(
        &self,
        url: &str,
        headers: &[&str],
        body: &[u8],
        cancel: &std::sync::atomic::AtomicBool,
        on_chunk: &mut dyn FnMut(&[u8]),
    ) -> Result<u16, String> {
        crate::http::post_stream(url, headers, body, cancel, on_chunk).map(|r| r.status)
    }
}

/// One scripted reply: either chunks delivered as a 200, or a status failure.
#[cfg(test)]
pub enum Script {
    Ok(Vec<String>),
    Fails(u16, String),
    Transport(String),
    /// The user cancels mid-request: sets the flag, as the cancel key does.
    Cancel,
}

/// Replays scripted replies and records what was sent. Tests only.
#[cfg(test)]
pub struct Fake {
    script: Mutex<Vec<Script>>,
    pub sent: Mutex<Vec<String>>,
}

#[cfg(test)]
impl Fake {
    pub fn new(script: Vec<Script>) -> Self {
        Self {
            script: Mutex::new(script),
            sent: Mutex::new(Vec::new()),
        }
    }

    pub fn attempts(&self) -> usize {
        self.sent.lock().unwrap().len()
    }

    pub fn last_body(&self) -> String {
        self.sent.lock().unwrap().last().cloned().unwrap_or_default()
    }
}

#[cfg(test)]
impl Transport for Fake {
    fn post(
        &self,
        _url: &str,
        _headers: &[&str],
        body: &[u8],
        cancel: &std::sync::atomic::AtomicBool,
        on_chunk: &mut dyn FnMut(&[u8]),
    ) -> Result<u16, String> {
        self.sent
            .lock()
            .unwrap()
            .push(String::from_utf8_lossy(body).to_string());
        let next = {
            let mut script = self.script.lock().unwrap();
            if script.is_empty() {
                return Err("no scripted reply left".into());
            }
            script.remove(0)
        };
        match next {
            Script::Ok(chunks) => {
                for chunk in chunks {
                    on_chunk(chunk.as_bytes());
                }
                Ok(200)
            }
            Script::Fails(status, body) => {
                on_chunk(body.as_bytes());
                Ok(status)
            }
            Script::Transport(message) => Err(message),
            Script::Cancel => {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                Err("cancelled".into())
            }
        }
    }
}

#[cfg(test)]
fn sse(value: serde_json::Value) -> String {
    format!("data: {value}\n\n")
}

#[cfg(test)]
const USAGE: &str = r#""usage":{"prompt_tokens":10,"completion_tokens":5,"prompt_cache_hit_tokens":8,"prompt_cache_miss_tokens":2}"#;

/// SSE payloads for a text reply, then `[DONE]`.
#[cfg(test)]
pub fn text_reply(text: &str) -> Vec<String> {
    let first = serde_json::json!({
        "choices": [{"index": 0, "delta": {"content": text}, "finish_reason": null}]
    });
    let last: serde_json::Value = serde_json::from_str(&format!(
        r#"{{"choices":[{{"index":0,"delta":{{}},"finish_reason":"stop"}}],{USAGE}}}"#
    ))
    .unwrap();
    vec![sse(first), sse(last), "data: [DONE]\n\n".into()]
}

/// SSE payloads for one tool call.
#[cfg(test)]
pub fn tool_reply(id: &str, name: &str, arguments: &str) -> Vec<String> {
    let first = serde_json::json!({
        "choices": [{
            "index": 0,
            "delta": {"tool_calls": [{
                "index": 0, "id": id, "type": "function",
                "function": {"name": name, "arguments": ""}
            }]},
            "finish_reason": null
        }]
    });
    let second = serde_json::json!({
        "choices": [{
            "index": 0,
            "delta": {"tool_calls": [{
                "index": 0, "function": {"name": "", "arguments": arguments}
            }]},
            "finish_reason": null
        }]
    });
    let last = serde_json::json!({
        "choices": [{"index": 0, "delta": {}, "finish_reason": "tool_calls"}]
    });
    vec![sse(first), sse(second), sse(last), "data: [DONE]\n\n".into()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Accum, SseParser};

    fn run(chunks: Vec<String>) -> Accum {
        let mut adapter = crate::providers::adapter(
            crate::providers::Provider::DeepSeek,
            "https://api.deepseek.com",
            "",
        );
        let mut parser = SseParser::default();
        let mut acc = Accum::default();
        for c in chunks {
            for ev in parser.push(c.as_bytes(), adapter.as_mut()) {
                acc.apply(ev);
            }
        }
        acc
    }

    #[test]
    fn text_fixture_parses() {
        let acc = run(text_reply("hello"));
        assert_eq!(acc.content, "hello");
        assert_eq!(acc.finish.as_deref(), Some("stop"));
        assert_eq!(acc.cache_hit, 8);
    }

    #[test]
    fn tool_fixture_parses() {
        let acc = run(tool_reply("call_1", "bash", "{\"command\":\"ls\"}"));
        let calls = acc.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "bash");
        assert_eq!(calls[0].arguments, "{\"command\":\"ls\"}");
        assert_eq!(acc.finish.as_deref(), Some("tool_calls"));
    }
}

impl<T: Transport + Send + Sync + ?Sized> Transport for std::sync::Arc<T> {
    fn post(
        &self,
        url: &str,
        headers: &[&str],
        body: &[u8],
        cancel: &std::sync::atomic::AtomicBool,
        on_chunk: &mut dyn FnMut(&[u8]),
    ) -> Result<u16, String> {
        (**self).post(url, headers, body, cancel, on_chunk)
    }
}
