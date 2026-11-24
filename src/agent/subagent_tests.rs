//! Tests for the `task` tool: profile lookup, the child run, depth, logging,
//! permission tagging, and the step cap.

use super::*;
use crate::transport::{text_reply, tool_reply, Fake, Script};
use std::sync::Arc;

fn tmp() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "imp-subagent-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn roster() -> Vec<Agent> {
    let reviewer = Agent::from_text(
        "reviewer".into(),
        "---\ntools: read\n---\nReview things.",
        None,
    );
    vec![Agent::builtin(), reviewer]
}

fn scripted(script: Vec<Script>, max_steps: usize) -> (Session, Arc<Fake>) {
    let fake = Arc::new(Fake::new(script));
    let d = tmp();
    let mut cfg = crate::config::Config::default();
    cfg.max_steps = max_steps;
    let mut s = Session::new(
        cfg,
        d.clone(),
        d.join("out"),
        String::new(),
        Agent::builtin(),
        roster(),
        Arc::new(std::sync::atomic::AtomicU16::new(0)),
    );
    s.transport = Box::new(fake.clone());
    s.retry_base = std::time::Duration::from_millis(1);
    (s, fake)
}

struct Rec {
    confirms: Vec<String>,
}

impl Hooks for Rec {
    fn ui(&mut self, _: Ui) {}
    fn confirm(&mut self, what: &str) -> bool {
        self.confirms.push(what.to_string());
        true
    }
    fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
        None
    }
}

fn task_args(agent: &str, prompt: &str) -> String {
    serde_json::json!({"agent": agent, "prompt": prompt}).to_string()
}

/// The content of the tool result for call `id` in the parent history.
fn tool_result(s: &Session, id: &str) -> String {
    s.history
        .iter()
        .find(|m| m.tool_call_id.as_deref() == Some(id))
        .map(|m| m.content.clone())
        .unwrap_or_default()
}

#[test]
fn task_is_offered_to_the_parent_and_hidden_from_children() {
    let (mut s, _) = scripted(vec![], 4);
    assert!(s.active_tools().iter().any(|t| t["function"]["name"] == "task"));
    s.child_call = Some("t1".into());
    assert!(!s.active_tools().iter().any(|t| t["function"]["name"] == "task"));
}

#[test]
fn unknown_or_missing_agent_is_refused_without_a_child() {
    let (mut s, fake) = scripted(
        vec![
            Script::Ok(tool_reply("t1", "task", &task_args("nope", "x"))),
            Script::Ok(tool_reply("t2", "task", "{\"prompt\":\"x\"}")),
            Script::Ok(text_reply("ok")),
        ],
        4,
    );
    s.turn("go", &mut Rec { confirms: vec![] }).unwrap();
    let unknown = tool_result(&s, "t1");
    assert!(unknown.contains("no such agent"), "{unknown}");
    assert!(unknown.contains("reviewer") && unknown.contains("build"), "{unknown}");
    let dump: Vec<String> = s.history.iter().map(|m| format!("{}|{:?}|{}", m.role.as_str(), m.tool_call_id, m.content)).collect();
    assert!(tool_result(&s, "t2").contains("needs an agent name"), "t2: {dump:#?}");
    assert_eq!(fake.attempts(), 3, "a child request was sent");
}

#[test]
fn child_returns_its_final_text_and_leaves_the_parent_unchanged() {
    let (mut s, fake) = scripted(
        vec![
            Script::Ok(tool_reply("t1", "task", &task_args("reviewer", "look at it"))),
            Script::Ok(text_reply("child says hi")),
            Script::Ok(text_reply("parent done")),
        ],
        4,
    );
    s.turn("go", &mut Rec { confirms: vec![] }).unwrap();
    assert_eq!(tool_result(&s, "t1"), "child says hi");
    assert_eq!(s.agent.name, "build");
    assert_eq!(s.history.last().unwrap().content, "parent done");
    // The child's request carries its prompt and not the task tool.
    let child_body = fake.sent.lock().unwrap()[1].clone();
    assert!(child_body.contains("look at it"));
    assert!(!child_body.contains("\"name\":\"task\""));
}

