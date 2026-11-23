# Proposal

## Why

While a question or confirmation is open, the rule above the composer says
`↑ edit queue`, but ↑ does nothing there, so the hint is false. The queue is
also invisible on the question itself, so a user who queued `use pnpm` cannot
tell the model's question will not see it until the turn ends.

## What Changes

- The rule never offers `↑ edit queue` while a question or confirmation is
  open.
- The question's hint row ends `· n queued` while messages wait.
- A requirement pins the rest: a question never takes a queued message as its
  answer, and Esc leaves the queue as it was.

## Keyboard and resources

No keys change. One short phrase on the hint row; on a narrow screen the row
wraps like any other.
