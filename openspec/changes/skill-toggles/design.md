# Design: skill toggles

## Comparable implementation

The provider picker (`openspec/changes/provider-picker`) is the pattern for a picker
that changes a saved setting. The config's dotted keys (`provider.<name>.base`) are the
pattern for per-item settings. Skill discovery is `load_skills` in `src/store.rs`, and the
prompt index is built in `src/prompt.rs`.

## Decisions

**Config key: `skill.<name>.enabled = false`.** It follows the `provider.<name>.*` dotted
form. Judgement call: a boolean under the skill's name keeps room for other per-skill
settings later. Cost if wrong: a rename later touches the config parser only.

**Filter at load time.** `load_skills` drops disabled skills, so both the prompt index and
`skill_load` see the same list. This avoids a second check in the tool path. Judgement call:
one filter point rather than two. Cost if wrong: none expected; a disabled skill cannot be
loaded by any path.

**`/skills` picker.** It reuses the picker widget used by `/provider`. Space and Enter both
toggle, and the saved value is written through the existing config writer. Judgement call:
space and Enter both toggle, because the provider picker uses Enter for editing. Cost if
wrong: a key rebind later.

**Reload.** `/reload` already re-reads skills, so the toggle takes effect there. The picker
writes the config and then reloads, so the user sees the change at once.

**Size.** The change adds a small amount of code and no assets. `scripts/size.sh` will
report the measured growth, and the design target is under 2 KB of added binary.

## Validation

- Unit tests: `load_skills` omits a skill whose config flag is false, and includes it when
  the flag is absent or true.
- Config round-trip test for `skill.<name>.enabled`.
- Picker test: the `/skills` list shows each skill with its state, and toggling flips it.
- `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`.
