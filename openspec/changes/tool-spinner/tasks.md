# Tasks

- [x] 1.1 Animate the running tool row. Add the running flag, rewrite the entry with the current frame on each tick, and stop the clock when no tool runs. Verify: a 300 ms call shows at least two frames and the finish replaces the glyph with `✓` or `✗`.
- [x] 1.2 Remove the spinner from the status line and start it at the model name. Verify: no frame glyph appears in the status line at idle, while streaming, or while a tool runs.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
