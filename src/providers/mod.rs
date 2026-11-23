//! Provider profiles: defaults, key variables, and capabilities.
//!
//! Each profile names an endpoint, a model, and the variable its key lives in.
//! The wire adapters live beside this module and share one internal event
//! stream, so the session loop never sees a provider's dialect.

pub mod anthropic;
pub mod gemini;
pub mod openai;

use crate::api::{Event, Message, RequestOptions};
use serde_json::Value;

/// One request, fully prepared: endpoint, headers, and JSON body.
pub struct Wire {
    pub url: String,
    pub headers: Vec<String>,
    pub body: Value,
}

/// A provider's request builder and chunk parser. State is per stream.
pub trait Adapter {
    fn wire(&mut self, opts: &RequestOptions, history: &[Message], tools: &[Value]) -> Wire;
    /// Turn one SSE data payload into internal events.
    fn event(&mut self, payload: &str) -> Vec<Event>;
}

/// Pick the adapter for a profile. OpenAI, Ollama, and custom hosts share one.
pub fn adapter(provider: Provider, base_url: &str, key: &str) -> Box<dyn Adapter> {
    match provider {
        Provider::Anthropic => Box::new(anthropic::Anthropic::new(base_url, key)),
        Provider::Gemini => Box::new(gemini::Gemini::new(base_url, key)),
        _ => Box::new(openai::OpenAi::new(base_url, key, provider.caps())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    DeepSeek,
    OpenAi,
    Anthropic,
    Gemini,
    Ollama,
    Custom,
}

/// Defaults for one provider. Config `base_url`, `model`, and `api_key` win.
#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub base_url: &'static str,
    pub model: &'static str,
    pub key_var: Option<&'static str>,
}

impl Provider {
    pub fn parse(s: &str) -> Result<Self, String> {
        Ok(match s.trim().to_ascii_lowercase().as_str() {
            "deepseek" => Self::DeepSeek,
            "openai" => Self::OpenAi,
            "anthropic" => Self::Anthropic,
            "gemini" => Self::Gemini,
            "ollama" => Self::Ollama,
            "custom" => Self::Custom,
            other => {
                return Err(format!(
                    "provider must be deepseek, openai, anthropic, gemini, ollama, or custom (got {other:?})"
                ))
            }
        })
    }

    pub fn profile(self) -> Profile {
        match self {
            Self::DeepSeek => Profile {
                base_url: "https://api.deepseek.com",
                model: "deepseek-flash",
                key_var: Some("DEEPSEEK_API_KEY"),
            },
            Self::OpenAi => Profile {
                base_url: "https://api.openai.com/v1",
                model: "gpt-4.1-mini",
                key_var: Some("OPENAI_API_KEY"),
            },
            Self::Anthropic => Profile {
                base_url: "https://api.anthropic.com",
                model: "claude-opus-5-5",
                key_var: Some("ANTHROPIC_API_KEY"),
            },
            Self::Gemini => Profile {
                base_url: "https://generativelanguage.googleapis.com",
                model: "gemini-2.5-pro",
                key_var: Some("GEMINI_API_KEY"),
            },
            Self::Ollama => Profile {
                base_url: "http://localhost:11434/v1",
                model: "llama3.2",
                key_var: None,
            },
            Self::Custom => Profile {
                base_url: "http://localhost:8000/v1",
                model: "local-model",
                key_var: None,
            },
        }
    }

    /// What the wire accepts: thinking field, token key, cache counters, and
    /// the search endpoints.
    pub fn caps(self) -> Caps {
        match self {
            Self::DeepSeek => Caps {
                thinking: Thinking::DeepSeek,
                tokens: TokenKey::MaxTokens,
                cache: true,
                search: true,
                reasoning: true,
            },
            Self::OpenAi => Caps {
                thinking: Thinking::Effort,
                tokens: TokenKey::MaxCompletionTokens,
                cache: false,
                search: false,
                reasoning: false,
            },
            Self::Anthropic => Caps {
                thinking: Thinking::None,
                tokens: TokenKey::MaxTokens,
                cache: false,
                search: false,
                reasoning: false,
            },
            Self::Gemini => Caps {
                thinking: Thinking::None,
                tokens: TokenKey::MaxOutputTokens,
                cache: false,
                search: false,
                reasoning: false,
            },
            Self::Ollama | Self::Custom => Caps {
                thinking: Thinking::None,
                tokens: TokenKey::MaxTokens,
                cache: false,
                search: false,
                reasoning: false,
            },
        }
    }
}

/// How a provider wants thinking switched on, if at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thinking {
    None,
    DeepSeek,
    Effort,
}

/// The field a provider uses to cap the reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKey {
    MaxTokens,
    MaxCompletionTokens,
    MaxOutputTokens,
}

#[derive(Debug, Clone, Copy)]
pub struct Caps {
    pub thinking: Thinking,
    pub tokens: TokenKey,
    pub cache: bool,
    pub search: bool,
    /// Echo assistant reasoning back in the history.
    pub reasoning: bool,
}

