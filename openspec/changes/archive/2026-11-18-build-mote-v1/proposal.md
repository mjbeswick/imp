# Proposal

## Why

The current shell agent is a stack: `aichat` plus `llm-functions`, two binaries,
a bash tool layer, YAML agent configs, and per-agent state under
`~/Library/Application Support`. It works, but every part Michael wants to
change is the hardest part to reach, and the stack is many times larger than
the job.

`mote` is the same job in one Rust binary: a terminal agent for DeepSeek with a
first-class TUI, compaction, persistent memory, skills, and subagents. Small
enough to read end to end, fast enough to open like a shell.

## What Changes

- `mote init` creates the config and state layout with one real agent and one
  real skill as examples, overwriting nothing.
- New Rust binary `mote`, about 1 MB, no runtime, one crate dependency
  (`serde_json`), TLS and HTTP from the system libcurl.
- DeepSeek Chat Completions over HTTPS with streaming, thinking mode, and
  prefix-cache-aware prompt construction.
- A simple chat interface: scrolling transcript, one input line, one dim status
  line, collapsible thinking, and one-line tool activity.
- Four built-in tools: `bash`, `read`, `write`, `edit`, with a CWD path jail and
  confirmation for destructive commands.
- Chat compaction driven by an absolute token budget, preserving the last four
  exchanges verbatim and invalidating the cache at most rarely.
- File-backed memory with global and project scope; titles only in the prompt.
- Skills as `SKILL.md` directories; description-only index, body loaded on
  demand. Reads the existing `llm-functions` skill files unchanged.
- Agents as markdown profiles, selectable at launch and switchable mid-conversation
  without losing the transcript.
- JSONL session log with wire-fidelity `--resume`.

**BREAKING**: replaces the `aichat`-based `shell-agent` workflow. `aichat` and
its agent config are removed from the machine; `~/Projects/shell-agent` and
`~/Projects/llm-functions` remain as references.

## Capabilities

### New Capabilities

- `agent-loop`: request a completion, stream it, run the requested tools, repeat
  until the model answers; retries, cancellation, thinking mode.
- `conversation-context`: what goes into the request each turn, cache-stable
  prefix ordering, reasoning round-trip, compaction.
- `tui`: layout, keys, streaming render, tool activity, status bar, terminal
  safety, confirmation prompts.
- `tools`: `bash`, `read`, `write`, `edit`, their limits, the path jail, and
  destructive-command confirmation.
- `memory`: persistent notes as files, global and project scope, index and
  search behavior.
- `skills`: discovery, index, on-demand loading, precedence.
- `agents`: named profiles for instructions, tools, and model, switchable mid-conversation.
- `session`: JSONL log, resume, titles, picker, delete.
- `instructions`: `AGENTS.md` discovery, bounds, frozen prefix, shared skill and agent roots.
- `setup`: `init` layout creation, examples, no overwrite, and the ban on agent writes to its own config.
- `permissions`: one setting, three modes, safe list, deny list, jail, approval flow.

### Modified Capabilities

None. This is a new project.

## Impact

- New code: `~/Projects/mote` (Rust crate, ~2,500 lines).
- New state: `~/.config/mote/`, `~/.local/state/mote/`, `./.mote/`.
- Dependencies: `serde_json`, plus libcurl and termios bound through `extern
  "C"`. No async runtime, no widget library.
- Removed: the aichat-based shell agent invocation path. `aichat` already
  uninstalled; its config archived to
  `~/Documents/aichat-config-backup-2026-11-17.tar.gz`.
- External: DeepSeek API, key from `DEEPSEEK_API_KEY`.

## Assumptions recorded

- `--resume` restores the real wire history, not just the visible transcript,
  and appends to the same log.
- A `context_length_exceeded` that arrives mid tool loop is handled by
  truncating older tool outputs and retrying once; full compaction happens only
  at turn boundaries.
