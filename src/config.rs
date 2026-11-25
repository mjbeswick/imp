//! Configuration: plain `key=value` file, environment wins.

use crate::providers::Provider;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permissions {
    Normal,
    Strict,
    /// Runs the project's development commands without asking.
    Auto,
    Yolo,
}

impl Permissions {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Ok(Self::Normal),
            "strict" => Ok(Self::Strict),
            "auto" => Ok(Self::Auto),
            "yolo" => Ok(Self::Yolo),
            _ => Err(format!(
                "permissions must be normal, strict, auto, or yolo (got {s:?})"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Strict => "strict",
            Self::Auto => "auto",
            Self::Yolo => "yolo",
        }
    }

    /// A stable byte for the shared mode.
    pub fn code(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Strict => 1,
            Self::Yolo => 2,
            Self::Auto => 3,
        }
    }

    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Strict,
            2 => Self::Yolo,
            3 => Self::Auto,
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

/// Keys that imp needs at once, so they cannot start a chord or be bound.
const RESERVED_KEYS: &[&str] = &["ctrl+c", "esc", "enter"];

/// Parse a chord from its config form: one or two keys separated by `-`, where
/// `ctrl` applies to the key after it, as in `ctrl-x-n`. Returns the keys as
/// `ctrl+x` and `n`.
pub fn parse_chord(text: &str) -> Result<Vec<String>, String> {
    let tokens: Vec<&str> = text.split('-').filter(|t| !t.is_empty()).collect();
    let mut keys: Vec<String> = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let key = if tokens[i] == "ctrl" {
            let next = tokens.get(i + 1).ok_or_else(|| format!("ctrl needs a key in {text:?}"))?;
            i += 2;
            format!("ctrl+{next}")
        } else {
            i += 1;
            tokens[i - 1].to_string()
        };
        keys.push(key);
    }
    if keys.is_empty() || keys.len() > 2 {
        return Err(format!("a chord has one or two keys: {text:?}"));
    }
    if let Some(reserved) = keys.iter().find(|k| RESERVED_KEYS.contains(&k.as_str())) {
        return Err(format!("{reserved} is reserved"));
    }
    Ok(keys)
}

/// A custom provider whose fields live in the config's managed block.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NamedProvider {
    pub name: String,
    pub base: String,
    pub model: String,
    /// Further models this provider offers, besides its default `model`.
    pub models: Vec<String>,
    pub key_env: String,
    pub dialect: String,
    /// The API key, saved in the config. Empty when the key comes from `key_env`.
    pub api_key: String,
}

impl NamedProvider {
    /// Every model this provider offers: the default first, then the listed
    /// ones, without repeats.
    pub fn offered_models(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for model in std::iter::once(&self.model).chain(self.models.iter()) {
            if !model.is_empty() && !out.contains(model) {
                out.push(model.clone());
            }
        }
        out
    }
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
    /// Reasoning effort: `low`, `medium` or `high`; `None` is the provider's.
    pub effort: Option<String>,
    /// Command shapes answered `a`: shared by every session, saved in the file.
    pub allow: Arc<std::sync::Mutex<Vec<String>>>,
    /// The file this was loaded from; `None` keeps `allow` in memory only.
    pub file: Option<PathBuf>,
    /// The file's `key = value` lines, in order, for `/reload` to diff.
    pub pairs: Vec<(String, String)>,
    /// Keys the file set that imp does not know; ignored, but named.
    pub unknown: Vec<String>,
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
    /// Shade the title bar from the select colour toward the accent.
    /// The model's context window in tokens, overriding the built-in table.
    pub context_window: Option<u64>,
    /// Summarise a long first message as the session title.
    pub title_summary: TitleSummary,
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
    /// Skills switched off with `skill.<name>.enabled = false`.
    pub disabled_skills: Vec<String>,
    /// Providers switched off with `provider.<name>.enabled = false`.
    pub disabled_providers: Vec<String>,
    /// Providers switched on with `provider.<name>.enabled = true`. A built-in
    /// that needs no key is listed only when it appears here.
    pub enabled_providers: Vec<String>,
    /// Slash command bindings, as `(chord, command)`. The chord is written as in
    /// the config key: keys separated by `-`, with `ctrl` before a key.
    pub bindings: Vec<(String, String)>,
    /// Terminal editor command for `/edit`, e.g. `vim -u NONE`.
    pub editor: Option<String>,
    /// GUI editor command for `/open`, e.g. `code --wait`.
    pub visual: Option<String>,
}

/// The argument list for an editor: program and its arguments, then the target as
/// one argument, so a path with spaces stays whole.
pub fn editor_launch(mut argv: Vec<String>, target: &str) -> Vec<String> {
    argv.push(target.to_string());
    argv
}

