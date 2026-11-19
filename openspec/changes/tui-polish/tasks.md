# Tasks

## 1. Resume

- [ ] 1.1 Add `display_entries`, `Ev::Replay`, and the post-resume usage event; send the replay from `-r`, `-c`, and the picker; retitle the picker `sessions`. Verify: a test resumes a fixture log and the frame shows its user, assistant, and tool rows in order with a non-zero token count.

## 2. Chat column

- [ ] 2.1 Draw the rule above the composer and the blank row before each later user turn; drop the rule below 8 rows. Verify: frame tests at 80x24 and 80x7.
- [ ] 2.2 Draw the welcome row while the transcript is empty. Verify: a frame test finds it when empty and not after the first message.
- [ ] 2.3 Add the right-aligned session root to the status line. Verify: frame tests at 100 and 40 columns.

## 3. Rows

- [ ] 3.1 Move tool durations to a dim right-aligned column with `ms` and `s` units. Verify: tests for 5 ms, 1400 ms, and a summary that fills the row.
- [ ] 3.2 Paint pending prompts and their rule in the accent role behind `▌`. Verify: a colour frame test and a no-colour test.

## 4. Picker

- [ ] 4.1 Redraw the overlay with rounded corners, edge title and footer, a filter row, and the 72-column cap. Verify: overlay tests for corners, edges, widths at 50 and 200 columns, and cursor position.
- [ ] 4.2 Keep the right column whole and cut labels; split session rows into title and detail. Verify: a long session title is cut and its message count is whole.

## 5. Finish

- [ ] 5.1 Update `README.md` where a sample changed and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
