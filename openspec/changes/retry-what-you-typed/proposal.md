# Proposal

## Why

`/retry` picks "the last user message" twice: the chat from its rows, the
agent from its history. They disagree. A `!cmd` row is a user row the agent
never sees, and each goal round adds a user message the chat never shows.
After `!ls`, the chat would keep the wrong rows while the agent resent an
older message.

## What Changes

- The chat picks the message: the last one the user typed, skipping `!cmd`
  and `/goal` rows, without a `(steered)` mark.
- `Cmd::Retry` carries that text; the agent cuts its history at the last user
  message with the same text, and the log records the text.
- Not found, for example after compaction: `nothing to retry`, nothing sent.

## Keyboard and resources

No keys change. No cost.
