# Design

## Decisions

- **Search on Enter, not per key.** Reading every log on each keystroke is
  too slow on a Pi Zero. Enter with no title match runs one scan.
- **Plain substring, ASCII case folding.** Byte offsets stay valid for the
  snippet. Non-ASCII letters match case-sensitively.
- **Message text only.** Tool output and JSON keys are skipped, so hits are
  things the user or the agent said.
- **Results replace the rows.** The title becomes `search "…"`, the filter
  clears, and the usual Enter, `r`, `d`, `n`, and Esc apply.
- **Spinner in the right column.** It sits before the count, so labels and
  alignment do not change. The picker repaints ten times a second only while
  it is open on sessions and one is working.
- **`Drop` for `Jobs`.** A session's worker drops its jobs when it quits,
  which sends SIGTERM to each running process group. No wait, so closing a
  session never blocks the interface.

## Keyboard

No new keys.

## Resources

One file scan per search. Twenty results at most.
