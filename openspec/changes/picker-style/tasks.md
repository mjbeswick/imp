# Tasks

## 1. Frame

- [x] 1.1 Test: the picker draws no border characters, no rule under the title row, and no key-hint line; the title row holds the filter or the placeholder, with `esc` at its right. Confirm it fails before the change.
- [x] 1.2 Implement: draw the borderless frame and the title row in place of the edges, the rule, and `Picker::footer()` (`src/tui.rs`).

## 2. Rows

- [x] 2.1 Test: the current entry shows `■` in the gutter and keeps `current` in its right column; the highlighted row has the accent bar and spans the full inner width; other rows carry no marker.
- [x] 2.2 Implement: the gutter marker, the accent bar, and the full-width highlight in `picker_row` (replacing `▸`).

## 3. Title-row hints and clicks

- [x] 3.1 Test: row-action keys appear before `esc` in the title row; clicking `esc` closes the picker; the scroll position shows before `esc` on a long list.
- [x] 3.2 Implement: title-row hint text and its click targets, replacing the footer click targets (`src/tui.rs` 749 onward).

## 4. Browsers and filter

- [x] 4.1 Test: `/cd` and `/image` show their path as the placeholder while the filter is empty; typed text replaces it; Enter, Tab, Right, and Left behave as before.
- [x] 4.2 Implement the placeholder and cursor position in the title row.

## 5. Existing tests and validation

- [x] 5.1 Update tui tests that assert borders, the `▸` marker, or footer text, so each asserts the new frame.
- [x] 5.2 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [x] 5.3 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 5.4 Review the picker on screen in the Hackberry terminal or a screenshot, and record the result in design.md.
