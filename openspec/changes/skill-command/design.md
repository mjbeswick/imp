# Design: /skill command

## Context

- `skill_load` (`src/agent.rs`, around line 1642) returns a skill's body, capped at 4096 characters. Skills
  come from `Session::skills` (`src/agent.rs`, line 382), which is loaded from `store::load_skills`.
- The worker thread (`src/tui.rs`, around line 8492) matches `Cmd::Prompt` and `Cmd::Retry`, builds the turn
  text, and calls `worker_session.turn(&text, ...)`. Attachments already work this way: the TUI sends a `Cmd`
  and the worker holds the data until the next prompt.
- `/skills` (`src/tui.rs`, around line 5551) opens a picker of skills that toggles them. Its picker kind,
  `PickKind::Skill`, is used for toggling, so `/skill` does not reuse it.
- Commands live in `COMMANDS` (`src/help.rs`, line 50). The command palette (Ctrl+P) reads that table, so a
  new entry appears there too.

## Decisions

- **A worker-side queue.** The worker thread keeps `pending_skills: Vec<String>` of skill bodies. A new
  `Cmd::LoadSkill(String)` adds a body to it. The TUI sends the command; the worker looks the name up in
  `worker_session.skills` and refuses an unknown or disabled one with an `Ev::Info` line.
- **Sent once, prepended.** On the next `Cmd::Prompt`, the worker prepends each queued body to the message
  text, then clears the queue. The user's text in the transcript is what they typed.
- **Validation in both places.** The TUI checks the name against enabled skills and shows the error before
  sending. The worker repeats the check, since skills can change on `/reload`.
- **Bare `/skill`** shows `usage: /skill <name>` and the enabled skill names as one toast line. It does not
  open a picker.

## Risks

- A queued skill that is never followed by a prompt (the user quits first) is dropped. It was never sent, so
  nothing is lost.
- A large skill adds its full body to the message. The 4 KB cap in `skill_load` is applied here too, so one
  skill cannot exceed it.

## Assumptions

- `turn` records the text it is given as the user message. The prepended skill therefore becomes part of the
  stored conversation, which is what the model needs to see again on resume.
