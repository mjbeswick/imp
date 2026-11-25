# Tasks

## 0. Baseline check

- [x] 0.1 Diagnose and correct the pre-existing `composer_rows_fit_the_terminal` assertion on fetched `main`, which counts an extra `x` outside the draft; targeted test and `cargo test`.

## 1. Contract and dispatch

- [x] 1.1 Add batch question schema, validation, ordered structured results and legacy compatibility in `src/agent.rs`; targeted tests and `cargo test`.

## 2. Interactive front ends

- [x] 2.1 Present progress, header, multiple selections, and custom answers for batches in `src/tui.rs`, preserving legacy picker behavior; TUI tests and `cargo test`.
- [x] 2.2 Support headers, descriptions, and several selections in `src/main.rs` line mode; parser tests and `cargo test`.

## 3. Safety

- [ ] 3.1 Dismissing any question aborts remaining tool calls in the response and the turn; regression test and `cargo test`.

Each task also requires `openspec validate --all --strict` and
`./scripts/size.sh` before its scoped commit, followed by release reinstall
for any source commit.
