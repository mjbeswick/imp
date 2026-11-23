# Tasks

## 1. Slash command

- [ ] 1.1 `/goal pause` and `/goal resume` keep the goal and stop or restart its rounds; `/goal clear` removes it and `/goal off` is an alias; tests
- [ ] 1.2 The status line shows `goal n/m` or `goal paused`; tests

## 2. Agent tool

- [ ] 2.1 A `goal` tool with `set` stores a pending goal and asks the user to confirm it; yes starts round 1, no drops it and tells the agent; tests
- [ ] 2.2 The agent cannot start rounds without the confirmation; tests

## 3. Persistence

- [ ] 3.1 A goal is written to the session metadata when it changes and read back on open or resume; a running goal restores as paused; tests

## 4. Docs

- [ ] 4.1 README describes the `/goal` subcommands, the `goal` tool, and goal persistence
