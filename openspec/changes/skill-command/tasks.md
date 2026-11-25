# Tasks: /skill command

## 1. Queue

- [ ] 1.1 Test: `Cmd::LoadSkill("config")` followed by `Cmd::Prompt("x")` sends a turn whose text starts with the config skill's body and ends with `x`.
- [ ] 1.2 Test: after that turn, a second prompt sends only its own text.
- [ ] 1.3 Test: `Cmd::LoadSkill` for an unknown name emits an info line and queues nothing.
- [ ] 1.4 Implement: `Cmd::LoadSkill`, the worker's `pending_skills` queue, and the prepend on `Cmd::Prompt` in `src/tui.rs`.

## 2. Command

- [ ] 2.1 Test: `/skill config` sends `Cmd::LoadSkill("config")`, and a toast confirms it.
- [ ] 2.2 Test: `/skill nosuch` shows an error and sends nothing.
- [ ] 2.3 Test: bare `/skill` shows the usage line with the enabled skill names.
- [ ] 2.4 Implement: the `skill` arm in `run_command` in `src/tui.rs`, and a `COMMANDS` entry in `src/help.rs`.

## 3. Docs and validation

- [ ] 3.1 Update the skills paragraph in `README.md`.
- [ ] 3.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome.
