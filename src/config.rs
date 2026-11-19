//! Configuration: plain `key=value` file, environment wins.

use crate::providers::Provider;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permissions {
    Normal,
    Strict,
    Yolo,
}

impl Permissions {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Ok(Self::Normal),
            "strict" => Ok(Self::Strict),
            "yolo" => Ok(Self::Yolo),
            _ => Err(format!(
                "permissions must be normal, strict, or yolo (got {s:?})"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Strict => "strict",
            Self::Yolo => "yolo",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum Links {
     Auto,
     Always,
     Never,
 }
 
 impl Links {
     pub fn parse(s: &str) -> Result<Self, String> {
         match s.trim().to_ascii_lowercase().as_str() {
             "auto" => Ok(Self::Auto),
             "always" => Ok(Self::Always),
             "never" => Ok(Self::Never),
             _ => Err(format!("links must be auto, always, or never (got {s:?})")),
         }
     }
 
     /// Whether this terminal can open an OSC 8 hyperlink.
     pub fn supported(self) -> bool {
         match self {
             Self::Always => true,
             Self::Never => false,
             Self::Auto => {
                 if std::env::var_os("KITTY_WINDOW_ID").is_some()
                     || std::env::var_os("WEZTERM_EXECUTABLE").is_some()
                     || std::env::var_os("VTE_VERSION").is_some()
                 {
                     return true;
                 }
                 matches!(
                     std::env::var("TERM_PROGRAM").as_deref(),
                     Ok("iTerm.app") | Ok("ghostty") | Ok("WezTerm")
                 )
             }
         }
     }
 }
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(format!("color must be auto, always, or never (got {s:?})")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub permissions: Permissions,
    pub allow_sudo: bool,
    pub prune_at: u32,
    pub compact_at: u32,
    pub max_tokens: u32,
    pub max_steps: usize,
    pub thinking: bool,
    pub color: ColorMode,
    pub links: Links,
    pub skills_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        let profile = Provider::DeepSeek.profile();
        Self {
            provider: Provider::DeepSeek,
            api_key: String::new(),
            model: profile.model.into(),
            base_url: profile.base_url.into(),
            permissions: Permissions::Normal,
            allow_sudo: false,
            prune_at: 45_000,
            compact_at: 60_000,
            max_tokens: 8192,
            max_steps: 24,
            thinking: false,
            color: ColorMode::Auto,
            links: Links::Auto,
            skills_dir: None,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs_config().join("imp").join("config")
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let mut cfg = Self::from_str(&text)?;
        cfg.apply_env(|k| std::env::var(k).ok())?;
        Ok(cfg)
    }

    pub fn from_str(text: &str) -> Result<Self, String> {
        let mut cfg = Self::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(format!("line {}: expected key=value", n + 1));
            };
            cfg.set(k.trim(), v.trim())?;
        }
        Ok(cfg)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "provider" => self.set_provider(Provider::parse(value)?),
            "api_key" => self.api_key = value.to_string(),
            "model" => self.model = value.to_string(),
            "base_url" => self.base_url = value.trim_end_matches('/').to_string(),
            "permissions" => self.permissions = Permissions::parse(value)?,
            "allow_sudo" => self.allow_sudo = parse_bool(value)?,
            "prune_at" => self.prune_at = parse_u32(key, value)?,
            "compact_at" => self.compact_at = parse_u32(key, value)?,
            "max_tokens" => self.max_tokens = parse_u32(key, value)?,
            "max_steps" => self.max_steps = parse_max_steps(value)?,
            "thinking" => self.thinking = parse_bool(value)?,
            "links" => self.links = Links::parse(value)?,
            "color" => self.color = ColorMode::parse(value)?,
            "skills_dir" => self.skills_dir = Some(PathBuf::from(value)),
            // Unknown keys are ignored so newer configs do not break older runs.
            _ => {}
        }
        Ok(())
    }

