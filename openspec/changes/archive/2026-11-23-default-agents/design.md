# Design

## Approaches

- **Compiled-in agents, overridable by file (chosen).** Same pattern as the
  built-in skills: present on every machine, replaceable by the user.
- **Files written by `imp init`.** Missing on machines that never ran init,
  and stale after an upgrade.

## Read-only agents

`plan` and `review` get `tools: read, bash`, so `write` and `edit` are gone.
A mutating bash command still asks in `normal` and `strict`, as today. Agents
cannot change the permission mode, so no new mechanism is needed.

## Details

`assets/agents/{build,plan,review}.md` are included with `include_str!` and
parsed like agent files. `load_agents` adds each built-in whose name no file
took. `default` maps to `build` wherever a name is read.
