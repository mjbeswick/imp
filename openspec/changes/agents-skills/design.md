# Design

## Approaches

1. **Add both project directories and keep one loader per concept.** Chosen:
   `.agents` is a sibling of `.imp`, so each loader gains one directory and one
   precedence rule.
2. **Symlink `.agents` into `.imp` at init.** Couples the two conventions and
   breaks when both exist.
3. **Treat `.agents` as the only project source.** Breaks existing `.imp`
   setups.

## Decisions

- Skills remain last-wins. Directory order is global config, `.imp/skills`,
  `.agents/skills`, then an explicit `--skills-dir`, so the most specific
  source is processed last and replaces a duplicate name. This fixes the
  current inversion where global config wins over the project.
- For each skill entry, require `path.is_dir()` before looking for `SKILL.md`.
  That skips `.agents/skills/.openspec-target` and similar files without a
  special case.
- Agents remain first-wins. `agent_dirs` becomes extra, `.agents/agents`,
  `.imp/agents`, global config, so the first file with a name is kept. Only
  `.md` entries are considered, as today.
- The two loaders reach the same precedence — extra, `.agents`, `.imp`, global
  — through different insertion rules. That is deliberate: skills replace
  duplicates in place, agents keep the first seen.
- `/agent` parsing: no argument opens the picker; `current` asks the worker for
  a report; any other argument becomes `Cmd::SetAgent`. The worker already
  handles a successful switch and a missing name, so only the error text grows
  to list the roster.
- `describe_agent(&Agent)` in `agent.rs` formats name, model, thinking, and the
  tool subset. The worker and line mode share it, so the two front-ends cannot
  drift.
- Line mode extends its existing `/agent` arm: no argument still lists
  profiles, a name switches, and `current` prints the report.
- No command-count change: `/agent` is already one of the commands.
- Tests use temp directories for both sources and a duplicate name in each.
  Command tests assert the emitted `Cmd::SetAgent` and `Cmd::ShowAgent`. The
  description helper is tested directly.
