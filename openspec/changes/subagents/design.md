# Design: subagents

## Context

Evidence from the current code (line numbers at the time of writing):

- `Session::turn` (`src/agent.rs` 993 to 1066) is the step loop: request, then
  dispatch each tool call, then request again until the model stops calling
  tools. It applies steering, pruning, compaction, job notices, and the step cap.
- `Session::request` (`src/agent.rs` 633) builds the wire from `self.history`
  and `self.active_tools()`. Its emergency truncation and image drop mutate
  `self.history` too, so whatever history is in place is the one that changes.
- `Session::say` (`src/agent.rs` 565) appends to the log and to `self.history`.
- `Session::active_tools` (`src/agent.rs` 607) filters the schema by
  `self.agent.allows(name)` (`src/store.rs` 27).
- `dispatch_inner` (`src/agent.rs` 1265) emits `Ui::ToolStart` and `Ui::ToolEnd`
  around `run_tool` (`src/agent.rs` 1295), which matches on the tool name.
- `Session::new` (`src/agent.rs` 351) builds history as the system prompt, then
  the agent's instructions as a system message.
- `Hooks` (`src/agent.rs` 20 to 60) carries `ui`, `confirm`, `ask`, and the
  rest. Permission prompts go through `confirm` and `confirm_always`.
- `Agent` (`src/store.rs` 8) has `name`, `instructions`, `tools`, `model`,
  `thinking`, `effort`. The session holds the list in `roster`.
- Tests use `transport::Fake` (`src/transport.rs` 61) with scripted replies.

## Decisions

1. **Child run inside the same Session.** The child swaps in its own history,
   profile, and model settings, runs the same request and dispatch path, then
   restores everything. Swapping is correct because `request`'s retry paths
   already mutate `self.history`, so they keep acting on the child's history.
   The alternative, a second `Session` with its own transport, cannot share the
   transport box, the permission state, or the always-allowed set.

2. **Child hooks wrap the parent hooks.** A `ChildHooks` struct forwards
   `confirm`, `confirm_always`, `ask`, `ask_choices`, `allowed`, `answered`,
   `last_wait`, `run_attached`, and `can_foreground` to the parent. It forwards
   `ToolStart` with the child name prepended to the summary, forwards
   `ToolEnd` and `Usage`, and drops `Content` and `Reasoning` so the child's
   prose does not appear in the parent transcript.

3. **Depth 1 by a flag.** `Session` gets a `child: bool`. While it is set,
   `active_tools` omits `task`, and `run_tool` refuses a `task` call. Depth is
   enforced even if a profile lists `task` in its `tools`.

4. **Nested child records.** While the child flag is set, `say` writes each
   message to the session file as `{"t":"child","call":"<task call id>","m":…}`
   instead of `{"t":"msg","m":…}`, and keeps it in a per-child transcript held
   by the parent session. The existing `Log::append` (`src/store.rs` 492) gains a
   variant for this record kind. Resume loads only `t:"msg"` records into the
   wire history, so nested records never reach the model. Resume builds the
   child-transcript map from the `t:"child"` records, keyed by call id. The
   session spec allows nested records (`openspec/specs/session/spec.md`
   "Children do not create sessions").

4a. **Drill-in view.** The chat's tool row for a `task` call takes a key to open
   the child transcript. The view is read-only: it lists messages in order, shows
   each tool call with its arguments and result (reusing the existing tool-row
   rendering), and returns with Esc. It reads the parent's child-transcript map
   and does not touch the running session's history. Opening a child that is
   still running shows its messages so far and follows new ones.

5. **Profile lookup.** `task` resolves `agent` against `self.roster`. An unknown
   name returns a refusal listing the available names, in the same form as
   `/agent nope`. A missing `agent` argument is a refusal.

6. **Step cap.** The child uses `cfg.max_steps` as its own limit. Hitting it
   returns the last text with a note that the child stopped at the cap.

7. **Settings restored on every path.** The child's model, thinking, and effort
   are saved before the run and restored after, including on error and cancel.
   The history, agent, and child flag are restored by the same guard.

## Risks

- Swapping state is easy to get wrong. Mitigation: one guard function that
  restores all fields, with a test that a failed child leaves the parent
  history and profile unchanged.
- A child could prompt often. The permission policy is unchanged, so this is
  the same prompt rate as the parent doing the work itself.
- Cost. A child is an extra inference chain, as the mote-v1 design noted. The
  user chose this knowingly; the tool description tells the model to use it
  for independent work only.

## Assumptions

- `Ui::ToolStart` and `Ui::ToolEnd` pairs render correctly when the child's
  rows appear inside the parent's turn. Checked in tasks, not assumed.
- `max_steps` is a sensible cap for a child as well as for a parent.

## Expected size

About 300 lines of Rust across `src/agent.rs` and `src/store.rs`, with tests.
Expected binary growth is under 20 KB. `./scripts/size.sh` records the measured
figure at archive time.
