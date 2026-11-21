# Design

## Decisions

- **curl's progress callback.** curl calls it about once a second even when
  no bytes arrive, so one check covers streaming and stalls. A non-zero return
  aborts with `CURLE_ABORTED_BY_CALLBACK`, mapped to `cancelled`.
- **The flag is a parameter.** `Transport::post` takes the session's cancel
  flag. A thread-local would be hidden, and sessions are built on one thread
  and run on another.
- **curl's low-speed limit** (1 byte/s for 60 s) for stalls. Slow models that
  think for a while still send keep-alive bytes or tokens within a minute.
- **No retry on cancel.** The retry path checks the flag first.

## Keyboard

No change.

## Resources

None. No extra thread or timer.
