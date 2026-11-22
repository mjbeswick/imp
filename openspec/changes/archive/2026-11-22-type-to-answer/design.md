# Design

## Approaches

- **Any letter starts the answer (chosen).** Matches what people try first.
- **Keep `e`, make the hint louder.** Less code, but still a mode to learn.

## Details

In `AskMode::Choose`, a non-digit printable key moves to `AskMode::Custom`
with that key typed. `y`/`n` keep their shortcut only when the two choices are
Yes and No, ignoring case. An empty Enter in `Custom` leaves the row open.

The hint has no click target; clicking it does nothing, as for plain text.
An answer starting with a digit needs a letter first; accepted.

## Size

Expected: a few hundred bytes.