    /// Select a profile. Model and base URL left at the old profile's defaults
    /// move with it; explicit values stay put.
    pub fn set_provider(&mut self, next: Provider) {
        let old = self.provider.profile();
        if self.model == old.model {
            self.model = next.profile().model.to_string();
        }
        if self.base_url == old.base_url {
            self.base_url = next.profile().base_url.to_string();
        }
        self.provider = next;
    }

    /// Fill the key in order: config `api_key`, `IMP_API_KEY`, then the
    /// provider's own variable. Providers that need none are left alone.
    pub fn resolve_key_env(
        &mut self,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<(), String> {
        let filled = |v: String| !v.trim().is_empty();
        if filled(self.api_key.clone()) {
            return Ok(());
        }
        if let Some(v) = lookup("IMP_API_KEY").filter(|v| filled(v.clone())) {
            self.api_key = v;
            return Ok(());
        }
        match self.provider.profile().key_var {
            Some(var) => match lookup(var).filter(|v| filled(v.clone())) {
                Some(v) => {
                    self.api_key = v;
                    Ok(())
                }
                None => Err(format!("{var} is not set")),
            },
            None => Ok(()),
        }
    }

    pub fn apply_env(&mut self, lookup: impl Fn(&str) -> Option<String>) -> Result<(), String> {
        let set = |k: &str| lookup(k);
        if let Some(v) = set("IMP_PROVIDER") {
            self.set_provider(Provider::parse(&v)?);
        }
        if let Some(v) = set("IMP_MAX_STEPS") {
            self.max_steps = parse_max_steps(&v)?;
        }
        if let Some(v) = set("IMP_MODEL") {
            self.model = v;
        }
        if let Some(v) = set("IMP_BASE_URL") {
            self.base_url = v.trim_end_matches('/').to_string();
        }
        if let Some(v) = set("IMP_PERMISSIONS") {
            self.permissions = Permissions::parse(&v)?;
        }
        if let Some(v) = set("IMP_ALLOW_SUDO") {
            self.allow_sudo = parse_bool(&v)?;
        }
        if let Some(v) = set("IMP_PRUNE_AT") {
            self.prune_at = parse_u32("IMP_PRUNE_AT", &v)?;
        }
        if let Some(v) = set("IMP_COMPACT_AT") {
            self.compact_at = parse_u32("IMP_COMPACT_AT", &v)?;
        }
        if let Some(v) = set("IMP_MAX_TOKENS") {
            self.max_tokens = parse_u32("IMP_MAX_TOKENS", &v)?;
        }
        if let Some(v) = set("IMP_SKILLS_DIR") {
            self.skills_dir = Some(PathBuf::from(v));
        }
        if let Some(v) = set("IMP_LINKS") {
            self.links = Links::parse(&v)?;
        }
        if let Some(v) = set("IMP_COLOR") {
            self.color = ColorMode::parse(&v)?;
        }
        if set("NO_COLOR").is_some() {
            self.color = ColorMode::Never;
        }
        Ok(())
    }
}

/// State and config directories without pulling in a crate.
pub fn dirs_config() -> PathBuf {
    env_dir("XDG_CONFIG_HOME", ".config")
}

pub fn dirs_state() -> PathBuf {
    env_dir("XDG_STATE_HOME", ".local/state")
}

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    if let Some(v) = std::env::var_os(var) {
        return PathBuf::from(v);
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(fallback)
}

/// Resolve a session root. `None` keeps the base. A leading `~` or `~/`
/// expands to HOME, a relative path joins the base, and the result must be an
/// existing directory.
pub fn resolve_root(raw: Option<&str>, base: &Path) -> Result<PathBuf, String> {
    let Some(raw) = raw else {
        return Ok(base.to_path_buf());
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("empty path".into());
    }
    let expanded = if raw == "~" || raw.starts_with("~/") {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| "HOME is not set".to_string())?;
        PathBuf::from(home).join(raw.trim_start_matches('~').trim_start_matches('/'))
    } else {
        PathBuf::from(raw)
    };
    let path = if expanded.is_absolute() {
        expanded
    } else {
        base.join(expanded)
    };
    let resolved = path
        .canonicalize()
        .map_err(|_| format!("{}: not a directory", path.display()))?;
    if !resolved.is_dir() {
        return Err(format!("{}: not a directory", path.display()));
    }
    Ok(resolved)
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("expected a boolean, got {s:?}")),
    }
}

