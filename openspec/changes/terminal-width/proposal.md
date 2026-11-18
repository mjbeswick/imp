# Proposal

## Why

The model cannot see the terminal, so its tables, rules, and code lines are
drawn at whatever width it guesses and then mangled by the renderer. A live
reply wrapped a news list and a weather line at 80 columns in a 140 column
window, and a table would have broken outright.

## What Changes

- Publish the column count the renderer already measures to the session.
- Send one system line before the user message when the width changed since the
  last turn: `Environment: the terminal is 118 columns wide.`
- Send nothing when the width is unchanged, when input is piped, or in line
  mode without a terminal.
- Tell the model in the baseline prompt to keep tables, rules, and code inside
  that width, and to prefer lists over tables when narrow.

## Capabilities

### Modified Capabilities

- `tui`: the model is told the terminal width, once per change.

## Impact

- `src/tui.rs`: publish the width to the session.
- `src/agent.rs`: session field, environment note, prompt rule.
- `src/prompt.rs`: the width rule.
- No dependency, wire, or size change of note.
