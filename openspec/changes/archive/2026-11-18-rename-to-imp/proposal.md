# Proposal

## Why

`mote` is a passive name for a tool that runs errands, and it collides with an
unrelated Python build tool. The name is baked into the binary, the environment
variables, the config and state directories, and the project-local directory.

`imp` is short, active, and says what the thing is: a small agent that lives in
the terminal and does work on request.

## What Changes

- Package and binary rename from `mote` to `imp`.
- **BREAKING**: environment variables rename from `MOTE_*` to `IMP_*`.
- **BREAKING**: config and state move to `~/.config/imp` and
  `~/.local/state/imp`.
- **BREAKING**: the project-local directory renames from `.mote/` to `.imp/`.
- Help, README, assets, and scripts follow the new name.
- No compatibility aliases. Pre-1.0, single user, no external installs.

## Capabilities

### Modified Capabilities

- `setup`: init creates the `imp` layout.
- `instructions`: the `imp prompt` subcommand, and `.imp/` discovery.
- `agents`: `.imp/agents/` discovery and `imp --agent`.
- `memory`: `.imp/memory/` project notes.
- `session`: `imp -c`, `imp --list`, `--rename`, `--delete`.
- `tui`: launch and help text.

## Impact

- `Cargo.toml`, `src/*`, `README.md`, `scripts/size.sh`,
  `assets/skills/config/SKILL.md`.
- Local migration, by hand: move `~/.config/mote` and `~/.local/state/mote`.
- Session count and history survive the move unchanged.
- Binary size and start time are unaffected.

## Out of Scope

- Version bump.
- `mote` shim or `MOTE_*` fallbacks.
- Renaming the repository directory. That is a final manual step after this
  change is archived.
