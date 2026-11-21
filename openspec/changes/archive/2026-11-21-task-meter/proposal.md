# Proposal

## Why

Watching live tasks on a 65-column screen showed:

- A labelled step hid its command, and the approval row cut it to
  `run \`python3 -…?`, so the user approved a command they could not read.
- The spinner kept spinning while imp waited for an answer.
- Nothing showed while the model thought, before its first word or between
  steps, so a slow link looked like a hang.
- A running step had no time, and the task had no total.
- The status line added only each turn's last request, so the cost of a
  multi-step turn was undercounted.

## What Changes

- An activity row closes the transcript while a task runs:
  `⠋ thinking · 4s`, `⠋ Run test suite · 12s`, or
  `▌ Run test suite · waiting for you`, with no spinner while waiting.
- While a labelled command waits for approval, the full command is shown,
  wrapped and dim, above the approval row.
- While a task runs, the status line's right side shows
  `1m12s · 3 steps · ↓2.1K` in place of the directory.
- When a task ends, a dim line follows the reply:
  `42s · 5 steps · 18K in (92% cached) · 1.4K out · $0.004`, hidden at
  verbosity low and never sent to the model. Line mode prints it to stderr.
- Usage is reported after every request, so the cost counts every step.

## Capabilities

### Modified Capabilities

- `tui`: the activity row, the command under approval, the meter, the
  finish line, and per-request usage.
