# Design

## Approaches

- **Shift+Tab for mode, Tab for agent (chosen).** The user's proposal; one
  key each, no picker.
- **Leave yolo out of the cycle.** Safer, but the user chose to include it;
  yolo already shows in red on the prompt and rule.
- **Tab cycles agents with text in the draft too.** Surprising mid-sentence.
  Rejected.

## Keys

Shift+Tab sets the session's live mode to the next of `normal`, `strict`,
`yolo`, as `/mode` does, and writes `perm::mode_line`. Tab on an empty draft
with no picker open moves to the next agent profile, by `/agent`'s path; while
a turn runs the choice is kept and applied when it ends, and the status line
shows `→ <agent>` until then.

## Status line

`<agent> · <mode> · <model> · …`, with the agent left out when it is the
default and only one exists. It sits before the model so it is the last
thing cut.

## `/keys`

Runs the code that `?` runs. `/help` ends with `Keys: ? or /keys`.

## Order

The command list is modified after `message-queue`; archive after it.

## Size

Expected: about 1 KB.
