# Design

## Finishes

`Ui::ToolEnd` gains `code: Option<i32>`, set when a command ran to an exit
code. One shared function turns `ok`, `code`, and the output into a finish:

| Finish   | When                                          | Mark | Colour |
|----------|-----------------------------------------------|------|--------|
| Done     | `ok`                                          | `✓`  | success mark, tool row |
| Exited   | not `ok`, `code` is `Some(n)` with `n != 0`   | `!`  | accent |
| Declined | output starts `refused: the user declined`    | `✗`  | muted |
| Failed   | anything else: refused, error, timeout        | `✗`  | error |

Approaches:

- **A distinct mark for a non-zero exit (chosen).** A failing test run still
  stands out, but it no longer looks like the tool broke. `!` is ASCII, one
  column wide, and draws on kmscon.
- **`✓` with a muted `exit 1`.** Hides failing tests. Rejected.
- **Keep `✗` and only fix the preview.** Leaves the false alarm. Rejected.
- **Parse `exit N` from the output in each front-end.** Two parsers for one
  fact the agent already has. Rejected in favour of `code`.

## Previews

An exited row shows the last three output lines after `exit N`, each on its
own line at the two-column indent and wrapped there, with `… +N lines
(Ctrl+E)` above them when more came before. Errors come last, so the tail
beats the head. A failed row keeps its one reason line. A declined row reads
`declined`.

## Expansion

`App.tool` becomes `App.tools: Vec<ToolRow>`, cleared when a turn starts.
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
JSON, each wrapped at the indent.

## Declines

The TUI stops pushing the `declined: …` info row. `TaskTotals` gains
`declined`, set by the same check that sets the goal's flag, and the worker
skips `suggest` when it is set.

## Size

Expected: 1.5 to 2.5 KB.
