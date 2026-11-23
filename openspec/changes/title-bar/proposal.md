# Proposal

## Why

Nothing on screen names the session or shows the time unless several
sessions are live, and then the name crowds the status line.

## What Changes

- A title bar on the top row: the session title on the left, `imp` before
  the first message, and the local time `HH:MM` on the right.
- With several sessions, the position, such as `2/3`, leads the title and
  leaves the status line.
- Flat colour, no gradient: the theme's select background, reversed in mono.
- No title bar under eight rows, where every row is needed.

## Keyboard and resources

No keys. One row of height. The clock repaints the title row once a minute;
the idle loop already wakes every five seconds.
