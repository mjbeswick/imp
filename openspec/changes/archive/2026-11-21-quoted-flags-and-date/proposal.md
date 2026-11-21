# Proposal

## Why

A review of `web-lookups` found two gaps.

- The safe-list flag checks read the masked words, where quoted text is
  blanked, so `find . '-delete'`, `sort '-o' f f`, and `rg '--pre' sh x` ran
  unprompted. The shell strips those quotes before the program sees them.
  `sort --compress-program` and `git log`/`git show --output` were not
  checked at all.
- The date was composed into the prompt once at start-up, and `/new` reuses
  that prompt, so a long-running imp, as under tmux, gives every later session
  the start-up date, and a session open past midnight keeps yesterday.

## What Changes

- Flag checks read each word as the program will, with quotes removed.
  `sort --compress-program` and `--output` on `git log` and `git show` ask.
- The date arrives as a system note before each day's first request, in every
  session. The prompt keeps the rule about fresh data.

## Capabilities

### Modified Capabilities

- `permissions`: quoted flags are flags.
- `instructions`: the date is noted per day.

## Impact

- `src/perm.rs`, `src/agent.rs`, `src/prompt.rs`.
