# Proposal

## Why

`/goal <text>` keeps a session working across rounds until the agent writes
`GOAL MET` or `GOAL BLOCKED`, or the round limit is reached. Three things are
missing. The agent cannot set a goal itself. A goal cannot be paused: `/goal off`
ends it, and the goal is gone. A goal is not kept with its session, so a resumed
session loses it.

## What Changes

- `/goal <text>` sets a goal and starts its rounds, as now. `/goal` alone shows
  the goal and its round. `/goal pause` keeps the goal and stops its rounds.
  `/goal resume` starts a paused goal again at its next round. `/goal clear` removes
  the goal. `/goal off` stays as an alias for `/goal clear`.
- A `goal` tool lets the agent set a goal for its session, with the round limit
  from `goal_rounds`. The agent cannot start the rounds itself: the goal waits for
  the user to confirm it in the chat, with the goal text shown.
- A confirmed or paused goal is saved with the session, so `--resume` restores it.
  A paused goal stays paused after a restart.
- The status line shows `goal n/m` for a running goal and `goal paused` for a
  paused one.

## Out of Scope

- A goal that runs without a session open.
- Changing the `GOAL MET` and `GOAL BLOCKED` signals.

## Keyboard and resources

No new keys. The tool confirmation uses the existing confirmation prompt.

## Goal UX

- `/goal` shows the goal's text, its round and limit, and its state: running,
  paused, or waiting for your confirmation.
- `/goal edit <new text>` changes the text and keeps the round. It starts no
  rounds. A running goal uses the new text from its next round.
- `/goal <text>` while a goal exists asks first whether to replace it. Declining
  keeps the current goal unchanged.
