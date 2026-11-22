# Proposal

## Why

The greyed suggestion ends with `  Tab`, which reads as part of the message.
Suggested questions often lack a closing `?`.

## What Changes

- The suggestion is drawn with no key hint. Tab and Right still take it.
- The suggestion request asks for a closing `?` on questions.
- A suggestion that starts with a question word and lacks `?` gets one; a
  trailing `.` on it is replaced.

## Keyboard and resources

No new keys. No extra requests.

## Capabilities

### Modified Capabilities

- `agent-loop`: question marks on suggestions.
- `tui`: no hint after the suggestion.
