# Design

## Context

- `Goal { text, round, rounds }` (src/agent.rs, around line 90) and `Session.goal`
  (around line 331) hold the goal. `start_goal` (around line 1151) sets it and
  returns the first round's message. `goal_next` (around line 1163) reads the
  agent's last reply for `GOAL MET` or `GOAL BLOCKED`, and clears the goal when
  the round limit is reached or the user declines a confirmation.
- The TUI handles `/goal` in `run_command` (src/tui.rs, around line 5189): `goal`
  shows the status, `goal off` sends `Cmd::GoalOff`, and `goal <text>` sends
  `Cmd::Goal`. The worker loop around line 7952 runs the rounds.
- `Hooks::confirm(what)` is the existing confirmation path that tools use.
- Session metadata is written by `Log` (src/store.rs). `Log::meta` already reads
  and writes the session's title and counts, so a goal can sit beside them.

## Decision

- Add `paused: bool` to `Goal`. `goal_next` returns no round while paused. `/goal
  pause` and `/goal resume` set the flag. `/goal clear` drops the goal.
- The `goal` tool's `set` operation does not start rounds. It stores a pending goal
  and asks through `Hooks::confirm` with the goal text. On yes it becomes a running
  goal from round 1. On no it is dropped and the agent is told the user declined.
- The goal (text, round, rounds, paused) is written to the session metadata when it
  changes, and read back when a session is opened or resumed. A running goal
  restored on resume starts paused, so resuming never runs rounds unasked.
- The status line reads `goal n/m` or `goal paused`.

## Risks

- A goal saved with the session that restarts running could run rounds unasked. The
  design restores every goal as paused.
- The confirmation runs inside a turn. Declining it must not leave the turn
  waiting; the tool result says the user declined.
