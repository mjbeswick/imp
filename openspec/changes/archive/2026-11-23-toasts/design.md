# Design

## Approaches

- **The status line hosts the toast (chosen).** Already on screen, already
  repainted alone, never covers text.
- **A floating box over the transcript.** Covers what the user is reading;
  more drawing on a software-rendered console.

## Details

`App::toast` stores the text and the time. The status line shows it ahead
of its usual content, after a mouse release note and a retry countdown,
which matter more. The idle loop wakes when a toast is due to expire and
repaints once. The five confirmations call `toast` instead of `push`.