impl Config {
    /// The editor command split into program and arguments. The terminal editor
    /// uses `editor` then `$EDITOR`; the GUI editor uses `visual` then `$VISUAL`,
    /// with no default here, so the caller can fall back to the OS opener.
    pub fn editor_argv(&self, lookup: impl Fn(&str) -> Option<String>, gui: bool) -> Option<Vec<String>> {
        let (configured, var) = if gui {
            (self.visual.clone(), "VISUAL")
        } else {
            (self.editor.clone(), "EDITOR")
        };
        let command = configured
            .or_else(|| lookup(var))
            .filter(|c| !c.trim().is_empty())?;
        let argv: Vec<String> = command.split_whitespace().map(str::to_string).collect();
        (!argv.is_empty()).then_some(argv)
    }

    /// Whether a built-in provider can be used now: its key variable is set, or it
    /// needs no key. Uses the same lookup as `resolve_key_env`.
    pub fn provider_configured(&self, provider: Provider, lookup: impl Fn(&str) -> Option<String>) -> bool {
        match provider.profile().key_var {
            None => true,
            Some(var) => lookup(var).is_some_and(|v| !v.trim().is_empty()),
        }
    }

    /// A provider other than the active one that is enabled and usable now: a
    /// built-in whose key is set or needs none, else a named one whose key
    /// variable is set or empty. Used at startup when the active provider has no
    /// key, so the user is not asked for a key they do not need.
    pub fn configured_fallback(&self, lookup: impl Fn(&str) -> Option<String>) -> Option<String> {
        use crate::providers::Provider;
        let built_in = [
            Provider::DeepSeek,
            Provider::OpenAi,
            Provider::Anthropic,
            Provider::Gemini,
            Provider::Ollama,
            Provider::Custom,
        ];
        for provider in built_in {
            let name = format!("{provider:?}").to_lowercase();
            // Only a built-in with its key set counts: a keyless one is not
            // picked by default, so a missing key still gets asked for.
            let keyed = provider.profile().key_var.is_some() && self.provider_configured(provider, &lookup);
            if keyed && name != self.active_name && !self.provider_disabled(&name) {
                return Some(name);
            }
        }
        for named in &self.named {
            let usable = named.key_env.is_empty()
                || !named.api_key.trim().is_empty()
                || lookup(&named.key_env).is_some_and(|v| !v.trim().is_empty());
            if named.name != self.active_name && !self.provider_disabled(&named.name) && usable {
                return Some(named.name.clone());
            }
        }
        None
    }

    /// Whether `provider.<name>.enabled` is set to false.
    pub fn provider_disabled(&self, name: &str) -> bool {
        self.disabled_providers.iter().any(|p| p == name)
    }

    /// Whether `provider.<name>.enabled` was set to true.
    pub fn provider_opted_in(&self, name: &str) -> bool {
        self.enabled_providers.iter().any(|p| p == name)
    }

    /// Whether a built-in is offered in the provider and model lists. The active
    /// provider always is, and a switched-off one never is. A built-in that needs
    /// a key is offered once the key is set. One that needs no key (Ollama,
    /// Custom) is offered only once it is switched on.
    pub fn provider_listed(&self, provider: Provider, lookup: impl Fn(&str) -> Option<String>) -> bool {
        let name = format!("{provider:?}").to_lowercase();
        if name == self.active_name {
            return true;
        }
        if self.provider_disabled(&name) {
            return false;
        }
        match provider.profile().key_var {
            None => self.provider_opted_in(&name),
            Some(_) => self.provider_configured(provider, lookup),
        }
    }

