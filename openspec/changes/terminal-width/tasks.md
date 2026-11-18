# Tasks

- [x] 1.1 Carry a column count from the UI to the session. Verify: a session built with 60 columns reports 60 and piped mode reports 0.
- [x] 1.2 Send one environment line when the width changes. Verify: two turns at 60 carry one note, a third at 100 carries a new one, and an unchanged width adds nothing.
- [ ] 1.3 Add the width rule to the baseline prompt. Verify: the prompt names the terminal width and a test asserts it.
- [ ] 1.4 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
