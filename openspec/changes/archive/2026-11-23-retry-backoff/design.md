# Design

## Approaches

- **Exponential backoff with jitter, `Retry-After` first (chosen).** The
  usual shape for rate limits; jitter keeps several sessions from retrying in
  step.
- **A fixed delay, more attempts.** Either too slow for a blip or too fast for
  a rate limit.

## Details

`Live::post` reads `retry-after` from the kept response headers into a
thread-local that the agent takes after a failure, so the `Transport` trait
does not change. The request loop calls one `backoff` step for every
retryable failure, which emits `Ui::Retry { secs, attempt, of }` each second
and returns early when the turn is cancelled. The base delay is a session
field so tests run in milliseconds. The one-off retries for a full context
and refused images stay as they are.
