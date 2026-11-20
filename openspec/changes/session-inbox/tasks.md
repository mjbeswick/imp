# Tasks

- [x] 1.1 Move the worker loop into `spawn_worker`, which returns the session's command sender, its own event receiver, and its id; no behaviour change. Verify: `cargo test` passes unchanged.
- [ ] 1.2 Move expand to Ctrl+E. Verify: key test for Ctrl+E, and Ctrl+O no longer expands.
- [ ] 1.3 Add `Pane` for sessions out of view, swapped with the fields in view, each with its own worker, mode handle, cancel flag, and job directory. Run up to four: `/new` and `/sessions` resume open a pane, the oldest idle one closes for room, a refusal when all are busy. Verify: tests for each scenario.
- [ ] 1.4 Ctrl+N and Ctrl+P switching with drafts and scroll kept, and the status line position. Verify: switching tests.
- [ ] 1.5 Share the buffer budget across panes with separate addresses, and free a closed pane's buffers. Verify: buffer tests.
- [ ] 1.6 The inbox: Ctrl+O and `/inbox`, rows, states, order, Enter, `y` `n` `a`, `x`. Verify: inbox tests for each scenario.
- [ ] 1.7 Attention: the waiting count, one bell, no deadline out of view. Verify: tests for the count and the paused deadline.
- [ ] 1.8 The `?` keys card and bottom-edge hints. Verify: card and hint tests.
- [ ] 1.9 Help, README, checks, size recorded, and reinstall. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
