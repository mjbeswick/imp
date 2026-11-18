# Agent Instructions

This repository uses OpenSpec. The specs are a contract, not a record of what
happened.

## The rule

Do not change source, config, or assets outside an OpenSpec change.

1. Before editing: `openspec list`. If no active change covers the work, create
   one — `openspec new change <name>`, then proposal, specs, design, tasks.
2. While working: tick the task in `openspec/changes/<change>/tasks.md` as you
   finish it. If the work reveals new scope, update the artifacts before the code.
3. Before committing: `scripts/check.sh`. It fails on drift, invalid specs,
   failing tests, or a binary over budget.
4. Commit per logical batch, with `jj commit`.

## No exceptions

- A typo in source is a change. Put it under an existing change if one covers it.
- Tooling and docs also need a change, with `skip_specs: true` when no behaviour
  changes.
- Do not mark a task done whose stated verification does not exist. Write the
  test, or leave the box empty.
