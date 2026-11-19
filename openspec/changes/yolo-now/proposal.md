# Proposal

## Why

`/yolo` is sent to the worker as a command, but the worker is inside the turn
and does not read commands until the turn ends. A user who enables yolo right
after sending a prompt still gets every confirmation for that turn, which reads
as yolo not working.

## What Changes

- The permission mode becomes shared state that the worker reads before every
  tool action, not the value captured at turn start.
- `/yolo` flips the shared mode immediately, then asks the worker to sync its
  config and path jail as soon as it can.
- A typed command carries the exact target mode, so queued commands cannot
  toggle twice.
- Line mode keeps its direct toggle and reports the same messages.

## Capabilities

### Modified Capabilities

- `tools`: the permission gate reads the live mode for each action.
- `tui`: `/yolo` takes effect on the current turn's later calls.

## Impact

- `src/perm.rs`: mode encoding.
- `src/agent.rs`: shared mode, per-action read, exact target command.
- `src/tui.rs`, `src/main.rs`: shared mode wiring and `/yolo`.
- `openspec/specs/tools/spec.md`, `openspec/specs/tui/spec.md`.
