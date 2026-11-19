# Tasks

- [x] 1.1 Render only on change. Verify: `an_idle_app_does_not_render` proves an idle app queues no frame while a key, event, or dirty state does, and the loop has no unconditional timer clause.
- [x] 1.2 Throttle the streaming spinner to ten frames a second. Verify: `the_spinner_runs_at_ten_frames_a_second` proves the spinner is not due under 100 ms and is due after it.
- [x] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
