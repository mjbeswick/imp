# Proposal

## Why

`build-mote-v1` was archived with 35 tasks still open. All of them are
verification, plus two known gaps: the mid-loop context-overflow retry, and the
in-TUI session picker. Leaving them inside a finished change would hide them.

## What Changes

Tests, fixtures, and the two gaps — nothing user-facing that is not already
specified in `openspec/specs/`.

- Unit tests for prune and compaction decision points.
- Snapshot tests for every TUI state the spec describes.
- A recorded-fixture harness so the loop runs offline.
- Round-trip tests for the session log and sidecar.
- The two gaps: overflow retry, picker loading in place.

## Capabilities

None. Verification of existing behaviour, with `skip_specs: true`.

## Impact

- Only tests, `scripts/`, and the two behaviours above.
- No new tools, no config, no spec deltas.
