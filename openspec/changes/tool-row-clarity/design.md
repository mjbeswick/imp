# Design

## Finishes

`Finish::of(ok, output)` in `tui.rs` turns a result into a finish. Every
command result already starts `exit N`, so it reads the code there. Line
mode calls the same function.

| Finish   | When                                          | Mark | Colour |
|----------|-----------------------------------------------|------|--------|
| Done     | `ok`                                          | `✓`  | success mark, tool row |
| Exited   | not `ok`, output starts `exit N` with `N > 0`  | `!`  | accent, tool row |
| Declined | output starts `refused: the user declined`    | `✗`  | muted, `Role::Context` row |
| Failed   | anything else: refused, error, timeout        | `✗`  | error |

A timeout reports `exit -1`, so it stays Failed.

Approaches:

- **A distinct mark for a non-zero exit (chosen).** A failing test run still
  stands out, but it no longer looks like the tool broke. `!` is ASCII, one
  column wide, and draws on kmscon.
- **`✓` with a muted `exit 1`.** Hides failing tests. Rejected.
- **Keep `✗` and only fix the preview.** Leaves the false alarm. Rejected.
- **A `code` field on `ToolEnd`.** The fact is already in the output's first
  line, and a field would touch every event constructor. Rejected; one
  shared reader instead.

## Previews

An exited row shows the last three output lines after `exit N`, each on its
own line at the two-column indent and wrapped there, with `… +N lines
(Ctrl+E)` above them when more came before. Errors come last, so the tail
beats the head. A failed row keeps its one reason line. A declined row reads
`declined`.

## Expansion

`App.tool` stays the newest row and `App.done` holds the latest turn's
earlier ones. `begin_task` clears both, and so does every transcript clear,
through `clear_entries`, so no row points at a stale entry.
Ctrl+E expands every row in it when any is collapsed, otherwise collapses
them all. With the mouse on, each of those rows is a click target that
toggles only itself. Rows from earlier turns and replayed sessions keep no
data and do not expand.

- **Every row of the latest turn (chosen).** One key opens the `✗` you are
  looking at, wherever it sits in the turn.
- **Ctrl+E steps back one row per press.** Hidden state, and the row you want
  may be five presses away. Rejected.
- **Keep every row of the session.** Unbounded memory on a 512 MB device.
  Rejected.

Expanded arguments: a shell call shows `$ <command>`. Other tools show one
`key: value` line per argument, strings unquoted, nested values as compact
JSON, each wrapped at the indent. `path` leads; the rest follow in key
order, as `serde_json` keeps no other. `description` is left out: the row
already says it.

## Declines

The TUI stops pushing the `declined: …` info row. `TaskTotals` gains
`declined`, set by the same check that sets the goal's flag, and the worker
skips `suggest` when it is set.

## Size

Expected: 1.5 to 2.5 KB.
