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
- The bell is a flag on the app, set when a prompt opens and cleared by the
  first frame that carries it. A plain time window would ring on two or three
  frames at the redraw rate.
- Ctrl+C while a prompt is open refuses the prompt and stops the turn, matching
  the streaming case where Ctrl+C cancels.
- A refusal stays a tool result, so the model can continue with a safer plan.
- `yolo` moves below the deny list and the sudo gate in `classify_command`. The
  mode keeps its meaning, no prompts and no path jail, but `rm -rf /`, device
  writes, `curl | sh`, force pushes and hard resets still stop, and `sudo` still
  needs `allow_sudo`. Both rules are what the permissions spec already says, so
  the code was the side that was wrong.
