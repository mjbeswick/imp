# The agent loop

What imp does between pressing Enter and the reply. Everything here lives in
`Session::turn` in `src/agent.rs`; the behaviour it must keep is specified
in `openspec/specs/agent-loop/` and its neighbours.

## The shape of a turn

```
 your message
      │
      ▼
 ┌──────────────────────────────────────────────┐
 │ step (up to 24)                               │
 │  1. wait while you edit a queued message      │
 │  2. stop if you pressed Esc                   │
 │  3. add anything you steered in               │
 │  4. prune, then compact if the context is big │
 │  5. note background jobs that ended           │
 │  6. one request to the model (with retries)   │
 │  7. no tool calls? ── the turn is done ───────┼──▶ reply
 │  8. run each tool call, add its result        │
 └────────────────────┬─────────────────────────┘
                      └── next step
```

A turn is one message from you and every step the model takes to answer it.
Each step is one request. The model either answers, which ends the turn, or
asks for tools, whose results go back in the next step.

## Before the first step

The **system prompt** is built when the session starts and again on
`/reload` or `/cd`. In order:

1. imp's own baseline: how to use the tools, and when to ask.
2. The session directory and a private scratch folder, `$IMP_TMP`.
3. Instructions from `AGENTS.md`: global, the repository root, then the
   session directory.
4. Memory notes, by title only, up to 40.
5. Skills, by name and description only, until the model loads one.

The agent's own instructions follow as a system message. Switching agent
appends a new one instead of rewriting the prompt, so the provider's prompt
cache stays valid.

Each turn then starts by:

- telling the model today's date, when the day has changed since it was
  last told;
- adding your message, with any attached images.

## The request

Each provider has an adapter that turns the history into its wire format:
one shared by DeepSeek, OpenAI, Ollama and custom endpoints, and one each
for Anthropic and Gemini. The reply streams back as it is written, so text
and reasoning appear on screen as they arrive. Every request's tokens are
counted, retries included.

When a request fails:

| Failure | What happens |
|---|---|
| rate limit, server error, timeout, dropped connection, empty stream | up to 5 retries, waiting 1, 2, 4, 8, then 16 s, or the provider's `Retry-After` up to 60 s; the status line counts down and Esc stops it |
| context too long | old tool results are cut to 160 characters, then one retry |
| images refused | the images are dropped, then one retry |
| anything else, such as a bad key | the turn ends with the error and `Ctrl+R retry` |

## Tool calls

The model may ask for several tools in one step; they run in order. For
each call imp:

1. parses its arguments; bad JSON goes back to the model as an error;
2. draws a tool row with a short description;
3. checks permission, and asks you when it must;
4. runs the tool;
5. replaces any secret value in the output with `[redacted NAME]`;
6. finishes the row with a preview, or a diff for edits;
7. adds the result to the history for the next step.

**Permission** for a shell command is decided in this order:

1. The deny list wins in every mode: `dd`, `mkfs`, `rm -rf` outside the
   project, force push and similar.
2. Privileged commands are refused unless `allow_sudo=true`.
3. In `yolo`, everything else runs.
4. In `normal`, a read-only command line runs: safe commands, pipes between
   them, no redirects, `&&` or `$(…)`. So do shapes you allowed with `s` or
   `a`. Anything else asks.
5. In `strict`, everything asks.

File tools reach only the session directory, imp's config folder and its
output folder; any other path is refused, with no prompt. `yolo` turns this
jail off. Shell commands are not jailed, which is why they ask. Commands and
jobs run without secret-looking environment variables unless `pass_env`
names them.

A **declined** prompt does not end the turn: the model is told you declined
and can try another way. It does stop a running goal, and skips the
suggested next message.

Results over 4 KB are written to a file; the model gets the head, the tail
and the path. Long commands run as **background jobs**, which the model
polls with the `job` tool.

## Talking to a running turn

- **Steer** (Enter while a turn runs, the default): your message joins the
  history before the next step, and shows marked `(steered)`.
- **Queue** (Ctrl+Q, then Enter): the message waits and starts its own turn
  when this one ends. Opening a queued message to edit it holds the turn
  before its next step.
- **Esc** stops at the next check: before a step, or during a request or a
  retry wait.

## When a turn ends

- The model answered without tools: the reply is the last thing it said.
- 24 steps ran: imp pauses and says so; send another message to continue.
  `max_steps` changes the limit.
- You pressed Esc: `cancelled`.
- A request failed for good: the error, with `Ctrl+R retry`.

After a turn that ended well, imp asks the model for your likely next
message and shows it greyed in the empty composer; → or Enter takes it.
`suggest=off` turns this off.

## Keeping the context small

The history grows with every tool result. Two steps keep it in bounds, both
checked before each request:

- **Prune**, past 45,000 estimated tokens (`prune_at`): tool results older
  than the last ten messages are cut to their first 240 characters.
- **Compact**, past 60,000 (`compact_at`), or on `/compact`: a separate
  request summarises the conversation. The summary replaces everything
  before your fourth-newest message; the system prompt and the recent tail
  stay. The summary may suggest up to three durable facts, which imp offers
  to save as memory notes.

Compaction loses detail. Facts that must last belong in memory.

## Goals and check-ins

`/goal <text>` runs turns until the goal is met. Each round asks the model
to end with `GOAL MET` and its evidence, or `GOAL BLOCKED` and why. Rounds
stop at either, after a declined prompt, when you take over by sending a
message, or after `goal_rounds` rounds (5 by default, at most 20).

A model can also check back without you. `job check` with `after_s`, or
`notify` on a background job, wakes the session later with the job's state.
The session then reports as if it had replied, so out of view it joins the
Ctrl+O queue. At most four check-ins wait per session.

## What is saved

Every message is appended to the session's log in
`~/.local/state/imp/sessions/` as it happens, along with events: agent
switches, directory changes, compactions, reloads and retries. Resuming
replays the log, so the model continues with the real context: reasoning,
tool calls and results included. A retry is recorded too, so a resumed
session drops the same reply.

## Where to change it

| To change | Look at |
|---|---|
| the step limit, prune and compact thresholds | `max_steps`, `prune_at`, `compact_at` in the config |
| the loop | `Session::turn` and `Session::request` in `src/agent.rs` |
| permission rules | `src/perm.rs` |
| the tools | `schema()` in `src/agent.rs`, `src/tools.rs` |
| what the chat shows | `src/tui.rs` |
| the rules the loop must keep | `openspec/specs/agent-loop/spec.md` |