#[test]
fn a_failed_child_restores_the_parent_state() {
    let (mut s, _) = scripted(
        vec![
            Script::Ok(tool_reply("t1", "task", &task_args("reviewer", "x"))),
            Script::Transport("down".into()),
            Script::Transport("down".into()),
            Script::Transport("down".into()),
            Script::Transport("down".into()),
            Script::Transport("down".into()),
        ],
        4,
    );
    let before = s.cfg.model.clone();
    let _ = s.turn("go", &mut Rec { confirms: vec![] });
    assert_eq!(s.agent.name, "build");
    assert_eq!(s.cfg.model, before);
    assert!(s.child_call.is_none());
    assert!(s.history.iter().all(|m| m.role != crate::api::Role::Assistant || m.content != "child"));
}

#[test]
fn child_messages_are_logged_nested_and_not_as_parent_messages() {
    let (mut s, _) = scripted(
        vec![
            Script::Ok(tool_reply("t1", "task", &task_args("reviewer", "x"))),
            Script::Ok(text_reply("child text")),
            Script::Ok(text_reply("done")),
        ],
        4,
    );
    let dir = tmp();
    let log = crate::store::Log::create(&dir, &s.tools.root, "deepseek-flash");
    let path = log.path.clone();
    s.attach_log(log);
    s.turn("go", &mut Rec { confirms: vec![] }).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let nested: Vec<&str> = text.lines().filter(|l| l.contains("\"t\":\"child\"")).collect();
    assert!(!nested.is_empty(), "no nested records:\n{text}");
    assert!(nested.iter().all(|l| l.contains("\"call\":\"t1\"")));
    assert!(text.contains("child text"));
    // The child's own reply is nested. Only the parent's tool result, which
    // legitimately carries the child's final answer, appears as a `msg`.
    assert!(text
        .lines()
        .filter(|l| l.contains("\"t\":\"msg\"") && l.contains("child text"))
        .all(|l| l.contains("\"role\":\"tool\"")));
}

#[test]
fn a_child_stops_at_the_step_cap() {
    let looping = |id: &str| Script::Ok(tool_reply(id, "read", "{\"paths\":[\"Cargo.toml\"]}"));
    let (mut s, _) = scripted(
        vec![
            Script::Ok(tool_reply("t1", "task", &task_args("reviewer", "x"))),
            looping("r1"),
            looping("r2"),
            looping("r3"),
            Script::Ok(text_reply("parent done")),
        ],
        2,
    );
    s.turn("go", &mut Rec { confirms: vec![] }).unwrap();
    let out = tool_result(&s, "t1");
    assert!(out.contains("step cap"), "{out}");
}

#[test]
fn child_confirmations_reach_the_parent_with_the_child_name() {
    let mut rec = Rec { confirms: vec![] };
    {
        let mut hooks = ChildHooks { parent: &mut rec, name: "reviewer".into() };
        assert!(hooks.confirm("write notes.md"));
    }
    assert_eq!(rec.confirms, vec!["[reviewer] write notes.md".to_string()]);
}

#[test]
fn child_streamed_text_is_not_forwarded_to_the_parent() {
    struct Count(usize, usize);
    impl Hooks for Count {
        fn ui(&mut self, e: Ui) {
            match e {
                Ui::Content(_) | Ui::Reasoning(_) => self.0 += 1,
                Ui::ToolStart { .. } => self.1 += 1,
                _ => {}
            }
        }
        fn confirm(&mut self, _: &str) -> bool {
            true
        }
        fn ask(&mut self, _: &str, _: &[String]) -> Option<String> {
            None
        }
    }
    let mut c = Count(0, 0);
    {
        let mut hooks = ChildHooks { parent: &mut c, name: "reviewer".into() };
        hooks.ui(Ui::Content("hi".into()));
        hooks.ui(Ui::Reasoning("hmm".into()));
        hooks.ui(Ui::ToolStart { name: "read".into(), summary: "read a".into(), args: "{}".into() });
    }
    assert_eq!(c.0, 0, "child prose reached the parent");
    assert_eq!(c.1, 1, "child tool row was dropped");
}
