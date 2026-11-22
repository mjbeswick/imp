# Design

## Approaches

- **Write then erase, plus synchronised output (chosen).** A small change to
  `frame()`; it removes the blank interval that kmscon shows.
- **Send only changed rows.** Fewest bytes, but needs the last frame kept and
  care around the overlay. Deferred until measured as needed.
- **Slow the countdown to every five seconds.** Hides the symptom and makes
  the timer coarse. Rejected.

## Frame

In `frame()`, every `ESC[2K` + text becomes text + `ESC[K`. Blank filler rows
become `ESC[K`. The status row keeps `ESC[J` after it to clear below. The
frame opens with `ESC[?2026h` and closes with `ESC[?2026l`, inside the
existing cursor-hide and wrap-off sequences.

## Size

Expected: under 100 bytes.
