# Proposal

## Why

A confirmation that nobody answers halts the turn for five minutes and then
tells the model the user declined. A live session lost 299,991 ms that way, the
model then believed it had been refused, and the user had no sign that anything
was waiting on them.

## What Changes

- Wait 60 seconds for an answer instead of 300.
- Show the seconds left inside the confirmation row.
- Ring the terminal bell when a prompt opens.
- On timeout, refuse and stop the turn, and say so in the transcript, rather
  than reporting a decline the user never gave.
- Let Ctrl+C refuse a pending prompt and stop the turn at once.

## Capabilities

### Modified Capabilities

- `tui`: a confirmation shows its deadline, asks for attention, and stops the
  turn when it goes unanswered.

## Impact

- `src/tui.rs`: confirmation wait, prompt row, attention, Ctrl+C while pending.
- `openspec/specs/tui/spec.md` confirmation requirement.
- No dependency, configuration, or wire changes.
