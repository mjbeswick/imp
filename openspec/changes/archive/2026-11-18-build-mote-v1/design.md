# Design

## Context

See `proposal.md` — Why. Three things shape every decision below.

**The API.** Verified against `api-docs.deepseek.com` on 2026-11-17:

- Models `deepseek-flash` (V4.1-Flash-1017) and `deepseek-v4-pro`, 1M context,
  384K output, tool calls supported. `deepseek-chat` is a retired name.
- Thinking is on by default. With `tools` present, `reasoning_content` from
  every prior assistant turn must be echoed back or the request is rejected
  with 400.
- Prefix caching is automatic: an exact prefix match bills at roughly 1/50th
  of a miss. `usage` reports hit and miss token counts.
- Chat Completions accepts `system` messages mid-conversation but cannot accept
  tool calls mid-conversation.
- Streaming tool calls are keyed by `index`; the first fragment carries the id
  and name, later ones carry arguments only, and some later fragments send an
  empty name.
- `finish_reason` includes `insufficient_system_resource` and `aborted`, both
  retryable.
- `deepseek_web_search` and `deepseek_read_url` are server tools and cannot be
  declared alongside function tools. The Search and Fetch endpoints are plain
  JSON and can.

**Measured on this machine**, from 316 assistant messages in the local agent
database: tool output 55.8% of turn bytes, reasoning 29.9%, tool arguments
11.9%, prose 2.4%. Cache reads were 96.6% of prompt tokens; the largest single
request was 39.8k tokens.

**The constraint.** One binary, ~1.2 MB, ~8 MB RSS, ≤ 3,000 lines.

## Goals / Non-Goals

**Goals**

- A prefix that is byte-identical between turns, so caching works.
- Tool results that cannot blow up the context.
- A tool loop that cannot hang: every wait has a timeout or a cancel path.
- Compaction that is rare, deep, and visible.
- Features that cost nothing in the prompt when unused.

**Non-Goals**

- Any provider other than DeepSeek.
- Async runtime. Blocking IO on threads, deliberately.
- Embeddings or vector search.
- Windows support.

## Decisions

### Rust, three crates, system TLS

The binary is the product, so the budget comes first:

| stack | binary |
|---|---|
| ratatui + crossterm + ureq/rustls + serde_json | ~3.5 MB |
| crossterm + ureq/rustls + serde_json | ~2.8 MB |
| **libcurl FFI + raw ANSI + serde_json** | **~1.0–1.2 MB** |
| everything hand-rolled, incl. JSON | ~0.7 MB |

Chosen: **libcurl FFI for HTTP, raw ANSI plus `tcgetattr`/`ioctl` for the
terminal, and `serde_json` as the only dependency.** TLS and HTTP/2 come from
the system libcurl, which is present on macOS and on any Linux with curl
installed; nothing is linked into the binary. JSON stays a crate because
hand-rolled parsing is the one cut that produces silent, hard-to-find bugs for
300 KB of savings.

Dropping ratatui costs less than it looks: the interface is a scrolling chat,
not a dashboard. It needs a clear, a wrap, a cursor, and a few colour escapes —
about 400 lines, and it renders faster than a widget tree.

Alternatives: ratatui for the nice default widgets (~900 KB, and a widget tree
to keep in sync with a simple layout); rustls for TLS (~1.5 MB, no system
dependency); a `curl` subprocess (smaller still, but a process per request and
no streaming handle).

### One binary, eight modules, one event loop

```
main.rs   argv, config, terminal guard, raw key parsing
app.rs    App state: history, scroll, streaming, usage, sessions
ui.rs     raw ANSI render of &App
http.rs   libcurl FFI: POST, streaming write callback, headers, timeouts
api.rs    request build, SSE parse, delta assembly, error mapping
agent.rs  turn loop, tool dispatch, compaction, subagents
store.rs  sessions, memory, skills, agents, instructions, jail
init.rs   the `init` layout, templates, and examples
tools.rs  bash, read, edit, write, search, ask
```

