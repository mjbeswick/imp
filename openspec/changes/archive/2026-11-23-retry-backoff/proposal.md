# Proposal

## Why

A failed request is retried once, 800 ms later, whatever the cause. A rate
limit or an overloaded provider usually needs longer, and the provider's
`Retry-After` is ignored. After the turn fails, nothing says that `/retry`
and Ctrl+R exist.

## What Changes

- Rate limits, server errors, timeouts, dropped connections and empty
  streams retry up to five times, waiting 1, 2, 4, 8 and 16 seconds plus up
  to a quarter second of jitter. A `Retry-After` from the provider wins,
  capped at a minute.
- The status line counts down: `retrying in 4s · 2/5 · Esc stop`. Esc stops
  at once.
- Errors a retry cannot fix, such as a bad key or a bad request, fail at
  once.
- When retries run out, the error row ends with `Ctrl+R retry`.

## Keyboard and resources

No new keys. A waiting worker thread sleeps in short slices, so Esc is seen
within a tenth of a second; no extra process.
