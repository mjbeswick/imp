# Tasks

## 1. The gate

- [x] 1.1 Write `scripts/check-drift.py`: fail when files outside `openspec/` changed and nothing under `openspec/changes/` did; fail when code changed and there is no active change. Verify: a simulated source edit with no task tick exits non-zero; ticking a task makes it pass.
- [x] 1.2 Write `scripts/check.sh`: drift, `openspec validate --strict`, `cargo test --quiet`, `scripts/size.sh`, each printed with its own line. Verify: a clean tree passes and reports four green lines.
- [x] 1.3 Write `AGENTS.md` stating the rule, the command to run, and what to do when no change covers the work. Verify: the file is read by the agent and names `scripts/check.sh`.
- [x] 1.4 Remove the dead-code warnings the gate surfaced (unused imports, constants, helper methods) so `cargo build` is quiet. Verify: `cargo build --release` prints no warnings.
- [ ] 1.5 Project `opencode.jsonc`: deny edits to `target/**`, ask before editing change specs, deny `git push`, `cargo publish`, and `rm -rf`. Verify: config parses and `target/` edits are refused.
- [ ] 1.6 A `spec` agent that may only edit `openspec/**`. Verify: as `spec`, an edit to `src/main.rs` is denied.