/// A curated list for the picker before a fetch answers, and after a failure.
pub fn catalog(provider: Provider) -> Vec<String> {
    let names: &[&str] = match provider {
        Provider::DeepSeek => &["deepseek-flash", "deepseek-chat", "deepseek-reasoner"],
        Provider::OpenAi => &[
            "gpt-4.1",
            "gpt-4.1-mini",
            "gpt-4o",
            "gpt-4o-mini",
            "o3",
            "o4-mini",
        ],
        Provider::Anthropic => &["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"],
        Provider::Gemini => &["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"],
        Provider::Ollama => &["llama3.2", "qwen2.5-coder", "mistral"],
        Provider::Custom => &[],
    };
    names.iter().map(|name| name.to_string()).collect()
}

/// The provider's model list, fetched with a short timeout.
pub fn list_models(provider: Provider, base_url: &str, key: &str) -> Result<Vec<String>, String> {
    let base = base_url.trim_end_matches('/');
    let (url, headers) = match provider {
        Provider::Anthropic => (
            format!("{base}/v1/models"),
            vec![
                format!("x-api-key: {key}"),
                "anthropic-version: 2023-06-01".to_string(),
            ],
        ),
        Provider::Gemini => (
            format!("{base}/v1beta/models"),
            vec![format!("x-goog-api-key: {key}")],
        ),
        _ => (
            format!("{base}/models"),
            vec![format!("Authorization: Bearer {key}")],
        ),
    };
    let refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let (status, body) = crate::http::get(&url, &refs, 5_000)?;
    if !(200..300).contains(&status) {
        return Err(format!("models: HTTP {status}"));
    }
    parse_models(provider, &body)
}

/// Read one provider's model document.
pub fn parse_models(provider: Provider, body: &str) -> Result<Vec<String>, String> {
    let value: Value = serde_json::from_str(body).map_err(|e| format!("models: {e}"))?;
    let mut out = Vec::new();
    if provider == Provider::Gemini {
        for model in value["models"].as_array().into_iter().flatten() {
            if let Some(name) = model["name"].as_str() {
                out.push(name.trim_start_matches("models/").to_string());
            }
        }
    } else {
        for model in value["data"].as_array().into_iter().flatten() {
            if let Some(id) = model["id"].as_str() {
                out.push(id.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_profile_is_known_and_parses_back() {
        for (name, p) in [
            ("deepseek", Provider::DeepSeek),
            ("openai", Provider::OpenAi),
            ("anthropic", Provider::Anthropic),
            ("gemini", Provider::Gemini),
            ("ollama", Provider::Ollama),
            ("custom", Provider::Custom),
        ] {
            assert_eq!(Provider::parse(name).unwrap(), p);
            let profile = p.profile();
            assert!(!profile.base_url.is_empty(), "{name} has no base url");
            assert!(!profile.model.is_empty(), "{name} has no model");
        }
    }

    #[test]
    fn unknown_provider_names_the_choices() {
        let err = Provider::parse("nope").unwrap_err();
        assert!(err.contains("anthropic"), "{err}");
    }

    #[test]
    fn only_deepseek_keeps_search_and_cache() {
        assert!(Provider::DeepSeek.caps().search);
        assert!(Provider::DeepSeek.caps().cache);
        for p in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
            assert!(!p.caps().search, "{p:?} kept search");
            assert!(!p.caps().cache, "{p:?} kept cache");
        }
    }

    #[test]
    fn ollama_and_custom_need_no_key() {
        assert!(Provider::Ollama.profile().key_var.is_none());
        assert!(Provider::Custom.profile().key_var.is_none());
        assert_eq!(
            Provider::DeepSeek.profile().key_var,
            Some("DEEPSEEK_API_KEY")
        );
    }

    #[test]
    fn anthropic_lists_current_models() {
        assert_eq!(catalog(Provider::Anthropic), vec!["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"]);
        assert_eq!(Provider::Anthropic.profile().model, "claude-opus-5-5");
    }

    #[test]
    fn catalog_names_are_unique_and_known() {
        for provider in [
            Provider::DeepSeek,
            Provider::OpenAi,
            Provider::Anthropic,
            Provider::Gemini,
            Provider::Ollama,
        ] {
            let names = catalog(provider);
            assert!(!names.is_empty(), "{provider:?} has no catalog");
            let mut sorted = names.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), names.len(), "{provider:?} repeats a name");
        }
        assert!(catalog(Provider::Custom).is_empty());
    }

    #[test]
    fn openai_shaped_model_lists_parse() {
        let body = r#"{"data":[{"id":"gpt-b"},{"id":"gpt-a"},{"id":"gpt-a"}]}"#;
        let names = parse_models(Provider::OpenAi, body).unwrap();
        assert_eq!(names, vec!["gpt-a", "gpt-b"]);
        let names = parse_models(Provider::Anthropic, body).unwrap();
        assert_eq!(names, vec!["gpt-a", "gpt-b"]);
    }

    #[test]
    fn gemini_model_names_lose_their_prefix() {
        let body = r#"{"models":[{"name":"models/gemini-b"},{"name":"models/gemini-a"}]}"#;
        assert_eq!(
            parse_models(Provider::Gemini, body).unwrap(),
            vec!["gemini-a", "gemini-b"]
        );
    }

    #[test]
    fn a_bad_model_document_is_an_error() {
        assert!(parse_models(Provider::OpenAi, "not json").is_err());
    }
}