    /// Whether `skill.<name>.enabled` is set to false.
    pub fn skill_disabled(&self, name: &str) -> bool {
        self.disabled_skills.iter().any(|s| s == name)
    }
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
            effort: None,
            allow: Arc::default(),
            file: None,
            pairs: Vec::new(),
            unknown: Vec::new(),
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
            context_window: None,
            title_summary: TitleSummary::Auto,
            pass_env: Vec::new(),
            verbosity: Verbosity::Medium,
            auth: Vec::new(),
            render: Vec::new(),
            links: Links::Auto,
            skills_dir: None,
            disabled_skills: Vec::new(),
            editor: None,
            visual: None,
            disabled_providers: Vec::new(),
            enabled_providers: Vec::new(),
            bindings: Vec::new(),
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
            cfg.pairs.push((k.trim().to_string(), v.trim().to_string()));
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
            "effort" => self.effort = parse_effort(value)?,
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
            "title_summary" => self.title_summary = TitleSummary::parse(value)?,
            "context_window" => {
                let tokens: u64 = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("context_window must be a number, got {value:?}"))?;
                if tokens == 0 {
                    return Err("context_window must be at least 1".into());
                }
                self.context_window = Some(tokens);
            }
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
            "editor" => self.editor = Some(value.trim().to_string()).filter(|v| !v.is_empty()),
            "visual" => self.visual = Some(value.trim().to_string()).filter(|v| !v.is_empty()),
            key if key.starts_with("skill.") && key.ends_with(".enabled") => {
                let name = key["skill.".len()..key.len() - ".enabled".len()].trim().to_string();
                if name.is_empty() {
                    return Err("skill setting needs a name, such as `skill.image.enabled = false`".into());
                }
                let enabled = parse_bool(value)?;
                self.disabled_skills.retain(|s| *s != name);
                if !enabled {
                    self.disabled_skills.push(name);
                }
            }
            key if key.starts_with("provider.") && key.ends_with(".enabled") => {
                let name = key["provider.".len()..key.len() - ".enabled".len()].trim().to_string();
                if name.is_empty() {
                    return Err("provider setting needs a name, such as `provider.openai.enabled = false`".into());
                }
                let enabled = parse_bool(value)?;
                self.disabled_providers.retain(|p| *p != name);
                self.enabled_providers.retain(|p| *p != name);
                if !enabled {
                    self.disabled_providers.push(name);
                } else {
                    self.enabled_providers.push(name);
                }
            }
            // A bad bind line is ignored, so the rest of the file still loads.
            key if key.starts_with("bind.") => {
                let chord = key["bind.".len()..].to_string();
                if parse_chord(&chord).is_ok() && value.starts_with('/') {
                    self.bindings.retain(|(c, _)| *c != chord);
                    self.bindings.push((chord, value.to_string()));
                } else if !self.unknown.iter().any(|u| u == key) {
                    // Named like an unknown key, so the user sees why it did nothing.
                    self.unknown.push(key.to_string());
                }
            }
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
            // Unknown keys are ignored so newer configs do not break older
            // runs, but named so a typo does not pass unseen.
            key => {
                if !self.unknown.iter().any(|u| u == key) {
                    self.unknown.push(key.to_string());
                }
            }
        }
        Ok(())
    }

    /// Apply the keys whose file value changed in `new`, leaving every other
    /// key at its session value. Returns `key: old → new` lines and the keys
    /// the file dropped, which need a restart. `allow` lines are imp's own
    /// and already live, so they are skipped.
    pub fn reload_from(&mut self, new: &Config) -> (Vec<String>, Vec<String>) {
        // The last value of each key, sorted by key.
        let last = |pairs: &[(String, String)]| -> Vec<(String, String)> {
            let mut out: Vec<(String, String)> = Vec::new();
            for (k, v) in pairs.iter().rev().filter(|(k, _)| k != "allow") {
                if !out.iter().any(|(seen, _)| seen == k) {
                    out.push((k.clone(), v.clone()));
                }
            }
            out.sort();
            out
        };
        let (old, now) = (last(&self.pairs), last(&new.pairs));
        let get = |list: &[(String, String)], key: &str| list.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        let mut changed = Vec::new();
        for (key, value) in &now {
            let before = get(&old, key);
            if before.as_ref() == Some(value) {
                continue;
            }
            let result = if key.starts_with("provider.") {
                self.named = new.named.clone();
                Ok(())
            } else {
                self.set(key, value)
            };
            let from = before.as_deref().unwrap_or("unset");
            changed.push(match result {
                Ok(()) => format!("{key}: {from} → {value}"),
                Err(e) => format!("{key}: not applied: {e}"),
            });
        }
        let removed = old.iter().filter(|(k, _)| get(&now, k).is_none()).map(|(k, _)| k.clone()).collect();
        self.pairs = new.pairs.clone();
        self.unknown = new.unknown.clone();
        (changed, removed)
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

    /// Every shape allowed in every session, in file order.
    pub fn always_allows(&self) -> Vec<String> {
        self.allow.lock().map(|a| a.clone()).unwrap_or_default()
    }

    /// Forget a shape allowed in every session, and drop its `allow` line.
    /// Every other byte of the file is left alone.
    pub fn revoke_always(&self, shape: &str) -> Result<(), String> {
        if let Ok(mut a) = self.allow.lock() {
            a.retain(|s| s != shape);
        }
        let Some(path) = &self.file else { return Ok(()) };
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let mut kept = String::new();
        for line in text.split_inclusive('\n') {
            let body = line.trim_end_matches('\n');
            let is_allow = body
                .trim_start()
                .strip_prefix("allow")
                .and_then(|rest| rest.trim_start().strip_prefix('='))
                .is_some_and(|value| value.trim() == shape);
            if !is_allow {
                kept.push_str(line);
            }
        }
        write_private(path, &kept)
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
        // An empty key variable means the provider needs no key; keep that
        // distinct from "unset", which means use the built-in profile's variable.
        self.key_env = Some(profile.key_env.clone());
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
        // A key saved with the active named provider, in the config, is used as typed.
        if let Some(named) = self.named.iter().find(|n| n.name == self.active_name) {
            if !named.api_key.trim().is_empty() {
                self.api_key = named.api_key.clone();
                return Ok(());
            }
        }
        // A named provider with no key variable needs no key.
        if matches!(&self.key_env, Some(v) if v.is_empty()) {
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
            // The bare name: the caller words the message.
            Some(var) => Err(var),
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
        "models" => {
            profile.models = value.split(',').map(str::trim).filter(|m| !m.is_empty()).map(String::from).collect()
        }
        "key_env" => profile.key_env = value.to_string(),
        "api_key" => profile.api_key = value.to_string(),
        "dialect" => profile.dialect = value.to_string(),
        _ => {}
    }
}

/// The built-in provider names a profile may not take.
const BUILT_IN_NAMES: &[&str] = &["deepseek", "openai", "anthropic", "gemini", "ollama", "custom"];

/// The default name for a profile of this dialect: the dialect, or `<dialect>-2`,
/// `<dialect>-3`, and so on, whichever is the first name not already taken.
pub fn default_provider_name(dialect: &str, named: &[NamedProvider]) -> String {
    let taken = |name: &str| BUILT_IN_NAMES.contains(&name) || named.iter().any(|p| p.name.eq_ignore_ascii_case(name));
    if !taken(dialect) {
        return dialect.to_string();
    }
    (2..)
        .map(|n| format!("{dialect}-{n}"))
        .find(|name| !taken(name))
        .unwrap_or_default()
}

/// Check a name before a profile takes it. `editing` is the profile's current
/// name when it is being saved again, so keeping its own name is allowed.
pub fn check_provider_name(candidate: &str, editing: Option<&str>, named: &[NamedProvider]) -> Result<(), String> {
    if !valid_profile_name(candidate) {
        return Err("name must be letters, digits, - or _".into());
    }
    if BUILT_IN_NAMES.contains(&candidate.to_ascii_lowercase().as_str()) {
        return Err(format!("{candidate} is a built-in provider name"));
    }
    if editing == Some(candidate) {
        return Ok(());
    }
    if named.iter().any(|p| p.name.eq_ignore_ascii_case(candidate)) {
        return Err(format!("provider {candidate} already exists"));
    }
    Ok(())
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
        if !profile.models.is_empty() {
            out.push_str(&format!("provider.{}.models = {}\n", profile.name, profile.models.join(", ")));
        }
        out.push_str(&format!(
            "provider.{}.key_env = {}\n",
            profile.name, profile.key_env
        ));
        // The key sits in the config only when one was typed for this provider.
        if !profile.api_key.is_empty() {
            out.push_str(&format!("provider.{}.api_key = {}\n", profile.name, profile.api_key));
        }
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
    write_private(path, &updated)
}

/// `text` with the last uncommented `key=` line set to `key = value`, or the
/// line added at the end. Every other line, comments included, is unchanged.
pub fn set_line(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let at = lines.iter().rposition(|l| {
        let l = l.trim();
        !l.starts_with('#') && l.split_once('=').is_some_and(|(k, _)| k.trim() == key)
    });
    let line = format!("{key} = {value}");
    match at {
        Some(i) => lines[i] = line,
        None => lines.push(line),
    }
    lines.join("\n") + "\n"
}

/// Save one setting to the config file at `path`, creating it if need be.
pub fn save_setting(path: &Path, key: &str, value: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    write_private(path, &set_line(&text, key, value))
}

/// Remove the setting named `key` from the config. Other lines stay as they are.
pub fn remove_setting(path: &Path, key: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| line.split_once('=').map(|(k, _)| k.trim()) != Some(key))
        .collect();
    write_private(path, &(kept.join("\n") + "\n"))
}

