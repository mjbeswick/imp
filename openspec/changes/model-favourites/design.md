# Design: model favourites

## Context

The model picker is built by `App::model_items` in `src/tui.rs` (around line 3962). Each row has the
value `provider::model`, and the rows come from the current model, the active provider's default, the
models the provider reported, and every usable provider's catalog. The row value is what Enter uses to
switch provider and model (around line 6146), so it is also the right key for a favourite.

Recent models are stored in one plain-text file at `dirs_state()/imp/models` (`src/tui.rs` around
line 2715). `load_models` and `record_model` (around lines 2111 and 2123) read and write it, one name per
line. Tests point `models_path` at a temporary file (`src/tui/tests.rs` around lines 3272 and 7106), so
tests never touch the real state directory.

Row actions are declared in `PickItem::actions` and run when the filter is empty (`src/tui.rs` around
line 5996). The provider picker uses `u`, `e`, `d`, and `n` this way. The model picker has no actions
today, so `f` is free.

The description column is rewritten on every tick by `mark_rows` (around line 1594). It strips only a
leading mark character from `is_mark` (around line 1578: `?`, `!`, `•`, and the spinner frames). A star
`★` is not a mark, so a description that begins with `★ ` survives repaint.

## Decisions

- **Storage.** A new file `favourite-models` next to `models`, derived with
  `self.models_path.with_file_name("favourite-models")`. Deriving it from `models_path` means tests that
  redirect `models_path` also redirect favourites, so no new field is needed and no test writes to the
  real state directory.
- **Format.** One `provider::model` value per line, in the order the user added them. The format matches
  the row value, so no translation is needed.
- **Reading.** `load_favourites` reads the file each time `model_items` runs. The file is tiny, and this
  avoids stale state after a toggle or after the picker rebuilds on `Ev::Models`.
- **Ordering.** `model_items` builds the rows as it does now, then moves favourites to the front with a
  stable partition. Favourites keep catalog order among themselves. A favourite whose provider is not
  usable is absent from the list, and the file keeps it, so it returns if the provider is re-enabled.
- **Marker.** The description gets `★ ` before the existing `current` or provider text. The star is
  included in `desc`, so the picker's description column stays aligned.
- **Toggle.** The `f` action on a model row adds or removes the row's value, writes the file, rebuilds
  `picker.items` with `model_items`, and keeps `picker.index` on the same value. It then toasts
  `favourite: <model>` or `unfavourite: <model>`.
- **Write failure.** A failed write pushes an error row, as `set_model` and the provider picker do,
  and leaves the picker unchanged.

## Risks

- A model's favourite flag is keyed by provider and model name. Renaming a provider orphans its
  favourites. Accepted: the same limit already applies to recent models and provider names.
- The `f` key is taken only when the filter is empty. A user who types `f` to filter the list still gets
  a filter. This matches the other row actions.

## Assumptions

- The star glyph `★` renders as a single cell in the terminals the TUI supports. If it does not, the
  description column shifts by one cell and the marker can be swapped for `*` in a follow-up.
- The `Ev::Models` rebuild happens while the picker is open, so the favourite list is re-read from the
  file there too. This needs no extra code.
