# Design: editor commands

## Comparable implementation

The `foreground` shell handoff in `bash` (`src/agent.rs`, `src/tools.rs`) is the pattern for
giving the terminal to a child process and restoring the screen after it exits. Slash
commands are parsed in `run_command` in `src/tui.rs`. The file jail for `read` and `edit` is
in `src/tools.rs`.

## Decisions

**Two commands, not one.** A terminal editor must own the terminal, and a GUI editor must
not block the chat. Judgement call: `/edit` and `/open` keep these apart. Cost if wrong: a
user who expects one command learns two names.

**Editor resolution.** `editor` and `visual` config keys first, then `$EDITOR` and
`$VISUAL`, then, for `/open` only, the OS opener. Judgement call: config overrides the
environment, as `resolve_key_env` does for keys. Cost if wrong: none expected.

**Argument list.** The editor string is split on whitespace into a program and arguments,
and the target is appended as one argument. Judgement call: no quoting in the editor string;
a path with spaces stays one argument. Cost if wrong: an editor string with quoted arguments
would need a config change.

**Jail.** A target outside the session directory asks through the existing permission
prompt. Judgement call: reuse that prompt rather than a new one. Cost if wrong: none.

**Targets.** With no argument, `/edit` and `/open` use the current directory. With the
picker open, the selected entry is used. A path that does not exist is created as an empty
file only after the user confirms.

**Testing.** Unit tests cover resolution order, splitting, argument placement with spaces,
and the jail prompt decision. Launch itself is not tested.

## Validation

- `cargo test`, `openspec validate --all --strict`, `./scripts/size.sh`.
