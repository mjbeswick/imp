# Design

## Approaches

1. **A shared atomic mode.** Chosen: the worker reads it for every action, so a
   mode change does not wait for the command queue.
2. **Poll the command channel between tool calls.** Needs the turn loop to own
   the receiver and changes the worker's shape.
3. **Make `/yolo` a UI-only flag.** The worker does the classifying, so the
   gate has to see it.

## Decisions

- `Permissions` gains `code()` and `from_code()`, mapping the three variants to
  `u8`.
- `Session` holds `mode: Arc<AtomicU8>`, shared with the App. `Session::new`
  takes it as its last argument. `mode()` reads it.
- Every policy read uses `self.mode()`: `classify_command`, `classify_write`,
  `self_config`, and `shell_touches_self_config`.
- `Cmd::Yolo` becomes `Cmd::SetPermissions(Permissions)`. The App computes the
  target from the shared mode, stores it immediately, updates its own config
  copy, and sends the target. The worker sets `cfg.permissions`, `tools.jail`,
  and the atomic to the same value, so repeated or queued commands converge.
- The path jail still widens only when the worker handles the command, because
  `Tools` owns it. The confirmation gate, which is the reported problem, is
  immediate.
- Line mode keeps `toggle_yolo`, implemented as a read, compute, and
  `set_permissions`.
- Tests: a session whose atomic flips to yolo after construction runs a bash
  call without asking; the App stores the new mode while a turn is streaming;
  queued targets converge; the deny list still refuses in yolo.

## Tests

- Build a session with a shared mode, set the atomic to `YOLO`, and run a bash
  tool with a hook that records confirmations; assert none.
- Flip it back and assert a confirmation.
- In the App, run `/yolo` with `status.streaming = true` and assert the shared
  mode changed before any command was consumed.
- Existing toggle and deny-list tests keep passing.
