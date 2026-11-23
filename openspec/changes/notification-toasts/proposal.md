# Proposal

## Why

When a session out of view finishes a turn, asks a question, or fails, the
only signal is the bell. The user cannot tell which session wants them
without switching. Background job completions and scheduled wakes reach the
transcript of whichever session owns them, and nothing points at them while
the user works elsewhere.

## What Changes

- A background session that finishes a turn, asks a question, or fails shows
  a toast on the status line naming its position and title, for example
  `2/3 fix the parser: needs an answer`. The bell still rings.
- A finished background job and a scheduled wake (`⏰`) in a background
  session also show a toast.
- A toast uses the existing three-second status-line slot, and the newest
  one replaces any older one.
- Errors that need action stay in the transcript. Toasts are information.
- Line mode prints each notification as a plain line.
- The current session's own events are unchanged: they stay in the transcript.

## Keyboard and resources

No keys. A toast is one status-line row for three seconds.
