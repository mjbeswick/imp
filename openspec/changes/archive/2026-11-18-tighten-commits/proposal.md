# Proposal

## Why

Work was committed in batches, and the archive move at the end of the verification
change was left uncommitted entirely. Batching hides the one thing a commit is
for: a checkpoint you can return to.

## What Changes

`AGENTS.md`: commit after every ticked task, not per batch.

## Capabilities

None. Process only, `skip_specs: true`.
