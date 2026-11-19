# Proposal

## Why

OpenSpec installs its skills into `.agents/skills/`. This repository already
has six of them, and imp cannot see any: it only reads `.imp/skills`,
`--skills-dir`, and `~/.config/imp/skills`. The same set is available to
OpenCode but invisible to the agent working in the same directory.

The precedence also contradicts its own spec. `load_skills` reads project,
extra, then global and replaces on duplicate, so a global skill wins over a
project one, while `openspec/specs/skills/spec.md` requires the project one to
win.

## What Changes

- Read project skills from `.agents/skills/<name>/SKILL.md` and project agent
  profiles from `.agents/agents/<name>.md`, alongside `.imp/skills` and
  `.imp/agents`.
- Precedence, most specific first: an explicit `--skills-dir`, then `.agents`,
  then `.imp`, then the global config. A later duplicate still loses to an
  earlier, more specific one.
- Ignore non-directory entries such as `.agents/skills/.openspec-target`, and
  non-markdown entries under `.agents/agents`.
- Extend `/agent`: no argument keeps the picker, a name switches directly,
  `current` reports the active profile, and an unknown name lists the choices.
  The command count does not change.
- Keep `AGENTS.md` loading exactly as it is: global, repository root, session
  directory.
- Document the new sources and the precedence in the README.

## Capabilities

### Modified Capabilities

- `skills`: adds `.agents/skills` as a project source and states the
  precedence order.
- `agents`: adds `.agents/agents` as a project source for profiles, and extends
  `/agent` with a direct name and a `current` report.

## Impact

- `src/store.rs`: `load_skills`, `agent_dirs`, and the directory check.
- `openspec/specs/skills/spec.md` and `openspec/specs/agents/spec.md`.
- `README.md`: the files it reads and the shadowing rule.
