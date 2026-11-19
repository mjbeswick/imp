# Proposal

## Why

The `/cd` browser cannot choose the folder it is showing. Enter opens a folder
instead of choosing it, Tab is the only way to choose, and Tab chooses the
highlighted row, so once inside the wanted folder there is nothing to pick. In
a folder with no subfolders the only row is `../`, and Tab there chooses the
parent. Choosing then reports the new root twice.

## What Changes

- Enter chooses the highlighted folder as the session root.
- Tab and Right open the highlighted folder; Left goes to the parent.
- A `./` row, first in every listing, stands for the folder being browsed, so
  it can be chosen. Rows read `./`, `../`, then subfolders.
- The footer reads `Enter choose · Tab/→ open · ← up · Esc close`.
- A root change is reported once: `cwd: ~/path (prompt cache reset)`.
- `/image` opens folders with Enter, Tab, or Right, and still attaches files
  with Enter.

## Capabilities

### Modified Capabilities

- `picker`: directory browser keys and rows; image browser opening keys.
- `tui`: one report per root change.

## Impact

- `src/tui.rs`, `README.md`.
