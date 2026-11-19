# Tasks

- [ ] 1.1 Add the theme module and state file. Four palettes, nine roles, load with fallback, save on change, and `mono` forcing colour off. Verify: each name round-trips through the state file, an unknown saved name yields `default`, and mono emits no escape bytes.
- [ ] 1.2 Apply themes in chat. Palette carries the theme, `/theme` switches live and lists on no argument, and an unknown name lists the choices. Verify: frame colours change per theme and the current name is marked.
- [ ] 1.3 Colour line mode. Paint the prompt, tool, error, and info rows with the palette; keep piped output and colour-off plain. Verify: a tty run contains the expected colour codes and a piped run contains no escape byte.
- [ ] 1.4 Fix line-mode rows and previews. Start and finish rows, `✓`/`✗` with duration, own-line failure preview, two-line or 240-character cap, and one blank line around blocks. Verify: a twenty-line result shows two lines plus a cut marker, and blocks are separated by exactly one blank line.
- [ ] 1.5 Remove `--chat`. Delete the parser arm and help line, make chat depend on the terminal only, and add an unknown-option test. Verify: `imp --chat` exits with an unknown option and a pty launch still opens chat.
- [ ] 1.6 Update `/help`, the README, and the command list after the other command changes land. Verify: `/theme` is documented and the command count matches.
- [ ] 1.7 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
