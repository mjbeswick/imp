# Tasks

## 1. Slash command

- [x] 1.1 `/goal pause` and `/goal resume` keep the goal and stop or restart its rounds; `/goal clear` removes it and `/goal off` is an alias; tests
- [x] 1.2 The status line shows `goal n/m` or `goal paused`; tests

## 2. Agent tool

- [x] 2.1 A `goal` tool with `set` stores a pending goal and asks the user to confirm it; yes starts round 1, no drops it and tells the agent; tests. Judgement call: the tool asks through `Hooks::confirm` in the same call and does not keep a separate pending state, so yes starts round 1 at once and no keeps nothing.
- [x] 2.2 The agent cannot start rounds without the confirmation; tests

## 3. Persistence

- [ ] 3.1 A goal is written to the session metadata when it changes and read back on open or resume; a running goal restores as paused; tests

## 4. Goal UX

- [x] 4.0 `/goal` shows text, round, limit and state; `/goal edit` changes the text without a round; `/goal <text>` asks before replacing; tests

## 5. Docs

- [ ] 5.1 README describes the `/goal` subcommands, the `goal` tool, and goal persistence
