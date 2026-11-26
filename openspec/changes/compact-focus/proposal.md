# Compact with focus instructions

## Why

`/compact` summarises the conversation with one fixed prompt. A user who knows what the next step needs has no
way to say so, so the summary may drop the decision or file that matters. An argument lets the user tell the
agent what to keep in the summary.

## What Changes

- `/compact <text>` adds the text to the summary request as focus instructions, such as
  `/compact keep the auth decisions and the failing test names`.
- `/compact` with no text runs exactly as it does today.
- The focus text is capped at 500 characters. Longer text is cut, and the report says so.
- The summary budget, the summary's format, and the last four exchanges kept verbatim are unchanged.
- The command's help line shows the argument form.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `conversation-context`: a new requirement for focus instructions on a user's compaction request.
- `tui`: a new requirement for the `/compact` argument at the composer.

## Out of Scope

- Choosing the range of messages to compact. The user chose focus instructions over a range.
- Automatic compaction at the token budget. It takes no argument.
- Changes to the summary's length or the memory facts it saves.

## Impact

- Code: the `compact` command arm and `Cmd::Compact` in `src/tui.rs`, the worker call, `Session::compact` in
  `src/agent.rs` and its prompt, and the help line in `src/help.rs`.
- Tests: the compaction tests in `src/agent.rs` and the `/compact` command tests in `src/tui/tests.rs`.
- Specs: `openspec/specs/conversation-context/spec.md` and `openspec/specs/tui/spec.md`, through deltas.
