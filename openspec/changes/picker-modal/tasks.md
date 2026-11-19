# Tasks

- [x] 1.1 Add the picker state and keys. Title, filter, items, highlight, offset, and kinds; type-to-filter, Up/Down, PgUp/PgDn, Enter, Tab, Left, Esc. Verify: filtering narrows and resets the highlight, paging moves eight rows at a time, and Esc returns the previous state.
- [x] 1.2 Render the overlay. Centre a bordered box sized 40 to 80% of the width, draw at most eight rows, and leave no border characters after close. Verify: a 50-column frame shows a 40-column box; a 40-item list scrolls; the next frame is byte-identical to one drawn before the picker opened.
- [x] 1.3 Wire the command pickers. `/agent`, `/sessions`, and `/verbosity` with no value open the shared picker and apply the chosen row; a typed value still applies directly. Verify: each command sends its existing action on Enter and nothing on Esc.
- [ ] 1.4 Add the directory browser for `/cd`. Enter navigates, Tab selects and closes, Left goes up but not past the root, and the root changes only after a successful selection. Verify: a fixture tree navigates, selects, and refuses the boundary.
- [ ] 1.5 Add the image browser for `/image`. List directories and the four image extensions, attach on Enter over a file, navigate on Enter over a directory. Verify: a `.txt` file never appears and a PNG attaches.
- [ ] 1.6 Update the dependent command scenarios and docs. Revise bare `/verbosity`, `/cd`, and `/image` in their specs now that they open pickers, and note the overlay in the README. Verify: `openspec validate --all --strict` and the help text agree.
- [ ] 1.7 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
