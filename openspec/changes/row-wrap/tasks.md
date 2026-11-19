# Tasks

- [x] 1.1 Give tool rows one prefix rule. Successful and failed rows start with the mark in column one; continuations, previews, questions, and options indent two columns. Verify: a success row and a failure row begin in the same column.
- [x] 1.2 Stop truncating summaries at 120 characters and let the renderer wrap the whole command or question; keep the duration at the end. Verify: a 300-character command spans rows and shows its time, and a long failure reason wraps.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
