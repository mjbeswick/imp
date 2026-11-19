# Agent Instructions

This repository uses OpenSpec. The specs are a contract, not a record of what
happened.

## Workflow

1. Discuss before building. Reply with the change name, a 3–5 bullet scope, and
   anything the request leaves open. Wait for a yes.
2. Then write the artifacts: proposal, specs, design, tasks.
3. Then code, ticking tasks as you go.
4. A small fix joins the active change as a new task. Do not open a second
   change for it, and do not code first and backfill the artifacts.
5. Tooling, docs, and process changes use `skip_specs: true`.

## The rule

Do not change source, config, or assets outside an OpenSpec change.

1. Before editing: `openspec list`. If no active change covers the work, create
   one — `openspec new change <name>`, then proposal, specs, design, tasks.
2. While working: tick the task in `openspec/changes/<change>/tasks.md` as you
   finish it. If the work reveals new scope, update the artifacts before the code.
3. Before committing, run the three checks by hand:

   ```sh
   cargo test
   openspec validate --all --strict
   ./scripts/size.sh      # binary under 776,000 bytes
   ```
4. Commit after every ticked task, with `jj commit`. One task, one commit.
   Not at the end of a batch: batching is how the archive move in this repo was
   briefly left uncommitted, and a commit you have not made is not a checkpoint.

## No exceptions

- A typo in source is a change. Put it under an existing change if one covers it.
- Tooling and docs also need a change, with `skip_specs: true` when no behaviour
  changes.
- Do not mark a task done whose stated verification does not exist. Write the
  test, or leave the box empty.
