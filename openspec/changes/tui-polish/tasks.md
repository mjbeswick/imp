# Tasks

## 0. Line mode bugs

- [x] 0.1 Run `!cmd` locally in line mode. Verify: a line-mode test with a fake provider sends no request for `!echo hi` and prints its output.
- [x] 0.2 Put a line-mode failure preview on its own indented line with no trailing space. Verify: a test for `!false`-style tool failure output.

## 1. Resume

- [x] 1.1 Add `display_entries`, `Ev::Replay`, and the post-resume usage event; send the replay from `-r`, `-c`, and the picker; retitle the picker `sessions`. Verify: a test resumes a fixture log and the frame shows its user, assistant, and tool rows in order with a non-zero token count.

## 2. Chat column

- [ ] 2.1 Draw the rule above the composer and the blank row before each later user turn; drop the rule below 8 rows. Verify: frame tests at 80x24 and 80x7.
- [ ] 2.2 Draw the welcome row while the transcript is empty. Verify: a frame test finds it when empty and not after the first message.
- [ ] 2.3 Add the right-aligned session root to the status line. Verify: frame tests at 100 and 40 columns.

## 3. Rows

- [ ] 3.1 Move tool durations to a dim right-aligned column with `ms` and `s` units. Verify: tests for 5 ms, 1400 ms, and a summary that fills the row.
- [ ] 3.2 Paint pending prompts and their rule in the accent role behind `▌`. Verify: a colour frame test and a no-colour test.

## 4. Picker

- [ ] 4.1 Redraw the overlay with rounded corners, edge title and footer, a filter row, and the 72-column cap. Verify: overlay tests for corners, edges, widths at 50 and 200 columns, and cursor position.
- [ ] 4.2 Keep the right column whole and cut labels. Verify: a long session title is cut and its message count is whole.

## 5. Modals

- [ ] 5.1 Open the picker for bare `/theme` and `/provider`, and close a value picker when its filter gains a space. Verify: tests for both pickers and for `/provider add work` reaching the form.
- [ ] 5.2 Use `current` as the only current marker and show agent descriptions. Verify: model, theme, and agent picker tests.
- [ ] 5.3 Show the browsed directory in `/cd` and `/image` titles. Verify: a test after opening a subdirectory.
- [ ] 5.4 Keep the session picker behind the delete confirmation and name the title. Verify: a frame test with both visible.
- [ ] 5.5 Draw the composer caret in text prompts. Verify: a `/rename` frame test finds no trailing `_`.
- [ ] 5.6 Add `session_row` and use it in `--list`, line-mode `/sessions`, and the picker; cap line mode at twenty. Verify: alignment, newline title, and untitled tests.

## 6. Finish

- [ ] 6.1 Update `README.md` where a sample changed and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
