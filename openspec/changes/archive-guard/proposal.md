# Proposal

## Why

Two changes archived on 2026-11-22 both modified "The interface is keyboard
first". The second was written against the older text and silently removed
Ctrl+Q and the Ctrl+D guard from the main spec. Validation passed.

## What Changes

- `scripts/archive.sh <change>` wraps `openspec archive`.
- It refuses when a requirement the change modifies has changed in the main
  spec since the change was proposed, and names the requirement.
- With nothing stale it archives as before.

## Keyboard and resources

None; a developer script.
