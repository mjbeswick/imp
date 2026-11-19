# Proposal

## Why

`--cwd` chooses the session root only at launch. Once the interface is open,
nothing shows that root, and nothing changes it. A user who opens imp in the
wrong directory has to quit and start again, and a resumed session always lands
where it is launched rather than where it was working.

## What Changes

- Add `/cwd`, which prints the session root.
- Add `/cd <path>`, which expands a leading `~`, resolves a relative path
  against the current root, and refuses a missing path or a file.
- Moving the root moves the shell directory, typed `!` commands, file tools,
  the path jail, `AGENTS.md`, skills, memory, and the system prompt together.
  The conversation is kept and the prompt cache resets.
- Record the root in the session sidecar and restore it on `-c` and `-r`, with
  an explicit `--cwd` taking precedence.
- Update `/help`, `--help`, the README, and the specs.

## Capabilities

### Modified Capabilities

- `tui`: two more slash commands, and a requirement covering inspection and
  changes of the session root.
- `tools`: the shell root can come from `/cd` as well as `--cwd` and launch.
- `session`: a resumed session restores the start directory recorded in its
  sidecar unless `--cwd` overrides it.

## Impact

- `src/config.rs` or `src/main.rs`: one shared path resolver with `~` expansion.
- `src/agent.rs`: `Session::set_root` reloads instructions and updates the
  sidecar.
- `src/store.rs`: update a sidecar's `cwd` without rewriting the transcript.
- `src/tui.rs`: `/cwd` and `/cd`, a `Cmd::SetRoot`, and the App's shell root.
- `src/main.rs`: line mode, and root precedence on resume.
- `src/help.rs` and `README.md`.
