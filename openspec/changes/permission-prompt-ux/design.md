# Design

## Approaches

1. **Shorter wait, visible deadline, stop on timeout.** Sixty seconds, a
   countdown in the prompt row, a bell on open, and a timed-out prompt ends the
   turn instead of inventing a refusal. Chosen: it fixes the cost the transcript
   showed, 300 seconds of nothing, without changing how a deliberate answer
   behaves.
2. **Keep 300 seconds and only notify.** Cheap, but the turn still stalls and
   the model still reads a decline it never got.
3. **Derive answers from a configurable environment variable.** Adds a knob for
   a number nobody tunes; the default is what matters.

## Decisions

- The wait lives in `WorkerHooks::confirm`. On timeout it sets the shared cancel
  flag, sends a stop event with the reason, and returns false. Setting the flag
  before returning matters: the worker otherwise reaches its next request before
  the UI thread processes the event, and the turn runs on.
- Refusal by key keeps the turn running. Only an unanswered prompt stops it,
  because the user may have answered from another window or stepped away.
- The countdown is drawn from `pending_since`, so no timer thread is needed and
  the existing frame tick updates it.
- The bell rides on the prompt row for the first 400 ms of the prompt. That
  keeps it once per prompt without a separate write path, and it is testable
  from `frame()`.
- Ctrl+C while a prompt is open refuses the prompt and stops the turn, matching
  the streaming case where Ctrl+C cancels.
- A refusal stays a tool result, so the model can continue with a safer plan.