fn parse_u32(key: &str, s: &str) -> Result<u32, String> {
    s.trim()
        .parse()
        .map_err(|_| format!("{key} must be a number, got {s:?}"))
}

/// Parse the per-turn tool-step limit. Below 1 is refused; above 1000 is
/// clamped.
pub fn parse_max_steps(s: &str) -> Result<usize, String> {
    let n: i64 = s
        .trim()
        .parse()
        .map_err(|_| format!("max_steps must be a number, got {s:?}"))?;
    if n < 1 {
        return Err("max_steps must be at least 1".into());
    }
    Ok((n as usize).min(1000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn parses_values_and_ignores_noise() {
        let cfg = Config::from_str(
            "# comment\n\nmodel = deepseek-v4-pro\npermissions=strict\nallow_sudo=yes\n",
        )
        .unwrap();
        assert_eq!(cfg.model, "deepseek-v4-pro");
        assert_eq!(cfg.permissions, Permissions::Strict);
        assert!(cfg.allow_sudo);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        assert!(Config::from_str("nonsense=1").is_ok());
    }

    #[test]
    fn bad_enum_is_an_error() {
        let err = Config::from_str("permissions=whatever").unwrap_err();
        assert!(err.contains("normal"), "{err}");
    }

    #[test]
    fn bad_bool_is_an_error() {
        assert!(Config::from_str("allow_sudo=perhaps").is_err());
    }

    #[test]
    fn links_setting_parses() {
        assert_eq!(Config::from_str("links=never").unwrap().links, Links::Never);
        assert_eq!(Config::from_str("links=always").unwrap().links, Links::Always);
        assert!(Config::from_str("links=sometimes").is_err());
    }

    #[test]
    fn missing_equals_is_an_error() {
        assert!(Config::from_str("model").is_err());
    }

    #[test]
    fn config_path_is_under_imp() {
        assert!(Config::config_path().ends_with("imp/config"));
    }

    #[test]
    fn env_overrides_file() {
        let mut cfg = Config::from_str("model=from-file\n").unwrap();
        cfg.apply_env(env(&[("IMP_MODEL", "from-env")])).unwrap();
        assert_eq!(cfg.model, "from-env");
    }

    #[test]
    fn no_color_forces_never() {
        let mut cfg = Config::default();
        cfg.apply_env(env(&[("NO_COLOR", "1")])).unwrap();
        assert_eq!(cfg.color, ColorMode::Never);
    }

    #[test]
    fn max_steps_parses_and_bounds() {
        let cfg = Config::from_str("max_steps=5\n").unwrap();
        assert_eq!(cfg.max_steps, 5);
        assert!(Config::from_str("max_steps=0\n").is_err());
        assert_eq!(parse_max_steps("5000").unwrap(), 1000);
        let mut cfg = Config::default();
        cfg.apply_env(env(&[("IMP_MAX_STEPS", "2")])).unwrap();
        assert_eq!(cfg.max_steps, 2);
    }

    #[test]
    fn base_url_trailing_slash_is_trimmed() {
        let cfg = Config::from_str("base_url=https://example.test/\n").unwrap();
        assert_eq!(cfg.base_url, "https://example.test");
    }

    #[test]
    fn provider_profiles_supply_defaults() {
        let openai = Config::from_str("provider=openai\n").unwrap();
        assert_eq!(openai.provider, Provider::OpenAi);
        assert_eq!(openai.base_url, "https://api.openai.com/v1");
        assert_eq!(openai.model, "gpt-4.1-mini");

        let anthropic = Config::from_str("provider=anthropic\n").unwrap();
        assert_eq!(anthropic.base_url, "https://api.anthropic.com");

        let gemini = Config::from_str("provider=gemini\n").unwrap();
        assert_eq!(gemini.base_url, "https://generativelanguage.googleapis.com");

        let ollama = Config::from_str("provider=ollama\n").unwrap();
        assert_eq!(ollama.base_url, "http://localhost:11434/v1");
        assert!(ollama.provider.profile().key_var.is_none());
    }

    #[test]
    fn explicit_model_and_base_beat_the_profile() {
        let cfg = Config::from_str(
            "provider=openai\nmodel=mine\nbase_url=https://example.test/v1/\n",
        )
        .unwrap();
        assert_eq!(cfg.model, "mine");
        assert_eq!(cfg.base_url, "https://example.test/v1");
    }

    #[test]
    fn a_profile_switch_keeps_an_explicit_model() {
        let mut cfg = Config::from_str("model=mine\n").unwrap();
        cfg.set_provider(Provider::Ollama);
        assert_eq!(cfg.model, "mine");
        assert_eq!(cfg.base_url, "http://localhost:11434/v1");

        let mut fresh = Config::default();
        fresh.set_provider(Provider::Ollama);
        assert_eq!(fresh.model, "llama3.2");
    }

    #[test]
    fn imp_provider_env_selects_the_profile() {
        let mut cfg = Config::default();
        cfg.apply_env(env(&[("IMP_PROVIDER", "gemini")])).unwrap();
        assert_eq!(cfg.provider, Provider::Gemini);
        assert_eq!(cfg.model, "gemini-2.5-pro");
        cfg.apply_env(env(&[("IMP_PROVIDER", "nowhere")]))
            .expect_err("an unknown provider must be refused");
    }

    #[test]
    fn key_order_is_config_then_imp_then_provider() {
        let mut cfg = Config::from_str("api_key=from-config\n").unwrap();
        cfg.resolve_key_env(env(&[
            ("IMP_API_KEY", "from-imp"),
            ("DEEPSEEK_API_KEY", "from-provider"),
        ]))
        .unwrap();
        assert_eq!(cfg.api_key, "from-config");

        let mut cfg = Config::default();
        cfg.resolve_key_env(env(&[
            ("IMP_API_KEY", "from-imp"),
            ("DEEPSEEK_API_KEY", "from-provider"),
        ]))
        .unwrap();
        assert_eq!(cfg.api_key, "from-imp");

        let mut cfg = Config::default();
        cfg.resolve_key_env(env(&[("DEEPSEEK_API_KEY", "from-provider")]))
            .unwrap();
        assert_eq!(cfg.api_key, "from-provider");
    }

    #[test]
    fn a_missing_key_names_its_variable() {
        let mut cfg = Config::from_str("provider=anthropic\n").unwrap();
        let err = cfg.resolve_key_env(env(&[])).unwrap_err();
        assert!(err.contains("ANTHROPIC_API_KEY"), "{err}");

        let mut cfg = Config::from_str("provider=deepseek\n").unwrap();
        let err = cfg.resolve_key_env(env(&[])).unwrap_err();
        assert!(err.contains("DEEPSEEK_API_KEY"), "{err}");
    }

    #[test]
    fn local_profiles_need_no_key() {
        for name in ["ollama", "custom"] {
            let mut cfg = Config::from_str(&format!("provider={name}\n")).unwrap();
            cfg.resolve_key_env(env(&[])).unwrap();
            assert!(cfg.api_key.is_empty());
        }
    }

    #[test]
    fn root_resolver_handles_tilde_relative_and_missing() {
        let base = std::env::temp_dir();
        assert_eq!(resolve_root(None, &base).unwrap(), base);
        assert_eq!(
            resolve_root(Some("."), &base).unwrap(),
            base.canonicalize().unwrap()
        );
        let home = std::env::var_os("HOME").expect("HOME");
        assert_eq!(
            resolve_root(Some("~"), &base).unwrap(),
            PathBuf::from(home).canonicalize().unwrap()
        );
        assert!(resolve_root(Some("/nope/imp-missing"), &base).is_err());

        let file = base.join(format!("imp-root-file-{}", std::process::id()));
        std::fs::write(&file, "x").unwrap();
        assert!(resolve_root(Some(file.to_str().unwrap()), &base).is_err());
        let _ = std::fs::remove_file(file);
    }
}
