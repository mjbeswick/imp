# Design

## Approaches

1. **Verb-only question.** `run it? y/N`, derived from the first word of the
   confirmation text. Chosen: one short row, no wrapping, and the tool row
   directly above still says exactly what will run.
2. **Delay the tool row until after approval.** Removes the duplication too, but
   a declined call would leave no row, and the spinner would start late.
3. **Drop the tool row while a prompt is open.** The row is the only place with
   the spinner and duration, and hiding it makes the transcript jump.

## Decisions

- The mapping is by first word: `run`, `write`, `save`, `delete`, anything else
  falls back to `approve?`. One function, no new field on the event.
- The full text is kept for the transcript lines, so `declined:` and
  `interrupted:` remain specific.
- The answer hint and countdown are always the suffix, so truncation can no
  longer hide them.
- Line mode is unchanged: it reads a typed answer and prints the same full text.
