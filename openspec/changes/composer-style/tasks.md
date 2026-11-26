# Tasks

## 1. Tests first

- [x] 1.1 Test: the composer block has the panel background, a left accent bar in the accent colour, a blank row above and below the input, and no `> ` glyph. Confirm it fails before the change.
- [x] 1.2 Test: the footer row reads the agent, mode, model, provider, and effort, with the agent and effort in the accent colour and the provider muted; the mode shows even when `normal`.
- [x] 1.3 Test: the status bar shows `262.2K (26%)`-style tokens and the cost, with no cache share, and the model name is absent from it.
- [x] 1.4 Test: a row above the composer reads `1 command` while one background job runs, and no row is drawn when none runs.
- [x] 1.5 Test: Ctrl+P opens the command menu with an empty filter; with text in the composer it keeps the text and opens nothing.
- [x] 1.6 Test: the command menu has no title row and no footer; Tab, Enter, and Esc behave as before.

## 2. Composer block

- [x] 2.1 Implement the block: panel fill, accent bar, blank rows, and the footer row in `composer` (`src/tui.rs`).
- [x] 2.2 Implement the glyph removal in `composer`, `paint_composer_row`, `prompt_agent`, and `prompt_width`, keeping the shell `!` draft and the caret.

## 3. Status bar

- [x] 3.1 Move the model, provider, mode, and effort into the footer row, and draw the status bar below the composer with the tokens and share, the cost, and the hint (`paint_status`, `status_line`).

## 4. Running commands row

- [x] 4.1 Count `Job::running` entries and draw the muted row above the composer while any runs (`src/jobs.rs`, `src/tui.rs`).

## 5. Ctrl+P

- [x] 5.1 Bind Ctrl+P to open the command menu with an empty filter when the composer is empty.

## 6. Command menu

- [x] 6.1 Draw the slash menu above the composer block at full width, with no title row and no footer; the highlight keeps the select background.

## 7. Existing tests and validation

- [x] 7.1 Update tests that assert the `> ` glyph, the status line text, the command picker title and footer, and the rule above the input.
- [x] 7.2 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [x] 7.3 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 7.4 Check the composer, status bar, and menu in a real terminal against the reference, and record the result in design.md.

## 8. Composer and status polish (specs of the design review)

- [x] 8.1 Test: the composer block's accent bar is drawn on every row of the block, the footer row is inside the block, and the agent name is capitalised. Confirm it fails before the change.
- [x] 8.2 Test: the input and the footer start in the same column; the agent name is in the accent colour, the effort in the warning colour, and the provider muted. Confirm it fails before the change.
- [x] 8.3 Test: the status bar is one row with the directory at the left, then the goal, tokens with share, cost, and `ctrl+p commands` at the right; no `/ for commands` hint appears in it. Confirm it fails before the change.
- [x] 8.4 Test: the command menu draws a muted rail at each row edge, has no blank row above the composer block, and draws the highlighted row's name and description in the accent colour. Confirm it fails before the change.
- [x] 8.5 Implement the block bar, footer placement, capitalised agent, column alignment, and footer colours in `composer` and `paint_composer_row` (`src/tui.rs`).
- [x] 8.6 Implement the one-row status bar with the directory at the left, and remove the slash hint from it (`paint_status`, `status_line`, the directory shortening).
- [x] 8.7 Implement the menu rails, the removed gap above the composer block, and the highlighted row colours in `command_menu`.
- [x] 8.8 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 8.9 Check the block, footer, status bar, and menu in a real terminal against the design, and record the result in design.md.

## 9. Menu box lines and left scroll indicator

- [x] 9.1 Test: each command menu row has a `│` box line at its left and right edge in the border colour, and the menu has no top or bottom edge. Confirm it fails before the change.
- [x] 9.2 Test: when the list scrolls, the scroll indicator is drawn over the left box line on the thumb rows only, and no indicator appears on the right edge. Confirm it fails before the change.
- [x] 9.3 Implement the box lines and the left scroll indicator in `command_menu` (`src/tui.rs`), replacing the rail glyph and the right-edge indicator.
- [ ] 9.4 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`, then reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 9.5 Check the menu's box lines and scroll indicator in a real terminal and record the result in design.md.
