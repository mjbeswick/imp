# Design

## Findings

- A question blocks its turn in the ask tool. Queued messages are delivered
  when the turn ends, never into the question, which is right: they were
  written before the question existed.
- Esc on a question stops the turn; the queue then waits, as for any stop.
- `Ev::Question` and `Ev::Confirm` replace whatever prompt is open, such as a
  picker or a rename row, without a word. Rare; left alone here.

## Approaches

- **Show the count, change nothing else (chosen).** Fixes the false hint and
  tells the user where their messages are.
- **Let Tab load the oldest queued message as the answer.** Useful when the
  queued text answers the question, but a hidden key and a restore rule for
  Esc. Not specified.
