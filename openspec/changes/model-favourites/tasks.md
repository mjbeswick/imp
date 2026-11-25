# Tasks: model favourites

## 1. Storage

- [x] 1.1 Test: `load_favourites` returns an empty list when the file is missing, and the saved values in file order when it exists.
- [x] 1.2 Test: `toggle_favourite` adds a value that is absent and removes one that is present, and the file keeps one value per line.
- [x] 1.3 Implement: `load_favourites` and `toggle_favourite` beside `load_models` and `record_model` in `src/tui.rs`, deriving the path from `models_path`.

## 2. Ordering and marker

- [x] 2.1 Test: `model_items` lists favourite rows first, in catalog order, and leaves the other rows in their usual order.
- [x] 2.2 Test: a favourite row's description starts with `★ `, and a non-favourite row's description has no star.
- [x] 2.3 Test: a favourite whose provider is not usable is absent from `model_items`, and stays in the file.
- [x] 2.4 Implement: favourite partition and star marker in `App::model_items`.

## 3. Toggle key

- [x] 3.1 Test: pressing `f` on a model row with an empty filter adds the favourite, writes the file, and toasts `favourite: <model>`.
- [x] 3.2 Test: pressing `f` on a favourite row removes it and toasts `unfavourite: <model>`.
- [x] 3.3 Test: pressing `f` with a non-empty filter types into the filter and changes no favourite.
- [x] 3.4 Test: after a toggle the highlight stays on the same value.
- [x] 3.5 Implement: the `f` action in `PickItem::actions` for model rows, and its handler in the picker key match.

## 4. Help and docs

- [x] 4.1 Test: the help text for the `/model` picker names the `f` key. Check `src/help.rs` and `README.md` for the picker key list, and update them.
- [x] 4.2 Implement: help and README lines for the `f` key.

## 5. Validation

- [x] 5.1 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 818 unit tests pass. `tests/tty.rs` has two failures (`tui_pushes_and_pops_the_keyboard_protocol`, `tui_restores_the_terminal_on_exit`) that also fail on `main`, so they predate this change.
- [x] 5.2 Run `./scripts/size.sh` and note the size in the report. Result: 1039296 bytes, +16 since 1039280.
