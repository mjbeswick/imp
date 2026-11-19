# Design

## Approaches

1. **A synthetic first item.** Chosen: the browser already handles any row whose
   label ends in `/` as a directory, so `../` needs no new key path.
2. **A footer hint only.** Says the key exists but does not make the parent
   selectable without reaching for Left.
3. **A `..` entry from the filesystem.** `read_dir` never returns it, and on
   every platform it would need synthesising anyway.

## Decisions

- `browse_items` prepends `PickItem { label: "../", value: parent }` for `Dir`
  when `current.parent()` exists. The image branch is untouched.
- The value is the canonical parent path, so Enter and Tab reuse the existing
  directory handling: Enter sets `current` and reloads; Tab runs `cd <value>`.
- The row carries no description, so the two-column layout of command pickers
  does not pad directory rows.
- Sorting happens after the prepend, and the sort key would move `../`; the
  parent row is inserted after the sort instead.
- Tests: `../` is first at a non-root directory; Enter navigates up; Tab sends
  the parent as the new root; `/` has no row; `/image` has no row.

## Tests

- Browse a nested root and assert the first label is `../`.
- Enter on it lists the parent and keeps the session root.
- Tab on it sends `Cmd::SetRoot(parent)`.
- Browse `/` and assert no `../`.
- Open the image picker and assert no `../`.
