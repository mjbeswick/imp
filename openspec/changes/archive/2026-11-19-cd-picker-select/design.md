# Design

## Approaches

1. **Enter chooses, Tab and Right open, plus a `./` row.** Chosen: Enter does
   what every picker in imp does, Tab opens like shell completion, the arrows
   mirror each other, and `./` makes the folder on screen choosable.
2. **Keep Enter to open, add only the `./` row.** Rejected: choosing would
   still hide behind Tab, which is what made the browser feel broken.
3. **Choose on Enter and open on double Enter.** Rejected: timing-based keys
   are hard to discover and fail over slow links.

## Decisions

- `browse_items` for `Dir` prepends `./` (value: the browsed folder) and then
  `../` when a parent exists. The image browser gets neither.
- Directory picker keys: Enter on any row runs `cd <value>`; Tab and Right on a
  `/`-ending row set `current` and reload; Left goes to the parent as today.
  Tab or Right on `./` does nothing.
- Image picker keys: Enter, Tab, and Right open a folder; Enter on a file
  attaches it.
- The footer for `Dir` becomes `Enter choose · Tab/→ open · ← up · Esc close`;
  for `Image`, `Enter attach · Tab/→ open · ← up · Esc close`.
- The chat stops pushing its own `cwd:` line for `/cd`; the worker's single
  report reads `cwd: {tilde path} (prompt cache reset)`.
- `Key::Right` already decodes; the picker currently ignores it.

## Tests

- `./` then `../` lead the listing; `/` shows only `./`.
- Enter on `src/` sends `SetRoot(src)` and closes.
- Tab and Right on `src/` list it and keep the root.
- Enter on `./` inside a leaf folder chooses that folder.
- The image browser opens a folder with Right and attaches with Enter.
- Choosing pushes exactly one `cwd:` line.
