# Design

## Approaches

1. **Check Ctrl+C before dispatching to the prompt.** One guard at the top of
   `handle_key`, excluding the command confirmation. Chosen: smallest change,
   and the confirmation keeps its own meaning.
2. **Add a Ctrl+C arm to every pending kind.** Five arms, each easy to forget,
   and the next prompt kind inherits the bug.
3. **Map Ctrl+C to Esc everywhere.** Esc means "dismiss", not "exit", and would
   still leave the session running.

## Decisions

- The guard runs before `self.pending.take()`, so the prompt is left as it is and
  the app quits.
- `Pending::Confirm` is excluded: its existing arm refuses the command and stops
  the turn, which is safer than quitting at the moment a destructive command is
  waiting on an answer.
- No new key. Ctrl+C means the same thing everywhere else in the interface.
- Tests cover a picker and an `ask` question, since both come from outside the
  composer.
