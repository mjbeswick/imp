//! Secrets in the environment: kept from the agent's commands, and redacted
//! from anything that enters the conversation.

use std::process::Command;
use std::sync::OnceLock;

/// Variables the user passes through with `pass_env`, set once at startup.
static KEEP: OnceLock<Vec<String>> = OnceLock::new();

pub fn keep(names: &[String]) {
    let _ = KEEP.set(names.to_vec());
}

/// Variables the config names in `auth` lines: secrets whatever their names.
static ALSO: OnceLock<Vec<String>> = OnceLock::new();

/// Register `auth` header templates, before any secret is looked up.
pub fn also(templates: &[String]) {
    let _ = ALSO.set(templates.iter().flat_map(|t| names_in(t)).collect());
}

/// The `$NAME` and `${NAME}` variables a template names.
pub fn names_in(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find('$') {
        rest = &rest[i + 1..];
        let braced = rest.starts_with('{');
        let body = if braced { &rest[1..] } else { rest };
        let name: String = body.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if !name.is_empty() {
            out.push(name.clone());
        }
        rest = &body[name.len()..];
    }
    out
}

/// A template with its variables filled from the environment, or the name of
/// the first one that is unset.
pub fn expand(template: &str) -> Result<String, String> {
    let mut out = template.to_string();
    for name in names_in(template) {
        let value = std::env::var(&name).ok().filter(|v| !v.is_empty()).ok_or_else(|| name.clone())?;
        out = out.replace(&format!("${{{name}}}"), &value).replace(&format!("${name}"), &value);
    }
    Ok(out)
}

fn kept(name: &str) -> bool {
    KEEP.get().is_some_and(|k| k.iter().any(|n| n == name))
}

/// A variable that looks like a credential by its name.
// ponytail: name patterns, not values; add a pattern when a tool names its secret oddly.
pub fn is_secret(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    ["_KEY", "_TOKEN", "_SECRET", "_CREDENTIALS"].iter().any(|s| n.ends_with(s))
        || n.contains("PASSWORD")
        || n.contains("PASSWD")
        || n.starts_with("AWS_")
        || n == "TOKEN"
}

#[cfg(test)]
pub const TEST_NAME: &str = "IMP_FAKE_TOKEN";
#[cfg(test)]
pub const TEST_VALUE: &str = "fake-secret-value-42";

/// Secret variables set in this process, with their values.
fn present() -> &'static [(String, String)] {
    static SET: OnceLock<Vec<(String, String)>> = OnceLock::new();
    SET.get_or_init(|| {
        #[allow(unused_mut)]
        let mut found: Vec<(String, String)> =
            std::env::vars()
                .filter(|(k, v)| (is_secret(k) || ALSO.get().is_some_and(|a| a.contains(k))) && !v.is_empty())
                .collect();
        // Tests get one known secret, whatever the environment holds.
        #[cfg(test)]
        found.push((TEST_NAME.into(), TEST_VALUE.into()));
        found
    })
}

/// Drop every secret the user did not pass through from a child's environment.
pub fn scrub(cmd: &mut Command) {
    scrub_with(cmd, present());
}

fn scrub_with(cmd: &mut Command, secrets: &[(String, String)]) {
    for (name, _) in secrets {
        if !kept(name) {
            cmd.env_remove(name);
        }
    }
}

/// The first stripped secret a command names, as `$NAME` or `${NAME}`.
pub fn mentioned(command: &str) -> Option<&'static str> {
    mentioned_in(command, present())
}

fn mentioned_in<'a>(command: &str, secrets: &'a [(String, String)]) -> Option<&'a str> {
    secrets.iter().map(|(n, _)| n.as_str()).find(|n| {
        !kept(n)
            && (command.contains(&format!("${{{n}}}"))
                || command.match_indices(&format!("${n}")).any(|(at, m)| {
                    let next = command[at + m.len()..].chars().next();
                    !next.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                }))
    })
}

/// Text with every secret value replaced by `[redacted NAME]`. Short values
/// are left alone: they would match ordinary words.
pub fn redact(text: &str) -> String {
    redact_with(text, present())
}

fn redact_with(text: &str, secrets: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (name, value) in secrets {
        if value.len() >= 8 && out.contains(value.as_str()) {
            out = out.replace(value.as_str(), &format!("[redacted {name}]"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_name_and_fill_variables() {
        assert_eq!(names_in("Authorization: Bearer $GH_TOKEN"), ["GH_TOKEN"]);
        assert_eq!(names_in("X-Key: ${A_1}-$B"), ["A_1", "B"]);
        std::env::set_var("IMP_EXPAND_PROBE", "v1");
        assert_eq!(expand("K: ${IMP_EXPAND_PROBE}/$IMP_EXPAND_PROBE").unwrap(), "K: v1/v1");
        assert_eq!(expand("K: $IMP_UNSET_PROBE_X").unwrap_err(), "IMP_UNSET_PROBE_X");
    }

    #[test]
    fn names_that_look_like_credentials() {
        for n in ["DEEPSEEK_API_KEY", "GITHUB_TOKEN", "AWS_REGION", "DB_PASSWORD", "client_secret"] {
            assert!(is_secret(n), "{n}");
        }
        for n in ["PATH", "HOME", "KEYBOARD", "TOKENIZERS_PARALLELISM", "LANG"] {
            assert!(!is_secret(n), "{n}");
        }
    }

    #[test]
    fn values_are_redacted_by_name() {
        let s = vec![("GITHUB_TOKEN".to_string(), "ghp_abcdef123456".to_string()), ("PIN".into(), "1234".into())];
        let out = redact_with("export GITHUB_TOKEN=ghp_abcdef123456 pin 1234", &s);
        assert_eq!(out, "export GITHUB_TOKEN=[redacted GITHUB_TOKEN] pin 1234");
    }

    #[test]
    fn the_agents_commands_do_not_see_secrets() {
        let s = vec![("HOME".to_string(), "unused".to_string())];
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("printf '%s:%s' \"$HOME\" \"$PATH\"");
        scrub_with(&mut cmd, &s);
        let out = String::from_utf8(cmd.output().unwrap().stdout).unwrap();
        assert!(out.starts_with(':') && out.len() > 1, "{out:?}");
    }

    #[test]
    fn a_command_naming_a_secret_is_noticed() {
        let s = vec![("GITHUB_TOKEN".to_string(), "x".repeat(10))];
        assert_eq!(mentioned_in("echo $GITHUB_TOKEN", &s), Some("GITHUB_TOKEN"));
        assert_eq!(mentioned_in("curl -H \"x: ${GITHUB_TOKEN}\" u", &s), Some("GITHUB_TOKEN"));
        assert_eq!(mentioned_in("echo $GITHUB_TOKENS", &s), None);
        assert_eq!(mentioned_in("echo GITHUB_TOKEN", &s), None);
    }
}
