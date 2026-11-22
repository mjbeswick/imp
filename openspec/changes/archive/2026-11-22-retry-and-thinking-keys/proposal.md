# Proposal

## Why

The key spec promised Ctrl+R to retry and Ctrl+T for thinking. Neither was
built, and `key-drift` removed them from the spec. Both are wanted: a bad
reply needs a quick second try, and thinking is toggled often.

## What Changes

- `/retry` and Ctrl+R drop the last reply, with its tool calls, and send the
  last user message again.
- Ctrl+T toggles thinking for the next turn, as `/think` does.
- The keys card lists both; the palette shows `/retry · Ctrl+R` and
  `/think · Ctrl+T`.
- Twenty-six commands.

## Keyboard and resources

Two Ctrl letters, easy on the Hackberry. Ctrl+H stays Backspace. A retry
costs one more model request, as a resend would.