Main thread never blocks on IO. Each request runs on a worker emitting `Token`,
`Reasoning`, `ToolStart`, `ToolEnd`, `Compacted`, `Done`, `Failed` over `mpsc`.
Cancel is an `AtomicBool` checked between chunks and before each tool call.
Approval is the one exception to one-way events: the worker sends
`Confirm{id, prompt}` and blocks on a `oneshot`, because a tool cannot return
until the user answers.

### The interface is a coloured chat column

One column, no widgets, and colour that carries meaning. Six roles: prompt
marker and user text, assistant text, thinking, tool activity, error, accent.
Two palettes, dark and light, each with a truecolor and a 256-colour table.
`NO_COLOR` and `color=never` keep the layout and drop the escapes.

Roles rather than raw codes because a palette can be tested and a no-colour
mode can be proven honest. Every role also carries a non-colour marker, so
nothing depends on colour alone.

### Skills are the extension point for command-line tools

The system SHALL treat skills as the only way to add knowledge about a specific
command-line tool, and SHALL ship no per-CLI tools. A skill for `gh`, `jq`,
`rg`, `docker`, or `git` is a `SKILL.md` that names the command, the flags worth
using, the failure modes, and when to reach for it — the model then runs it
through `bash`.

This is the whole bet on smallness: a tool costs schema tokens on every request
and cannot be uninstalled at runtime; a skill costs one line until it is
loaded, and adding one is a file copy. Six tools cover the primitives — run,
read, edit, write, search, ask — and everything else is knowledge.

### Six slash commands, with a rule

A command must do something the model should not decide, or something a
sentence cannot express reliably. That keeps `/help`, `/new`, `/sessions`,
`/compact`, `/think`, `/quit` and rejects `/clear` (Ctrl+L), `/model` (flag and
status line), `/reload` (startup), `/mode` and `/cost` — the last two for a
specific reason: a one-keystroke permission change or cost query invites
flipping guardrails mid-session, which is the wrong moment for convenience.

### Two front-ends, one loop

Line mode and full-screen chat share the same session, tools, and history; they
differ only in input and rendering. Full-screen is chosen when both ends are
terminals, line mode otherwise, and `--cli` / `--chat` force either. Scripted
use, pipes, and CI never get escape sequences unless asked.

### A frozen prefix

Request is `system` (built-in prompt + instructions + skill index) → conversation.
Nothing volatile before the conversation: no timestamps, cwd, token counts, or
cache stats. Those live in the status line, which the model never sees.

Memory and skill changes mid-session arrive as a `system` message at the tail,
not as a prefix rewrite. Reloading instructions or skills is an explicit action
and reports that it reset the cache. This is the difference between a 96% hit
rate and a cache miss every few turns.

### Thinking off by default

Reasoning is 30% of turn bytes and, with tools present, cannot be pruned or
dropped. Sending it on every request is the single largest avoidable cost.
Default is disabled; `/think` enables one turn, `/think max` raises effort.

Alternatives: on at low effort (30% more context for marginal planning gains);
on always (unaffordable once compaction is considered).

### Six tools, batched

`bash`, `read`, `edit`, `write`, `search`, `ask` — 505 tokens of schema. No
glob, grep, list, patch, or todo; those are one bash line each.

Speed comes from round trips, not tool count: `read` takes `paths[]` and `edit`
takes `edits[]` applied atomically, so a typical fix is read → edit → bash
rather than six calls.

`ask` exists because the alternative is worse: without it, an agent facing a
real choice either guesses or spends turns asking in prose that the user must
answer in prose. It reuses the confirmation channel with a richer payload —
options plus a free-text row — and is prompt-instructed to fire only when
blocked.

Alternatives: five tools without `ask` (cheaper, but the agent guesses);
seven with glob/grep (123 more tokens, no new capability); bash-only (worst
edit reliability).

### Background jobs behind one extra tool

A 30-second foreground timeout is right for commands and wrong for builds. Rather
than raise it, `bash` takes `background` and a `job` tool lists, waits, and
kills. Jobs are process-group leaders writing to a spill file, so cancel and
kill reuse the same signal path as foreground commands, and a finished job
arrives as a tail `system` note instead of a prefix rewrite.

