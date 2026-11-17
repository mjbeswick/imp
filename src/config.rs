//! Configuration: plain `key=value` file, environment wins.

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
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub permissions: Permissions,
    pub allow_sudo: bool,
    pub prune_at: u32,
    pub compact_at: u32,
    pub max_tokens: u32,
    pub thinking: bool,
    pub color: ColorMode,
    pub skills_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "deepseek-flash".into(),
            base_url: "https://api.deepseek.com".into(),
            permissions: Permissions::Normal,
            allow_sudo: false,
            prune_at: 45_000,
            compact_at: 60_000,
            max_tokens: 8192,
            thinking: false,
            color: ColorMode::Auto,
            skills_dir: None,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs_config().join("mote").join("config")
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
            "api_key" => self.api_key = value.to_string(),
            "model" => self.model = value.to_string(),
            "base_url" => self.base_url = value.trim_end_matches('/').to_string(),
            "permissions" => self.permissions = Permissions::parse(value)?,
            "allow_sudo" => self.allow_sudo = parse_bool(value)?,
            "prune_at" => self.prune_at = parse_u32(key, value)?,
            "compact_at" => self.compact_at = parse_u32(key, value)?,
            "max_tokens" => self.max_tokens = parse_u32(key, value)?,
            "thinking" => self.thinking = parse_bool(value)?,
            "color" => self.color = ColorMode::parse(value)?,
            "skills_dir" => self.skills_dir = Some(PathBuf::from(value)),
            // Unknown keys are ignored so newer configs do not break older runs.
            _ => {}
        }
        Ok(())
    }

    pub fn apply_env(&mut self, lookup: impl Fn(&str) -> Option<String>) -> Result<(), String> {
        let set = |k: &str| lookup(k);
        if let Some(v) = set("DEEPSEEK_API_KEY") {
            self.api_key = v;
        }
        if let Some(v) = set("MOTE_MODEL") {
            self.model = v;
        }
        if let Some(v) = set("MOTE_BASE_URL") {
            self.base_url = v.trim_end_matches('/').to_string();
        }
        if let Some(v) = set("MOTE_PERMISSIONS") {
            self.permissions = Permissions::parse(&v)?;
        }
        if let Some(v) = set("MOTE_ALLOW_SUDO") {
            self.allow_sudo = parse_bool(&v)?;
        }
        if let Some(v) = set("MOTE_PRUNE_AT") {
            self.prune_at = parse_u32("MOTE_PRUNE_AT", &v)?;
        }
        if let Some(v) = set("MOTE_COMPACT_AT") {
            self.compact_at = parse_u32("MOTE_COMPACT_AT", &v)?;
        }
        if let Some(v) = set("MOTE_MAX_TOKENS") {
            self.max_tokens = parse_u32("MOTE_MAX_TOKENS", &v)?;
        }
        if let Some(v) = set("MOTE_SKILLS_DIR") {
            self.skills_dir = Some(PathBuf::from(v));
        }
        if let Some(v) = set("MOTE_COLOR") {
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
    fn missing_equals_is_an_error() {
        assert!(Config::from_str("model").is_err());
    }

    #[test]
    fn env_overrides_file() {
        let mut cfg = Config::from_str("model=from-file\n").unwrap();
        cfg.apply_env(env(&[("MOTE_MODEL", "from-env")])).unwrap();
        assert_eq!(cfg.model, "from-env");
    }

    #[test]
    fn no_color_forces_never() {
        let mut cfg = Config::default();
        cfg.apply_env(env(&[("NO_COLOR", "1")])).unwrap();
        assert_eq!(cfg.color, ColorMode::Never);
    }

    #[test]
    fn base_url_trailing_slash_is_trimmed() {
        let cfg = Config::from_str("base_url=https://example.test/\n").unwrap();
        assert_eq!(cfg.base_url, "https://example.test");
    }
}
