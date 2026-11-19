# Proposal

## Why

`~/.local/state/imp/out` held 15 legacy spill files from the removed disk-spill
code and one job log. The seven-day rule does not touch recent legacy files,
and nothing reads those names any more.

## What Changes

- Delete every top-level `out-*` file at startup; they are unreachable.
- Keep `out/jobs/*.log` for seven days, because a resumed transcript may still
  name one.
- Cap the output directory at 64 MB, deleting the oldest job logs first.
- Document the retention.

## Capabilities

None. This is state maintenance with no behavior change the specs describe, so
the change uses `skip_specs: true`.

## Impact

- `src/tools.rs`: pruning, the size cap, and tests.
- `README.md`: retention note.
