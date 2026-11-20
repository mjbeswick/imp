# Proposal

## Why

The inbox is the generic picker squeezed into a 72-column overlay. Titles are
cut to `Giv…`, a filter row is wasted on at most four sessions, and one line
per session cuts off the command or question the user needs to see.

## What Changes

- The inbox takes over the transcript area and composer; the status line
  stays.
- One card per session: the session number (as in the `2/3` status), the full
  title, the state and age at the right, and below it what the session needs
  or is doing:
  - an approval: the command, cut in the middle, then the answer chips
  - a question: the question
  - replied: the last two lines of the reply
  - working: the running tool row
  - idle: the last line of the reply
- The session in view is marked `in view`.
- The footer lists the keys of the highlighted card only.
- `1`–`4` open that session directly. The filter is gone.

## Capabilities

### Modified Capabilities

- `inbox`: cards in place of picker rows, number keys.

## Impact

- `src/tui.rs`, `README.md`.
