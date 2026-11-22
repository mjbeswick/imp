# Design

## Approaches

- **Write then erase, plus synchronised output (chosen).** A small change to
  `frame()`; it removes the blank interval that kmscon shows.
- **Send only changed rows (chosen as well).** The user asked that only what
  changed is drawn. Keeps the last frame's rows; fewest bytes on the Pi.
- **Slow the countdown to every five seconds.** Hides the symptom and makes
  the timer coarse. Rejected.

## Frame

In `frame()`, every `ESC[2K` + text becomes text + `ESC[K`. Blank filler rows
become `ESC[K`. The status row keeps `ESC[J` after it to clear below. The
frame opens with `ESC[?2026h` and closes with `ESC[?2026l`, inside the
existing cursor-hide and wrap-off sequences.

## Changed rows

`frame()` builds one painted string per screen row. `App.shown: Vec<String>`
holds the last frame's rows. Each row that differs is written as
`ESC[<row>;1H` + row + `ESC[K`; equal rows are skipped. The cursor and the
overlay are drawn after the rows as now.

ponytail: while a picker overlay is open, or the size changed, every row is
written, since the overlay paints over rows with cursor moves. Compositing
the overlay into the row strings would lift that ceiling if pickers flicker.

## Size

Expected: under 1 KB.
