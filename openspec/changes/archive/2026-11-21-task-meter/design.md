# Design

## Usage

`Ui::Usage { hit, miss, output, model }` goes out after each request. The
end-of-turn usage event carries only the context estimate, so nothing is
counted twice. `Session.task` sums the same numbers, and the step count, for
line mode.

## Task

`App.task: Option<Task { start, steps, hit, miss, out, chars }>` starts on
send or wake and ends on `Done`. Streamed output is counted as characters / 4
until the provider reports real numbers, then the larger wins.

## Activity row

The activity row is drawn in `frame()`, not stored as an entry:

- A running tool keeps its own row, and the spinner tick adds ` · Ns` after
  one second.
- While a prompt is open, that row shows `▌ … · waiting for you` with no
  spinner.
- With no tool running and nothing streamed since the last step, a
  `⠋ thinking · Ns` row is drawn after the transcript.

## Command under approval

The composer gains the same multi-row path the ask rows use. For a labelled
command, the command is shown `$ `-prefixed, dim, and wrapped, capped at a
third of the screen, above the one-line approval row.
