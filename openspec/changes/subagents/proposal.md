# Proposal: subagents

## Why

imp runs every task in one conversation. A large search or an independent
review fills the parent's context with work that only its final answer needs.
Claude Code and OpenCode solve this with subagents: the model delegates a task
to a named profile that works in its own context and returns a summary. This
change adds that capability, reversing the decision in the archived mote-v1
design ("Agents are profiles, not delegates"), which rejected it for cost. The
user has asked for it.

## What Changes

- A `task` tool. The model passes an agent profile name and a prompt. imp runs
  that profile in a fresh context and returns the child's final text as the
  tool result.
- A child uses the profile's instructions, `tools`, `model`, `thinking`, and
  `effort`, as `/agent` would apply them. Its tools are exactly those the
  profile allows. Write-capable profiles are allowed.
- Every confirmation and question a child raises goes to the root interface,
  tagged with the child's name. The session permission policy applies unchanged.
- Child depth is 1. A child has no `task` tool.
- A child's streamed text and reasoning are not shown in the parent transcript.
  Its tool activity is shown with the child's name in the row.
- A child's messages, tool calls, and results are written to the parent's
  session file as nested records tagged with the parent's `task` call id. No
  separate session file is created.
- The user can drill into a child from its `task` row. The child transcript
  opens read-only, shows every message and tool call with its result, and returns
  to the parent with one key. The same view works after the session is resumed.
- Nested records are never sent to the model. Only the parent's `task` call and
  result form the parent's wire history.
- Esc cancels the parent turn and any running child.

## Out of Scope

- Parallel subagents. Calls run one after another in v1.
- Subagents launched by a child (depth above 1).
- A per-call model override. A child uses its profile's model or the session's.
- Persisting child transcripts for later resume.
- Changing the built-in agents `build`, `plan`, and `review`. They can be used
  as children if their profile allows the tools.

## Impact

- `src/agent.rs`: schema entry, `run_tool` arm, child run, tool-list filter,
  logging guard.
- `src/store.rs`: profile lookup by name from the roster.
- `src/tui.rs`: no change needed if child tool rows reuse the existing tool
  event; verified in tasks.
- `docs/agent-loop.md`, `README.md`: document the `task` tool.
- Binary size rises slightly. The design states the expected bytes.