`wait` is capped at 55 seconds so it never turns into a silent hang; the model
polls again if the job is still going. Jobs do not survive a restart, and the
interface says so rather than pretending.

Alternatives: raise the timeout (freezes the chat, still finite); `nohup` in a
shell (no id, no output, no completion signal); a jobs pane only (the model
cannot start one).

### Tool output spills to disk

Inline cap 4 KB. Larger output goes to the session output directory, and the
result carries head, tail, path, and original size. With tool output at 56% of
turn bytes, this is the highest-leverage decision in the design: it bounds
context growth without spending a model call or losing data.

### Search is client-side

One `search` tool, two operations: `search` returns five results with
summaries and URLs, `fetch` returns one page as markdown. Same API key, same
host, $0.00075 per successful call, billed only on success.

Alternatives: server-side web tools (cannot coexist with function tools); a
second vendor (another key, another failure mode); `curl` scraping (unreliable
on HTML, no ranking).

### Memory is plain markdown files

`~/.config/mote/memory/*.md` and `./.mote/memory/*.md`. One note, one file, no
frontmatter: the filename is a slug of the title, the first heading is the
title, the rest is prose. Any editor writes them, `rg` searches them, and the
system re-reads them on every search so an external edit is visible with no
reload.

A 40-title index would cost ~480 tokens on every request for a feature used in
a minority of turns, and would break the prefix whenever a note is saved. The
two-operation tool costs ~80 tokens and nothing when unused.

Writes require approval, enforced by path so `write` and `bash` cannot bypass
it. The summarizer proposes up to three durable facts at compaction; nothing is
written without a yes.

Alternatives: frontmatter with tags and timestamps (a parser, a schema, and a
failure mode where a hand-edited file breaks); SQLite (a dependency and a
migration path for a directory of notes).

### Instructions from AGENTS.md

Global file, repository root, session directory, in that order, 8 KB cap,
`CLAUDE.md` only as a fallback when `AGENTS.md` is absent in the same
directory. Loaded once into the frozen prefix and reported at startup.

This is the cheapest way to make the agent project-aware: no new format, no
database, and the file is already there in most repositories.

### Skills as directories, one line each

`skills/<name>/SKILL.md` under the global config, `.mote/`, and any extra
directory passed at launch; project shadows global. Only `name — description`
is in the prompt; `skill_load` returns the body.

Frontmatter matches the existing `llm-functions` skills, so that collection is
reusable unchanged.

### Init writes two files and never overwrites

`mote init` creates directories because the layout is the interface, and writes
exactly two examples because formats are learned from examples, not docs. It
never overwrites, needs no key, makes no request, and finishes in milliseconds.
The alternative — auto-creating on first run — hides the layout and teaches
nothing.

The config template deliberately omits `api_key`: a key file is a key that gets
copied, and the environment already works.

### Agents are profiles, not delegates

`agents/<name>.md` holds instructions in the body and `tools`, `model`,
`thinking` in frontmatter. `--agent` picks one at launch; `/agent` switches
during a conversation, keeping the transcript.

No delegation, no threads, no `task` tool. A delegated child costs an extra
inference, can only return text because the API forbids splicing tool calls
into history, and needs confirm proxying, cancellation propagation, depth
limits and nested logging. For a shell agent on a 1M-context model with
spill-to-disk, that machinery buys less than it costs.

Switching appends a `system` note that supersedes the previous role rather than
rewriting the system message. Rewriting would be cleaner prose and would throw
away the cached prefix; that trade is wrong for a switch you might make twice.

### Sessions are a file plus a sidecar

One JSONL transcript, one small sidecar with title, cwd, model, counts. Listing
reads sidecars only; the transcript is parsed only when resuming. Titles come
from the first user message and live in the sidecar, so renaming never rewrites
the log. Deleting moves both files to a trash directory.

Resume restores the wire history exactly — reasoning, tool calls, matching
results, summary — because anything less produces either a 400 or a model that
has forgotten its own work.

### Prune, then compact

Prune at 45k estimated tokens: older tool results collapse to a stub plus spill
path, structure intact, no model call. Compact at 60k: keep the system prompt
and the last four exchanges verbatim, replace everything older with one
`system` message named `compacted`.

