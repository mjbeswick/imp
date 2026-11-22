//! Configuration: plain `key=value` file, environment wins.

use crate::providers::Provider;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

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

    /// A stable byte for the shared mode.
    pub fn code(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Strict => 1,
            Self::Yolo => 2,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Strict,
            2 => Self::Yolo,
            _ => Self::Normal,
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

/// A custom provider whose fields live in the config's managed block.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NamedProvider {
    pub name: String,
    pub base: String,
    pub model: String,
    pub key_env: String,
    pub dialect: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: Provider,
    /// The profile named in `provider=`: a built-in name or a custom one.
    pub active_name: String,
    /// Custom profiles parsed from the managed block.
    pub named: Vec<NamedProvider>,
    /// Environment variable named by the active custom profile.
    pub key_env: Option<String>,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub permissions: Permissions,
    /// The live mode, shared with the worker so a change applies mid-turn.
    pub mode: Arc<AtomicU8>,
    pub allow_sudo: bool,
    /// Command shapes answered `a`: shared by every session, saved in the file.
    pub allow: Arc<std::sync::Mutex<Vec<String>>>,
    /// The file this was loaded from; `None` keeps `allow` in memory only.
    pub file: Option<PathBuf>,
    pub prune_at: u32,
    pub compact_at: u32,
    pub max_tokens: u32,
    pub max_steps: usize,
    /// Rounds a `/goal` may run before it stops.
    pub goal_rounds: u32,
    pub thinking: bool,
    pub color: ColorMode,
    /// Capture the mouse wheel in the chat.
    pub mouse: bool,
    /// Offer a likely next message after each turn in the chat.
    pub suggest: bool,
    /// Show the lines an edit or write changed under its tool row.
    pub diffs: bool,
    /// Index into `SPINNERS`.
    pub spinner: usize,
    /// Move to the next waiting session after handing one work.
    pub auto_next: bool,
    /// Secret-looking variables the agent's commands may still see.
    pub pass_env: Vec<String>,
    /// How much of each tool step the interface shows.
    pub verbosity: Verbosity,
    /// Credentials by host: `host[:port]` and a header template naming
    /// environment variables.
    pub auth: Vec<(String, String)>,
    /// Commands that draw fenced blocks of a language in replies.
    pub render: Vec<(String, String)>,
    pub links: Links,
    pub skills_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        let profile = Provider::DeepSeek.profile();
        Self {
            provider: Provider::DeepSeek,
            active_name: "deepseek".into(),
            named: Vec::new(),
            key_env: None,
            api_key: String::new(),
            model: profile.model.into(),
            base_url: profile.base_url.into(),
            permissions: Permissions::Normal,
            mode: Arc::new(AtomicU8::new(Permissions::Normal.code())),
            allow_sudo: false,
            allow: Arc::default(),
            file: None,
            prune_at: 45_000,
            compact_at: 60_000,
            max_tokens: 8192,
            max_steps: 24,
            goal_rounds: 5,
            thinking: false,
            color: ColorMode::Auto,
            mouse: true,
            suggest: true,
            diffs: true,
            spinner: 0,
            auto_next: true,
            pass_env: Vec::new(),
            verbosity: Verbosity::Medium,
            auth: Vec::new(),
            render: Vec::new(),
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
        cfg.file = Some(path.to_path_buf());
        cfg.apply_env(|k| std::env::var(k).ok())?;
        Ok(cfg)
    }

    pub fn from_str(text: &str) -> Result<Self, String> {
        let mut cfg = Self::default();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let Some(rest) = k.trim().strip_prefix("provider.") else {
                continue;
            };
            let Some((name, field)) = rest.rsplit_once('.') else {
                continue;
            };
            set_named_field(&mut cfg.named, name, field, v.trim());
        }
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

    pub(crate) fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "provider" => self.set_active(value)?,
            "api_key" => self.api_key = value.to_string(),
            "model" => self.model = value.to_string(),
            "base_url" => self.base_url = value.trim_end_matches('/').to_string(),
            "permissions" => self.set_permissions(Permissions::parse(value)?),
            "allow_sudo" => self.allow_sudo = parse_bool(value)?,
            "allow" => self.allows(value),
            "verbosity" => self.verbosity = Verbosity::parse(value)?,
            "auth" => {
                let (host, header) = value.trim().split_once(char::is_whitespace).unwrap_or((value, ""));
                if !header.contains(':') {
                    return Err(format!("auth needs a host and a header, such as `auth = {host} Authorization: Bearer $TOKEN`"));
                }
                self.auth.push((host.to_ascii_lowercase(), header.trim().to_string()));
            }
            "pass_env" => self.pass_env.extend(value.split(',').map(str::trim).filter(|n| !n.is_empty()).map(String::from)),
            "prune_at" => self.prune_at = parse_u32(key, value)?,
            "compact_at" => self.compact_at = parse_u32(key, value)?,
            "max_tokens" => self.max_tokens = parse_u32(key, value)?,
            "max_steps" => self.max_steps = parse_max_steps(value)?,
            "goal_rounds" => {
                self.goal_rounds = parse_u32("goal_rounds", value)?;
                if !(1..=20).contains(&self.goal_rounds) {
                    return Err(format!("goal_rounds is 1 to 20, got {}", self.goal_rounds));
                }
            }
            "thinking" => self.thinking = parse_bool(value)?,
            "links" => self.links = Links::parse(value)?,
            "color" => self.color = ColorMode::parse(value)?,
            "mouse" => self.mouse = parse_bool(value)?,
            "suggest" => self.suggest = parse_bool(value)?,
            "diffs" => self.diffs = parse_bool(value)?,
            "spinner" => {
                self.spinner = SPINNERS.iter().position(|(n, _)| *n == value.trim()).ok_or_else(|| {
                    let names: Vec<&str> = SPINNERS.iter().map(|(n, _)| *n).collect();
                    format!("unknown spinner {value:?}; use {}", names.join(", "))
                })?
            }
            "auto_next" => self.auto_next = parse_bool(value)?,
            "skills_dir" => self.skills_dir = Some(PathBuf::from(value)),
            // Managed profile lines were read before this pass.
            key if key.starts_with("provider.") => {}
            key if key.starts_with("render.") => {
                let lang = key["render.".len()..].trim().to_ascii_lowercase();
                if lang.is_empty() || value.trim().is_empty() {
                    return Err("render needs a language and a command, such as `render.mermaid = mmd`".into());
                }
                self.render.retain(|(l, _)| *l != lang);
                self.render.push((lang, value.trim().to_string()));
            }
            // Unknown keys are ignored so newer configs do not break older runs.
            _ => {}
        }
        Ok(())
    }

    /// Set the mode in one place so the shared byte cannot drift.
    pub fn set_permissions(&mut self, mode: Permissions) {
        self.permissions = mode;
        self.mode
            .store(mode.code(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Whether a command shape was answered `a`.
    pub fn allowed(&self, shape: &str) -> bool {
        self.allow.lock().is_ok_and(|a| a.iter().any(|s| s == shape))
    }

    fn allows(&self, shape: &str) {
        if let Ok(mut a) = self.allow.lock() {
            if !shape.is_empty() && !a.iter().any(|s| s == shape) {
                a.push(shape.to_string());
            }
        }
    }

    /// Allow a shape in every session, and append it to the config file.
    pub fn allow_always(&self, shape: &str) -> Result<(), String> {
        self.allows(shape);
        let Some(path) = &self.file else { return Ok(()) };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let sep = if text.is_empty() || text.ends_with('\n') { "" } else { "\n" };
        use std::io::Write;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| write!(f, "{sep}allow = {shape}\n"))
            .map_err(|e| e.to_string())
    }

    /// A copy whose mode no longer follows this one: another session's
    /// `/mode` must not change it.
    pub fn detached(&self) -> Self {
        let mut copy = self.clone();
        copy.mode = Arc::new(AtomicU8::new(self.live_permissions().code()));
        copy
    }

    /// The mode as of this read, not as of the session's start.
    pub fn live_permissions(&self) -> Permissions {
        Permissions::from_code(self.mode.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Select the profile named in `provider=`: a built-in or a custom one.
    pub fn set_active(&mut self, name: &str) -> Result<(), String> {
        if let Ok(provider) = Provider::parse(name) {
            self.key_env = None;
            self.set_provider(provider);
            self.active_name = name.trim().to_ascii_lowercase();
            return Ok(());
        }
        let Some(profile) = self.named.iter().find(|n| n.name == name).cloned() else {
            return Err(format!(
                "unknown provider {name:?}; add it with /provider add {name}"
            ));
        };
        self.apply_named(&profile)
    }

    /// Apply a custom profile's endpoint, model, and dialect.
    pub fn apply_named(&mut self, profile: &NamedProvider) -> Result<(), String> {
        let dialect = Provider::parse(&profile.dialect).map_err(|_| {
            format!(
                "{}: dialect must be deepseek, openai, anthropic, gemini, ollama, or custom",
                profile.name
            )
        })?;
        self.provider = dialect;
        if !profile.base.is_empty() {
            self.base_url = profile.base.trim_end_matches('/').to_string();
        }
        if !profile.model.is_empty() {
            self.model = profile.model.clone();
        }
        self.key_env = if profile.key_env.is_empty() {
            None
        } else {
            Some(profile.key_env.clone())
        };
        self.active_name = profile.name.clone();
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

    /// Fill the key in order: `IMP_API_KEY`, the profile's variable, then the
    /// config `api_key`. Providers that need none are left alone.
    pub fn resolve_key_env(
        &mut self,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Result<(), String> {
        let filled = |v: String| !v.trim().is_empty();
        if let Some(v) = lookup("IMP_API_KEY").filter(|v| filled(v.clone())) {
            self.api_key = v;
            return Ok(());
        }
        let var = self
            .key_env
            .clone()
            .or_else(|| self.provider.profile().key_var.map(str::to_string));
        if let Some(var) = &var {
            if let Some(v) = lookup(var).filter(|v| filled(v.clone())) {
                self.api_key = v;
                return Ok(());
            }
        }
        // A key in the config still counts, but anything exported wins.
        if filled(self.api_key.clone()) {
            return Ok(());
        }
        match var {
            Some(var) => Err(format!("{var} is not set")),
            None => Ok(()),
        }
    }

    pub fn apply_env(&mut self, lookup: impl Fn(&str) -> Option<String>) -> Result<(), String> {
        let set = |k: &str| lookup(k);
        if let Some(v) = set("IMP_PROVIDER") {
            self.set_active(&v)?;
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

/// Start of the managed provider block in the config file.
pub const PROVIDERS_START: &str = "# >>> providers";
/// End of the managed provider block in the config file.
pub const PROVIDERS_END: &str = "# <<< providers";

fn set_named_field(named: &mut Vec<NamedProvider>, name: &str, field: &str, value: &str) {
    let at = match named.iter().position(|n| n.name == name) {
        Some(at) => at,
        None => {
            named.push(NamedProvider {
                name: name.to_string(),
                ..NamedProvider::default()
            });
            named.len() - 1
        }
    };
    let profile = &mut named[at];
    match field {
        "base" => profile.base = value.to_string(),
        "model" => profile.model = value.to_string(),
        "key_env" => profile.key_env = value.to_string(),
        "dialect" => profile.dialect = value.to_string(),
        _ => {}
    }
}

/// A valid custom profile name.
pub fn valid_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 40
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The managed block, markers included, with one group per profile.
pub fn providers_block(named: &[NamedProvider]) -> String {
    let mut out = String::new();
    out.push_str(PROVIDERS_START);
    out.push('\n');
    for profile in named {
        out.push_str(&format!("provider.{}.base = {}\n", profile.name, profile.base));
        out.push_str(&format!(
            "provider.{}.model = {}\n",
            profile.name, profile.model
        ));
        out.push_str(&format!(
            "provider.{}.key_env = {}\n",
            profile.name, profile.key_env
        ));
        out.push_str(&format!(
            "provider.{}.dialect = {}\n\n",
            profile.name, profile.dialect
        ));
    }
    out.push_str(PROVIDERS_END);
    out.push('\n');
    out
}

/// Replace the managed block and leave every other byte of the file alone.
pub fn save_named(path: &Path, named: &[NamedProvider]) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let block = providers_block(named);
    let updated = match (text.find(PROVIDERS_START), text.find(PROVIDERS_END)) {
        (Some(start), Some(end)) if end > start => {
            let after = end + PROVIDERS_END.len();
            let after = text[after..]
                .strip_prefix('\n')
                .map(|_| after + 1)
                .unwrap_or(after);
            format!("{}{}{}", &text[..start], block, &text[after..])
        }
        _ => {
            let mut out = text;
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&block);
            out
        }
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, updated).map_err(|e| e.to_string())
}

/// Stored provider keys: `NAME=value`, one per line, owner only.
pub fn keys_path() -> PathBuf {
    dirs_config().join("imp").join("keys")
}

/// A key file as pairs, skipping comments, blanks, and lines without a value.
fn read_keys(path: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .collect()
}

/// The keys file, read when it exists.
pub fn load_keys() -> Vec<(String, String)> {
    read_keys(&keys_path())
}

/// Store one key, replacing its line, and keep the file to its owner.
fn write_key(path: &Path, name: &str, value: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut lines: Vec<String> = std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let name = name.trim();
    lines.retain(|line| match line.split_once('=') {
        Some((key, _)) => key.trim() != name,
        None => true,
    });
    lines.push(format!("{name}={}", value.trim()));
    std::fs::write(path, format!("{}\n", lines.join("\n"))).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn save_key(name: &str, value: &str) -> Result<(), String> {
    write_key(&keys_path(), name, value)
}

/// Environment first, then the stored keys, as one lookup.
pub fn key_lookup() -> impl Fn(&str) -> Option<String> {
    let stored = load_keys();
    move |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                stored
                    .iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.clone())
            })
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

/// Spinner names and frames. `random` draws a frame at random from its set.
// ponytail: single-width glyphs only; emoji frames are two cells and jitter rows.
pub const SPINNERS: &[(&str, &str)] = &[
    ("circle", "◐◓◑◒"),
    ("braille", "⠋⠙⠹⠸⠼⠴⠦⠧"),
    ("dots", "⣾⣽⣻⢿⡿⣟⣯⣷"),
    ("line", "|/-\\"),
    ("arc", "◜◠◝◞◡◟"),
    ("square", "◰◳◲◱"),
    ("triangle", "◢◣◤◥"),
    ("pulse", "·•●•"),
    ("bar", "▁▂▃▄▅▆▇█▇▆▅▄▃▂"),
    ("star", "✶✸✹✺✹✷"),
    // No digits or letters: rows trim leading frames, and counts start with digits.
    ("random", "@#$%&*+=?~^!"),
];

/// How much of each tool step the interface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    /// One line per step.
    Low,
    /// The line and a short result.
    Medium,
    /// The arguments and capped output as well.
    High,
}

impl Verbosity {
    pub const ALL: [(Verbosity, &'static str); 3] = [
        (Verbosity::Low, "one line per step"),
        (Verbosity::Medium, "each step and its result"),
        (Verbosity::High, "arguments and output"),
    ];

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(format!("unknown verbosity {other:?}; use low, medium, or high")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
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
    fn spinner_names_parse_and_frames_are_one_cell() {
        assert_eq!(SPINNERS[Config::from_str("spinner=random").unwrap().spinner].0, "random");
        assert!(Config::from_str("spinner=nope").unwrap_err().contains("braille"));
        for (name, frames) in SPINNERS {
            // Wide glyphs sit outside these ranges; one cell keeps rows still.
            assert!(frames.chars().all(|c| (c as u32) < 0x2E80), "{name} has a wide frame");
            assert!(!frames.chars().any(|c| c.is_alphanumeric()), "{name} could eat a count");
        }
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
    fn key_order_is_imp_then_provider_then_config() {
        let mut cfg = Config::from_str("api_key=from-config\n").unwrap();
        cfg.resolve_key_env(env(&[
            ("IMP_API_KEY", "from-imp"),
            ("DEEPSEEK_API_KEY", "from-provider"),
        ]))
        .unwrap();
        assert_eq!(cfg.api_key, "from-imp");

        let mut cfg = Config::from_str("api_key=from-config\n").unwrap();
        cfg.resolve_key_env(env(&[("DEEPSEEK_API_KEY", "from-provider")]))
            .unwrap();
        assert_eq!(cfg.api_key, "from-provider");

        let mut cfg = Config::from_str("api_key=from-config\n").unwrap();
        cfg.resolve_key_env(env(&[])).unwrap();
        assert_eq!(cfg.api_key, "from-config");
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
    fn a_cloned_config_shares_the_live_mode() {
        let cfg = Config::default();
        let clone = cfg.clone();
        cfg.mode
            .store(Permissions::Yolo.code(), std::sync::atomic::Ordering::Relaxed);
        assert_eq!(clone.live_permissions(), Permissions::Yolo);
        clone
            .mode
            .store(Permissions::Strict.code(), std::sync::atomic::Ordering::Relaxed);
        assert_eq!(cfg.live_permissions(), Permissions::Strict);
    }

    #[test]
    fn a_custom_profile_resolves_before_the_builtins() {
        let text = "provider=work\n\n# >>> providers\nprovider.work.base = http://box:8080/v1\nprovider.work.model = local-coder\nprovider.work.key_env = WORK_KEY\nprovider.work.dialect = openai\n# <<< providers\n";
        let mut cfg = Config::from_str(text).unwrap();
        assert_eq!(cfg.active_name, "work");
        assert_eq!(cfg.provider, Provider::OpenAi);
        assert_eq!(cfg.base_url, "http://box:8080/v1");
        assert_eq!(cfg.model, "local-coder");
        assert_eq!(cfg.key_env.as_deref(), Some("WORK_KEY"));
        let err = cfg.resolve_key_env(env(&[])).unwrap_err();
        assert!(err.contains("WORK_KEY"), "{err}");
        cfg.resolve_key_env(env(&[("WORK_KEY", "secret")])).unwrap();
        assert_eq!(cfg.api_key, "secret");
    }

    #[test]
    fn keys_are_stored_owner_only_and_replaced() {
        let dir = std::env::temp_dir().join(format!("imp-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("keys");
        write_key(&path, "DEEPSEEK_API_KEY", "first").unwrap();
        write_key(&path, "OTHER_KEY", "two").unwrap();
        write_key(&path, "DEEPSEEK_API_KEY", "second").unwrap();
        assert_eq!(
            read_keys(&path),
            vec![
                ("OTHER_KEY".to_string(), "two".to_string()),
                ("DEEPSEEK_API_KEY".to_string(), "second".to_string()),
            ]
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "keys file mode {mode:o}");
        }
    }

    #[test]
    fn a_key_file_skips_comments_and_blanks() {
        let dir = std::env::temp_dir().join(format!("imp-keys-parse-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("keys");
        std::fs::write(&path, "# a key\n\n A_KEY = v1 \nbroken\nB_KEY = a=b\n").unwrap();
        assert_eq!(
            read_keys(&path),
            vec![
                ("A_KEY".to_string(), "v1".to_string()),
                ("B_KEY".to_string(), "a=b".to_string()),
            ]
        );
    }

    #[test]
    fn saving_profiles_keeps_comments_and_other_keys() {
        let dir = std::env::temp_dir().join(format!("imp-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(
            &path,
            "# keep me\nmodel = old\n\n# >>> providers\nprovider.gone.base = x\n# <<< providers\n\nthinking = true\n",
        )
        .unwrap();
        let named = vec![NamedProvider {
            name: "work".into(),
            base: "http://box/v1".into(),
            model: "m".into(),
            key_env: "K".into(),
            dialect: "openai".into(),
        }];
        save_named(&path, &named).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# keep me\nmodel = old\n"), "{text}");
        assert!(text.contains("provider.work.base = http://box/v1"), "{text}");
        assert!(!text.contains("provider.gone"), "{text}");
        assert!(text.ends_with("thinking = true\n"), "{text}");
        let cfg = Config::from_str(&text).unwrap();
        assert_eq!(cfg.named, named);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn auth_lines_bind_a_header_to_a_host() {
        let mut cfg = Config::default();
        cfg.set("auth", "API.github.com Authorization: Bearer $GITHUB_TOKEN").unwrap();
        assert_eq!(cfg.auth, [("api.github.com".to_string(), "Authorization: Bearer $GITHUB_TOKEN".to_string())]);
        assert!(cfg.set("auth", "api.github.com").is_err());
    }

    #[test]
    fn render_lines_name_a_command_per_language() {
        let mut cfg = Config::default();
        cfg.set("render.Mermaid", "mmd --color").unwrap();
        cfg.set("render.mermaid", "mmd").unwrap();
        assert_eq!(cfg.render, [("mermaid".to_string(), "mmd".to_string())]);
        assert!(cfg.set("render.d2", " ").is_err());
    }

    #[test]
    fn pass_env_lists_accumulate() {
        let mut cfg = Config::default();
        cfg.set("pass_env", "GH_TOKEN, NPM_TOKEN").unwrap();
        cfg.set("pass_env", "HF_TOKEN").unwrap();
        assert_eq!(cfg.pass_env, ["GH_TOKEN", "NPM_TOKEN", "HF_TOKEN"]);
    }

    #[test]
    fn allow_lines_load_append_and_survive_a_profile_save() {
        let dir = std::env::temp_dir().join(format!("imp-allow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "# mine\nallow = cargo build").unwrap();
        let cfg = Config::load(&path).unwrap();
        assert!(cfg.allowed("cargo build"));
        assert!(!cfg.allowed("npm test"));
        let other = cfg.detached();
        cfg.allow_always("npm test").unwrap();
        assert!(other.allowed("npm test"), "another session did not see it");
        save_named(&path, &[]).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# mine\nallow = cargo build\nallow = npm test\n"), "{text}");
        assert!(Config::load(&path).unwrap().allowed("npm test"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_active_name_is_an_error() {
        let err = Config::from_str("provider=work\n").unwrap_err();
        assert!(err.contains("unknown provider"), "{err}");
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
