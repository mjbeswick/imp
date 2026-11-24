# Tasks: editor commands

## 1. Resolution

- [x] 1.1 Test: the editor comes from `editor` or `visual` config first, then `$EDITOR` or `$VISUAL`, then the OS opener for `/open` only. Confirm it fails first.
- [x] 1.2 Implement: the resolution function and the whitespace split into program and arguments.
- [x] 1.3 Test: a path with spaces stays one argument.

## 2. Commands

- [x] 2.1 Test: `/edit` with no target uses the current directory; with a target it uses that path.
- [x] 2.2 Implement: `/edit` as a foreground handoff, with the terminal restored on exit.
- [x] 2.3 Implement: `/open` as a detached launch.
- [x] 2.4 Add both commands to the help list.

## 3. Jail and new files

- [x] 3.1 Test: a target outside the session directory asks before launch; declining launches nothing.
- [x] 3.2 Test: a missing target asks, and only a yes creates the file.
- [x] 3.3 Implement: a Confirm picker, "No" first; a missing file is created only inside the session folder.

## 4. Validation

- [x] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Record the size.
- [x] 4.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
