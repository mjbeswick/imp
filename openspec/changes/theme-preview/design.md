# Design

## Approaches

1. **Swap the palette after every key while the theme picker is open.**
   Chosen: one check after key handling covers moving, paging, filtering, and
   every way of closing.
2. **Preview only on Up and Down.** Rejected: filtering also moves the
   highlight, and each close path would need its own revert.

## Decisions

- `Palette` keeps `allowed`, whether colour is on before `mono` turns it off,
  so `with_theme` can switch between `mono` and a colour theme both ways.
- `App::preview_from` holds the theme active when the preview began.
- After each key: if the theme picker is open, remember the starting theme
  once and paint with the highlighted one; otherwise, if a preview was running,
  restore the starting theme.
- Choosing with Enter saves the theme and clears `preview_from`, so the close
  does not restore.

## Tests

- Moving down repaints in the next theme and saves nothing.
- Esc restores the starting theme.
- Enter keeps the chosen theme after the picker closes.
