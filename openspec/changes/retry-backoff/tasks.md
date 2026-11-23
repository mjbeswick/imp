# Tasks

## 1. Backoff

- [x] 1.1 Retryable failures back off 1, 2, 4, 8, 16 s with jitter up to five times; `Retry-After` wins; others fail at once; tests
- [x] 1.2 Esc during a wait stops the turn; the status line counts down; tests

## 2. Manual

- [x] 2.1 A failed turn's error row ends with `Ctrl+R retry`; test
