# Tasks

## 1. Tests first

- [x] 1.1 Test: the composer block has the panel background, a left accent bar in the accent colour, a blank row above and below the input, and no `> ` glyph. Confirm it fails before the change.
- [x] 1.2 Test: the footer row reads the agent, mode, model, provider, and effort, with the agent and effort in the accent colour and the provider muted; the mode shows even when `normal`.
- [ ] 1.3 Test: the status bar shows `262.2K (26%)`-style tokens and the cost, with no cache share, and the model name is absent from it.
- [ ] 1.4 Test: a row above the composer reads `1 command` while one background job runs, and no row is drawn when none runs.
- [ ] 1.5 Test: Ctrl+P opens the command menu with an empty filter; with text in the composer it keeps the text and opens nothing.
- [ ] 1.6 Test: the command menu has no title row and no footer; Tab, Enter, and Esc behave as before.

## 2. Composer block

- [ ] 2.1 Implement the block: panel fill, accent bar, blank rows, and the footer row in `composer` (`src/tui.rs`).
- [ ] 2.2 Implement the glyph removal in `composer`, `paint_composer_row`, `prompt_agent`, and `prompt_width`, keeping the shell `!` draft and the caret.

## 3. Status bar

- [ ] 3.1 Move the model, provider, mode, and effort into the footer row, and draw the status bar below the composer with the tokens and share, the cost, and the hint (`paint_status`, `status_line`).

## 4. Running commands row

- [ ] 4.1 Count `Job::running` entries and draw the muted row above the composer while any runs (`src/jobs.rs`, `src/tui.rs`).

## 5. Ctrl+P

- [ ] 5.1 Bind Ctrl+P to open the command menu with an empty filter when the composer is empty.

## 6. Command menu

- [ ] 6.1 Draw the slash menu above the composer block at full width, with no title row and no footer; the highlight keeps the select background.

## 7. Existing tests and validation

- [ ] 7.1 Update tests that assert the `> ` glyph, the status line text, the command picker title and footer, and the rule above the input.
- [ ] 7.2 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
- [ ] 7.3 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
- [ ] 7.4 Check the composer, status bar, and menu in a real terminal against the reference, and record the result in design.md.
