# Tasks: /skill command

## 1. Queue

- [x] 1.1 Test: a queued skill is put before the next prompt's text. Covered by `a_queued_skill_goes_with_the_next_prompt_only` through `compose_turn`, which the worker calls on `Cmd::Prompt`.
- [x] 1.2 Test: after that turn, a second prompt sends only its own text. Covered by the same test.
- [x] 1.3 Test: an unknown name is refused and queues nothing. Covered by `an_unknown_or_switched_off_skill_is_refused` through `skill_body`. The worker's info line is not asserted by a test of its own.
- [x] 1.4 Implement: `Cmd::LoadSkill`, the worker's `pending_skills` queue, and `compose_turn` on `Cmd::Prompt` in `src/tui.rs`. A `LoadSkill` during a goal round is queued like other commands.

## 2. Command

- [x] 2.1 Test: `/skill config` sends `Cmd::LoadSkill("config")`. Covered by `slash_skill_queues_an_enabled_skill`. The toast is not asserted.
- [x] 2.2 Test: `/skill nosuch` sends nothing. Covered by `slash_skill_with_an_unknown_name_sends_nothing`. The error row is not asserted.
- [x] 2.3 Test: bare `/skill` shows the usage line. Covered by `bare_slash_skill_shows_the_usage_line`; the enabled-name list is not asserted.
- [x] 2.4 Implement: the `skill` arm in `run_command` in `src/tui.rs`, and the `COMMANDS` and `GROUPS` entries in `src/help.rs`. The count test in `src/help.rs` moved from 31 to 32 commands.

## 3. Docs and validation

- [x] 3.1 Update the tools paragraph in `README.md` with the `/skill` behaviour.
- [x] 3.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 847 unit tests pass. The two `tests/tty.rs` failures predate this change.