/// Remove every setting whose key starts with `prefix`, such as `bind.`.
pub fn remove_settings_with_prefix(path: &Path, prefix: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim_start().starts_with(prefix))
        .collect();
    write_private(path, &(kept.join("\n") + "\n"))
}

/// Write the config so only its owner can read it: it can hold API keys.
fn write_private(path: &Path, text: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, text).map_err(|e| e.to_string())?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
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
/// An effort level, or `None` for `default` or nothing.
pub fn parse_effort(value: &str) -> Result<Option<String>, String> {
    match value.trim() {
        "" | "default" => Ok(None),
        v @ ("low" | "medium" | "high") => Ok(Some(v.to_string())),
        v => Err(format!("effort is low, medium, high, or default (got {v:?})")),
    }
}

/// One line naming config keys imp ignored, or `None` when there are none.
pub fn unknown_note(keys: &[String]) -> Option<String> {
    match keys {
        [] => None,
        [one] => Some(format!("config: unknown key {one} ignored")),
        many => Some(format!("config: unknown keys {} ignored", many.join(", "))),
    }
}

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
    write_private(path, &format!("{}\n", lines.join("\n")))?;
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

    #[test]
    fn skill_enabled_flag_is_read() {
        let mut cfg = Config::default();
        cfg.set("skill.image.enabled", "false").unwrap();
        cfg.set("skill.browse.enabled", "true").unwrap();
        assert!(cfg.skill_disabled("image"));
        assert!(!cfg.skill_disabled("browse"));
        assert!(!cfg.skill_disabled("absent"));
    }

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
    fn revoking_an_allow_keeps_the_rest_of_the_config() {
        let dir = std::env::temp_dir().join(format!("imp-allow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(
            &path,
            "allow = npm test\nmodel = deepseek-flash\nallow = cargo test\n# allow = keep me\n",
        )
        .unwrap();
        let cfg = Config::load(&path).unwrap();
        assert!(cfg.allowed("npm test"));
        cfg.revoke_always("npm test").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "model = deepseek-flash\nallow = cargo test\n# allow = keep me\n"
        );
        assert!(!cfg.allowed("npm test"), "the revoked allow is still live");
        assert!(cfg.allowed("cargo test"), "another allow was dropped");
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
            models: Vec::new(),
            model: "m".into(),
            key_env: "K".into(),
            dialect: "openai".into(),
            api_key: String::new(),
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
        let dir = std::env::temp_dir().join(format!("imp-allow-save-{}", std::process::id()));
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

    #[test]
    fn unknown_keys_are_named_not_fatal() {
        let cfg = Config::from_str("allow_sudo = true\nsudoo = yes\nprovider.work.base = http://box/v1\n").unwrap();
        assert!(cfg.allow_sudo);
        assert_eq!(cfg.unknown, vec!["sudoo"], "provider keys are known, typos are not");
        assert_eq!(cfg.pairs.len(), 3);
        assert_eq!(unknown_note(&cfg.unknown).as_deref(), Some("config: unknown key sudoo ignored"));
        assert_eq!(unknown_note(&[]), None);
    }

    #[test]
    fn reload_applies_only_what_the_file_changed() {
        let mut live = Config::from_str("model = a\nmax_steps = 5\nallow = ls\n").unwrap();
        live.model = "picked".into(); // chosen with /model during the session
        let new = Config::from_str("model = a\nmax_steps = 9\nallow_sudo = true\nallow = ls\nallow = pwd\n").unwrap();
        let (changed, removed) = live.reload_from(&new);
        assert_eq!(changed, vec!["allow_sudo: unset → true", "max_steps: 5 → 9"]);
        assert!(removed.is_empty());
        assert!(live.allow_sudo && live.max_steps == 9);
        assert_eq!(live.model, "picked", "a session choice was undone");
        let gone = Config::from_str("model = a\n").unwrap();
        let (_, removed) = live.reload_from(&gone);
        assert_eq!(removed, vec!["allow_sudo", "max_steps"]);
    }

    #[test]
    fn effort_takes_three_levels_or_default() {
        assert_eq!(Config::from_str("effort = high\n").unwrap().effort.as_deref(), Some("high"));
        assert_eq!(Config::from_str("effort = default\n").unwrap().effort, None);
        assert!(Config::from_str("effort = max\n").is_err());
    }

    #[test]
    fn a_setting_replaces_its_line_and_keeps_the_rest() {
        let text = "# mine\nthinking=false\n# thinking=true\nmodel = a\n\n# >>> providers\nprovider.w.base = x\n# <<< providers\n";
        let out = set_line(text, "thinking", "on");
        assert_eq!(out, text.replace("thinking=false", "thinking = on"), "{out}");
        // A key not in the file is appended; a comment is not that key.
        let out = set_line("# effort=default\nmodel = a", "effort", "high");
        assert_eq!(out, "# effort=default\nmodel = a\neffort = high\n");
        // Duplicates: the last line is the one that counts, so it is the one changed.
        let out = set_line("mouse=on\nmouse=off\n", "mouse", "on");
        assert_eq!(out, "mouse=on\nmouse = on\n");
        assert_eq!(set_line("", "mouse", "off"), "mouse = off\n");
    }

    #[test]
    fn context_window_is_a_positive_number_or_unset() {
        assert_eq!(Config::from_str("").unwrap().context_window, None);
        assert_eq!(Config::from_str("context_window = 32000").unwrap().context_window, Some(32_000));
        assert!(Config::from_str("context_window = 0").is_err());
        assert!(Config::from_str("context_window = big").is_err());
    }

    #[test]
    fn a_setting_is_saved_to_the_file() {
        let dir = std::env::temp_dir().join(format!("imp-setting-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "# keep\nthinking=false\n").unwrap();
        save_setting(&path, "thinking", "on").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# keep\nthinking = on\n");
        assert!(Config::load(&path).unwrap().thinking);
    }

    #[test]
    fn a_key_saved_with_a_named_provider_is_used_and_counts_as_configured() {
        let mut cfg = Config::default();
        cfg.named = vec![NamedProvider {
            name: "work".into(),
            base: "http://box/v1".into(),
            model: "m".into(),
            dialect: "openai".into(),
            api_key: String::from("test-key-value"),
            ..NamedProvider::default()
        }];
        cfg.set_active("work").unwrap();
        assert!(cfg.resolve_key_env(env(&[])).is_ok(), "a saved key was not accepted");
        assert_eq!(cfg.api_key, "test-key-value", "the saved key was not used");
        assert_eq!(Config::default().configured_fallback(env(&[])), None);
        let mut other = Config::default();
        other.named = cfg.named.clone();
        assert_eq!(other.configured_fallback(env(&[])), Some("work".to_string()));
    }

    #[test]
    fn a_saved_config_is_private_to_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("imp-private-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "# keep\n").unwrap();
        save_setting(&path, "thinking", "on").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the config holds keys and must be owner-only");
    }

    #[test]
    fn a_skill_switch_is_saved_and_loaded_back() {
        let dir = std::env::temp_dir().join(format!("imp-skill-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "# keep\n").unwrap();
        save_setting(&path, "skill.image.enabled", "false").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# keep\n"), "comments kept: {text:?}");
        let loaded = Config::load(&path).unwrap();
        assert!(loaded.skill_disabled("image"));
        assert!(loaded.unknown.is_empty(), "no unknown-key warning: {:?}", loaded.unknown);
    }

    #[test]
    fn editor_resolves_config_then_environment() {
        let mut cfg = Config::default();
        assert_eq!(cfg.editor_argv(env(&[("EDITOR", "nano")]), false), Some(vec!["nano".to_string()]));
        cfg.set("editor", "vim -u NONE").unwrap();
        assert_eq!(
            cfg.editor_argv(env(&[("EDITOR", "nano")]), false),
            Some(vec!["vim".to_string(), "-u".to_string(), "NONE".to_string()])
        );
        assert_eq!(Config::default().editor_argv(env(&[]), false), None);
    }

    #[test]
    fn visual_resolves_config_then_environment_and_has_no_default() {
        let mut cfg = Config::default();
        assert_eq!(cfg.editor_argv(env(&[("VISUAL", "code --wait")]), true).unwrap()[0], "code");
        cfg.set("visual", "subl").unwrap();
        assert_eq!(cfg.editor_argv(env(&[("VISUAL", "code")]), true), Some(vec!["subl".to_string()]));
        assert_eq!(Config::default().editor_argv(env(&[("EDITOR", "vim")]), true), None);
    }

    #[test]
    fn a_provider_switch_is_saved_and_loaded_back() {
        let dir = std::env::temp_dir().join(format!("imp-provider-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        std::fs::write(&path, "# keep\n").unwrap();
        save_setting(&path, "provider.openai.enabled", "false").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# keep\n"), "comments kept: {text:?}");
        let loaded = Config::load(&path).unwrap();
        assert!(loaded.provider_disabled("openai"));
        assert!(loaded.unknown.is_empty(), "no unknown-key warning: {:?}", loaded.unknown);
    }

    #[test]
    fn a_built_in_provider_is_configured_by_its_key_or_needs_none() {
        let cfg = Config::default();
        assert!(!cfg.provider_configured(Provider::OpenAi, env(&[])));
        assert!(cfg.provider_configured(Provider::OpenAi, env(&[("OPENAI_API_KEY", "k")])));
        assert!(cfg.provider_configured(Provider::Ollama, env(&[])));
        assert!(cfg.provider_configured(Provider::Custom, env(&[])));
    }

    #[test]
    fn a_named_provider_without_a_key_variable_needs_no_key() {
        let mut cfg = Config::default();
        cfg.named = vec![NamedProvider {
            name: "local".into(),
            base: "http://localhost:4141/v1".into(),
            model: "m".into(),
            dialect: "openai".into(),
            ..NamedProvider::default()
        }];
        cfg.set_active("local").unwrap();
        assert!(cfg.resolve_key_env(env(&[])).is_ok(), "asked for a key it does not use");
    }

    #[test]
    fn startup_falls_back_to_a_configured_provider() {
        let cfg = Config::default(); // DeepSeek is active and has no key here
        assert_eq!(cfg.configured_fallback(env(&[("OPENAI_API_KEY", "k")])), Some("openai".to_string()));
        assert_eq!(cfg.configured_fallback(env(&[])), None);
    }

    #[test]
    fn provider_enabled_flag_is_read() {
        let mut cfg = Config::default();
        cfg.set("provider.openai.enabled", "false").unwrap();
        cfg.set("provider.ollama.enabled", "true").unwrap();
        assert!(cfg.provider_disabled("openai"));
        assert!(!cfg.provider_disabled("ollama"));
        assert!(!cfg.provider_disabled("absent"));
    }

    #[test]
    fn a_target_with_spaces_is_one_argument() {
        let argv = editor_launch(vec!["vim".to_string()], "my notes.md");
        assert_eq!(argv, vec!["vim".to_string(), "my notes.md".to_string()]);
    }
}

#[cfg(test)]
mod auto_mode_tests {
    use super::*;

    #[test]
    fn auto_is_a_permissions_value_and_round_trips() {
        assert_eq!(Permissions::parse("auto"), Ok(Permissions::Auto));
        assert_eq!(Permissions::Auto.as_str(), "auto");
        assert_eq!(Permissions::from_code(Permissions::Auto.code()), Permissions::Auto);
        assert_eq!(Config::from_str("permissions = auto").unwrap().permissions, Permissions::Auto);
    }

    #[test]
    fn an_unknown_permissions_value_names_all_four() {
        let err = Permissions::parse("sometimes").unwrap_err();
        assert!(err.contains("normal, strict, auto, or yolo"), "{err}");
    }
}

#[cfg(test)]
mod title_summary_setting_tests {
    use super::*;

    #[test]
    fn title_summary_is_auto_on_or_off() {
        assert_eq!(Config::from_str("").unwrap().title_summary, TitleSummary::Auto);
        assert_eq!(Config::from_str("title_summary = on").unwrap().title_summary, TitleSummary::On);
        assert_eq!(Config::from_str("title_summary = off").unwrap().title_summary, TitleSummary::Off);
        assert!(Config::from_str("title_summary = maybe").is_err());
    }
}

/// Whether a long first message gets a summary as the session title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleSummary {
    /// On for hosted providers, off for Ollama and custom endpoints.
    Auto,
    On,
    Off,
}

impl TitleSummary {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            _ => Err(format!("title_summary must be auto, on, or off (got {s:?})")),
        }
    }
}

/// The text each enumerable setting uses in the config file and the settings form.
impl Links {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

impl ColorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

impl TitleSummary {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::On => "on",
            Self::Off => "off",
        }
    }
}

#[cfg(test)]
mod provider_models_tests {
    use super::*;

    #[test]
    fn a_models_line_parses_as_a_comma_separated_list() {
        let text = "# >>> providers\nprovider.work.base = http://box/v1\nprovider.work.model = m1\nprovider.work.models = m1, m2 ,m3\n# <<< providers\n";
        let cfg = Config::from_str(text).unwrap();
        let work = cfg.named.iter().find(|n| n.name == "work").unwrap();
        assert_eq!(work.models, vec!["m1", "m2", "m3"]);
    }

    #[test]
    fn the_default_model_is_always_offered() {
        let work = NamedProvider {
            name: "work".into(),
            model: "m1".into(),
            models: vec!["m2".into()],
            ..NamedProvider::default()
        };
        assert_eq!(work.offered_models(), vec!["m1".to_string(), "m2".to_string()]);
        let bare = NamedProvider { name: "bare".into(), model: "m1".into(), ..NamedProvider::default() };
        assert_eq!(bare.offered_models(), vec!["m1".to_string()]);
    }

    #[test]
    fn models_are_written_back_and_the_line_is_omitted_when_empty() {
        let with = NamedProvider {
            name: "work".into(),
            base: "http://box/v1".into(),
            model: "m1".into(),
            models: vec!["m2".into(), "m3".into()],
            dialect: "openai".into(),
            ..NamedProvider::default()
        };
        let text = providers_block(&[with.clone()]);
        assert!(text.contains("provider.work.models = m2, m3\n"), "{text}");
        let back = Config::from_str(&text).unwrap();
        assert_eq!(back.named[0].models, vec!["m2", "m3"]);
        let without = NamedProvider { models: Vec::new(), ..with };
        assert!(!providers_block(&[without]).contains(".models"), "an empty list was written");
    }

    fn saved(name: &str) -> NamedProvider {
        NamedProvider { name: name.into(), ..Default::default() }
    }

    #[test]
    fn a_default_name_is_the_dialect_then_the_lowest_free_number() {
        // Every dialect is a built-in name, so the first default is `<dialect>-2`.
        assert_eq!(default_provider_name("gemini", &[]), "gemini-2");
        assert_eq!(default_provider_name("gemini", &[saved("gemini-2")]), "gemini-3");
        assert_eq!(default_provider_name("zebra", &[]), "zebra");
    }

    #[test]
    fn check_provider_name_refuses_built_ins_duplicates_and_bad_characters() {
        let named = [saved("work")];
        assert!(check_provider_name("openai", None, &named).is_err());
        assert!(check_provider_name("OpenAI", None, &named).is_err());
        assert!(check_provider_name("work", None, &named).is_err());
        assert!(check_provider_name("bad name", None, &named).is_err());
        assert!(check_provider_name("office", None, &named).is_ok());
    }

    #[test]
    fn a_keyless_built_in_is_listed_only_once_switched_on() {
        let mut cfg = Config::default();
        let no_env = |_: &str| None::<String>;
        assert!(!cfg.provider_listed(Provider::Custom, no_env), "custom listed by default");
        assert!(!cfg.provider_listed(Provider::Ollama, no_env), "ollama listed by default");
        cfg.set("provider.custom.enabled", "true").unwrap();
        assert!(cfg.provider_listed(Provider::Custom, no_env));
        cfg.set("provider.custom.enabled", "false").unwrap();
        assert!(!cfg.provider_listed(Provider::Custom, no_env));
    }

    #[test]
    fn a_bind_line_reads_as_a_chord_and_a_command() {
        let cfg = Config::from_str("bind.ctrl-x-n = /model\n").unwrap();
        assert_eq!(cfg.bindings, vec![("ctrl-x-n".to_string(), "/model".to_string())]);
    }

    #[test]
    fn a_bind_line_on_a_reserved_key_is_ignored_and_the_others_load() {
        let cfg = Config::from_str("bind.ctrl-c = /model\nbind.ctrl-x-n = /skills\nmodel = deepseek-pro\n").unwrap();
        assert_eq!(cfg.bindings.len(), 1, "{:?}", cfg.bindings);
        assert_eq!(cfg.bindings[0].0, "ctrl-x-n");
        assert_eq!(cfg.model, "deepseek-pro", "a bad bind line stopped the file loading");
        assert_eq!(
            unknown_note(&cfg.unknown).as_deref(),
            Some("config: unknown key bind.ctrl-c ignored"),
            "the ignored bind line was not named"
        );
    }

    #[test]
    fn a_chord_has_one_or_two_keys_and_no_reserved_key() {
        assert_eq!(parse_chord("ctrl-x-n"), Ok(vec!["ctrl+x".to_string(), "n".to_string()]));
        assert_eq!(parse_chord("ctrl-p"), Ok(vec!["ctrl+p".to_string()]));
        assert!(parse_chord("ctrl-c").is_err(), "Ctrl+C is reserved");
        assert!(parse_chord("esc").is_err(), "Esc is reserved");
        assert!(parse_chord("enter").is_err(), "Enter is reserved");
        assert!(parse_chord("a-b-c").is_err(), "three keys");
    }

    #[test]
    fn a_config_line_switches_a_keyless_built_in_on() {
        let cfg = Config::from_str("provider.ollama.enabled = true\n").unwrap();
        assert!(cfg.provider_listed(Provider::Ollama, |_: &str| None::<String>));
        assert!(!cfg.provider_listed(Provider::Custom, |_: &str| None::<String>));
    }

    #[test]
    fn a_keyed_built_in_is_listed_once_its_key_is_set() {
        let cfg = Config::default();
        assert!(!cfg.provider_listed(Provider::OpenAi, |_: &str| None::<String>));
        assert!(cfg.provider_listed(Provider::OpenAi, |_: &str| Some("k".to_string())));
    }

    #[test]
    fn a_profile_keeps_its_own_name_and_may_rename_to_a_free_one() {
        let named = [saved("work")];
        assert!(check_provider_name("work", Some("work"), &named).is_ok());
        assert!(check_provider_name("office", Some("work"), &named).is_ok());
        assert!(check_provider_name("work", Some("office"), &named).is_err());
    }
}
