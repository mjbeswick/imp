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

## 6. Panel background

- [x] 6.1 Test: the picker's title row and item rows start with the panel background in a coloured theme, and the picker draws no fill in the mono theme. Confirm it fails before the change.
- [x] 6.2 Implement: derive the panel from the theme's text colour (near-white for light, near-black for dark) and wrap each picker line with it, re-applying it after each reset.
- [x] 6.3 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.

## 7. Fixed height

- [x] 7.1 Test: the box height is the same for three items and for forty in a 30-row terminal; the window is seventeen rows at 30 rows and twenty at 40 rows; a short screen cuts the highlighted description. Confirm the height test fails before the change.
- [x] 7.2 Implement: the item rows follow the terminal height, the list fills them and pads the rest with blank rows, and the window scrolls to keep the highlight visible (`src/tui.rs`).
- [x] 7.3 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.

## 8. Padding

- [x] 8.1 Test: the title row and item rows start at the box's left edge with no padding column, and a blank padding row sits above the title row; the box keeps its width and the filter cursor follows the new column. Confirm the layout test fails before the change.
- [x] 8.2 Implement: drop the left margin, add one blank row above the title row, and move the title, rows, cursor, and click targets to match (`src/tui.rs`).
- [x] 8.3 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.

## 9. Bottom padding

- [x] 9.1 Test: a blank padding row sits below the last item row, so the box is one row taller than before; the height test expects twenty rows for seventeen item rows. Confirm the bottom-padding test fails before the change.
- [x] 9.2 Implement: add one blank row after the item rows and count it in the height (`src/tui.rs`).
- [x] 9.3 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.

## 10. Gap under the input and no right padding

- [ ] 10.1 Test: one blank row sits between the title row and the first item row; the item rows and the title row end at the box's right edge with no right margin; the height test expects twenty-one rows and the bottom and top tests move with the gap. Confirm the gap test fails before the change.
- [ ] 10.2 Implement: add the gap row, drop the right margin, and move the item targets and the window fit to match (`src/tui.rs`). The window leaves room for the gap.
- [ ] 10.3 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
