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

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepSeek => "deepseek",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
            Self::Custom => "custom",
        }
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
                model: "claude-sonnet-4-5",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_profile_is_known_and_parses_back() {
        for p in [
            Provider::DeepSeek,
            Provider::OpenAi,
            Provider::Anthropic,
            Provider::Gemini,
            Provider::Ollama,
            Provider::Custom,
        ] {
            assert_eq!(Provider::parse(p.as_str()).unwrap(), p);
            let profile = p.profile();
            assert!(!profile.base_url.is_empty(), "{} has no base url", p.as_str());
            assert!(!profile.model.is_empty(), "{} has no model", p.as_str());
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
            assert!(!p.caps().search, "{} kept search", p.as_str());
            assert!(!p.caps().cache, "{} kept cache", p.as_str());
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
}
