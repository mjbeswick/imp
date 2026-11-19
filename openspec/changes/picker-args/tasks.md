# Tasks

- [ ] 1.1 Let the directory browser climb above the session root. Stop only at `/`, and keep the image browser on its existing clamp. Verify: `Left` at the root lists the parent, `Left` at `/` does nothing, and Tab on an ancestor changes the root.
- [ ] 1.2 Open the value picker after a valid command word and space. Reuse the bare-command value builders for agent, model, theme, verbosity, provider verbs, and the empty `cd`/`image` arguments; leave free-text arguments in the composer. Verify: `/theme ` shows all themes, typing narrows them, and `/stash note` opens nothing.
- [ ] 1.3 Put focus in the filter and draw its cursor. Store the prefix, mirror `prefix + filter` into the composer, and place the terminal cursor on the filter row until the overlay closes. Verify: the frame carries the cursor sequence and Backspace edits the filter.
- [ ] 1.4 Show a description beside each command. Render the command list as two aligned columns, keep the name whole, clip the description, and filter on the name only. Verify: `/` shows `/compact` beside `compact the conversation now`, descriptions align, and `/co` does not match description words.
- [ ] 1.5 Update the README and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
