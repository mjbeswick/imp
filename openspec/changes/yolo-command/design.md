# Design

## Approaches

1. **A session field and one toggle.** `Session` remembers the launch mode;
   `/yolo` flips between it and `Yolo`, and flips the path jail with it. Chosen:
   no state in the UI, and both front-ends share it.
2. **A UI-only override.** The UI would have to pass the mode on every turn and
   the worker would have to trust it.
3. **Write the config file.** Refused by design: the agent may not edit its own
   config, and a persisted yolo is easy to forget.

## Decisions

- `launch_permissions` is copied from `cfg.permissions` in `Session::new` and
  never changes, so toggling is symmetric.
- The toggle updates `tools.jail` through `perm::jail_enabled`, matching startup.
  Without that, the mode would say yolo while file tools still refused.
- The switch line is emitted by the session, not the UI, so line mode and the TUI
  print the same sentence.
- No confirmation prompt. The command is typed, the deny list holds, and asking
  twice makes the command two keystrokes every time.
- The command does not persist. The next launch returns to the configured mode.
