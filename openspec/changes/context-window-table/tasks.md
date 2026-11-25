# Tasks: context window table entries

## 1. Table

- [x] 1.1 Test: `window("gpt-4.1-mini", None)` is `Some(1_047_576)`, and `window("gemini-2.5-pro", None)` is `Some(1_048_576)`.
- [x] 1.2 Test: a configured size overrides the table for `gpt-4.1-mini`, and an unlisted model gives `None`.
- [x] 1.3 Implement: the two entries in `WINDOWS` in `src/context.rs`, each with its source comment.

## 2. Docs and validation

- [x] 2.1 Document which models show a percentage in `README.md`. No sentence named them before, so one was added beside the status line description.
- [x] 2.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 836 unit tests pass. The two `tests/tty.rs` failures predate this change.
