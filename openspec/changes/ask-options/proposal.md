# Proposal

## Why

`ask` offers plain strings only: no way to explain a choice, no way to pick
several, and no shared convention for a recommendation. opencode's
`question` tool has all three. Its spec also says a dismissed question lets
the turn continue, while Esc now stops the turn.

## What Changes

- An option may be `{label, description}`; plain strings still work. The
  description is drawn muted after the label and wraps under it.
- `multiple: true` lets the user mark several options with their number
  keys and send them with Enter; the model receives them joined by `, `.
- The tool description asks the model to put a recommended option first and
  end its label with `(Recommended)`, and says a typed answer is always
  possible.
- The spec says dismissing a question stops the turn, as it now does.

## Keyboard and resources

Number keys toggle marks under `multiple`; Enter sends. One question per
call stays: a 54-column screen reads one at a time better.

## Capabilities

### Modified Capabilities

- `tools`: richer `ask` options and the dismissal rule.
