# Proposal

## Why

This repository uses OpenSpec, but nothing stopped source changes from landing
outside a change. Twice in this session code was edited before the artifacts
described it, which makes the specs a record of intent rather than a contract.

## What Changes

- An `AGENTS.md` rule: no source, config, or asset change outside an active change.
- `scripts/check-drift.py`: fails when code changed without a corresponding
  change to `openspec/changes/**`, or when code changed with no active change.
- `scripts/check.sh`: one gate running drift, `openspec validate --strict`,
  `cargo test`, and the binary size budget.
- The rule applies to this change too: tooling only, so `skip_specs: true`.

## Capabilities

None. Tooling and process only, with `skip_specs: true` recorded in
`.openspec.yaml`.

## Impact

- New: `AGENTS.md`, `scripts/check.sh`, `scripts/check-drift.py`.
- Changed: how work is committed; no runtime behaviour.
