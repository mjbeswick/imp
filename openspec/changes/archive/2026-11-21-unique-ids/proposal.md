# Proposal

## Why

Session ids come from the current second, and `Log::create` never checks for
a clash. Two sessions started in the same second, by `/new` straight after
launch, a script, or two imp processes in tmux panes, share one log, so both
transcripts land in one file and each overwrites the other's title. `fork`
already checks.

Separately, the path jail resolves the deepest existing part of a path, and a
dangling symlink does not count as existing, so a write through one inside the
session directory creates its target outside.

## What Changes

- A new session claims its id by creating the sidecar exclusively, adding
  `-1`, `-2`, … on a clash. `fork` uses the same claim.
- The jail treats a dangling symlink as present and refuses it, since it
  cannot be resolved.

## Capabilities

### Modified Capabilities

- `session`: ids are unique.
- `permissions`: dangling links cannot leave the jail.

## Impact

- `src/store.rs`, `src/tools.rs`.