Why those numbers: the largest observed real request on this machine was 39.8k,
so 45k is "one big output away" and 60k is the point where the model's attention
on the task is clearly degrading. Both are configurable.

Honest cost note: at flash prices a 40k cache bust costs about a cent. The
reason to compact rarely is latency and quality, not money.

### Typed commands are the user's approval

The input line doubles as a shell: anything starting with `!` runs locally, skips
the confirmation step because the user typed it, and still hits the deny list.
Same runner, same caps, same spill file as the `bash` tool, so there is one
implementation and one set of limits.

The record goes back into the conversation as a `system` message at the tail —
compact, capped, with the exit code — so "what did git say" works without
re-running anything, and the prefix stays frozen.

### Guardrails, not a sandbox

One setting: `permissions = normal | strict | yolo`. One safe list (listing,
inspection, search, read-only git). One short deny list. Path jail on except in
`yolo`, which says so loudly at startup.

`sudo` is not on the deny list, because `sudo` is a capability, not a
catastrophe. It is gated separately by `allow_sudo=false`, always prompts when
enabled, and any genuinely destructive command stays denied whether or not it
is prefixed with `sudo`.

A password prompt needs the real terminal, so sudo gets a handoff: leave the
alternate screen, restore cooked mode, run attached to the tty, redraw after.
The password is never read, stored, or forwarded by mote; `sudo -n` is used
first so a cached credential skips the handoff entirely.

The honest framing matters: pattern matching on shell strings slows down
mistakes, it does not contain a determined command. The real boundary is the
jail for file tools and a human for shell. It is one enum, two lists, and one
boolean, about eighty lines, and it replaces three ad-hoc rules — the
destructive-command list, the memory-path approval rule, and the separate
`--yolo` flag.

Alternatives: a full allow/ask/deny rule DSL (more capable, harder to explain);
per-tool permissions (a matrix nobody reads); refusing sudo outright (safe, and
it makes the agent useless for anything system-level).

### Three-way project detection

`.git`, `.jj`, or `.mote`. A `.git`-only check files project memory globally in
a jj checkout, which is most of these repositories.

### Write requires a prior read

Overwriting a non-empty file not read this session is refused. The failure this
prevents is a plausible rewrite of a file the model never opened. Creating a
new file is unrestricted.

### Release profile

`lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`. Size
checked in CI; over 1.5 MB fails.

## Risks / Trade-offs

- **SSE assembly bugs** → reconstruct by index, ignore empty names, test against
  fixtures including split and interleaved calls.
- **Cache silently missed** → the status line shows the hit share, so a
  regression is visible on the next turn rather than on a bill.
- **Compaction loses a decision** → verbatim tail, a summarizer prompt aimed at
  decisions and open threads, and durable facts routed to memory with approval.
- **Prune loses detail** → stubs carry the spill path; anything pruned can be
  re-read.
- **Reasoning cannot be trimmed** → don't generate it by default; when it is on,
  it is part of the cost and the UI says so.
- **`panic = "abort"` skips destructors** → an explicit guard plus a panic hook
  that writes restore sequences before aborting.
- **Threads leak on cancel** → join with a deadline, kill the process group,
  detach rather than deadlocking the UI.
- **~200 lines of `unsafe` FFI** (libcurl, termios, ioctl) → keep every binding
  in `http.rs` and one terminal module, with the invariants stated at each
  call. A wrong `curl_easy_setopt` variadic type is undefined behaviour, so
  each option gets a typed wrapper and a test that the option took effect.
- **libcurl must exist at runtime** → it does on macOS and any Linux with curl
  installed; startup checks for it and exits with a plain message if missing.
- **Small binary creep** → pinned profile, CI check at 1.5 MB, no crate without
  a stated reason.
- **DeepSeek price change** → one price table; unknown models show tokens only.

## Migration Plan

New project. `aichat` is already uninstalled and its config archived.
`~/Projects/shell-agent` and `~/Projects/llm-functions` remain as references;
mote reads the latter's skills read-only.

## Open Questions

None.
