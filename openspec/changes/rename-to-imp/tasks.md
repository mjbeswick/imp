# Tasks

## 1. Name

- [ ] 1.1 Rename the package and binary in `Cargo.toml` from `mote` to `imp`. Verify: `cargo build --release` succeeds and `target/release/imp --version` prints `imp 0.1.0`.
- [ ] 1.2 Rename the environment variables from `MOTE_*` to `IMP_*` in `src/config.rs` and the help environment list. Verify: a unit test proves `IMP_MODEL` overrides the config file, and `grep -r MOTE_ src` finds nothing.
- [ ] 1.3 Rename the config and state directory components from `mote` to `imp` in `src/config.rs`, `src/store.rs`, `src/main.rs`, `src/tui.rs`, `src/init.rs`, and `src/perm.rs`. Verify: tests assert `~/.config/imp/config` and `~/.local/state/imp/sessions`, and the self-config guard test names `~/.config/imp/config`.
- [ ] 1.4 Rename the project-local directory from `.mote/` to `.imp/` in the instructions, agents, skills, and memory loaders. Verify: tests for `.imp/agents/reviewer.md`, `.imp/skills/`, and `.imp/memory/` pass, and the repository-marker check accepts `.imp`.

## 2. Surface

- [ ] 2.1 Update help examples, the environment section, the files section, and the `prompt` hint in `src/help.rs`. Verify: a snapshot test contains `imp` and no `mote`.
- [ ] 2.2 Update `README.md`, `assets/skills/config/SKILL.md`, and `scripts/size.sh` to the new name. Verify: `grep -rn mote README.md assets scripts` finds nothing outside `target/`.
- [ ] 2.3 Run the three checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` all pass.

## 3. Local

- [ ] 3.1 Move `~/.config/mote` to `~/.config/imp` and `~/.local/state/mote` to `~/.local/state/imp`. Verify: `imp -l` lists the existing sessions and `imp --version` prints the new name.
- [ ] 3.2 Install the binary as `~/.local/bin/imp`, remove any installed `mote`. Verify: `command -v imp` resolves and `command -v mote` fails.

## 4. Finish

- [ ] 4.1 Archive the change, then rename `~/Projects/mote` to `~/Projects/imp`. Verify: `jj log` works in `~/Projects/imp`, the bookmark still points at the archive commit, and `imp` runs from the new path. Run after the workspace is gone, because workspace git pointers are relative to the repo directory.
