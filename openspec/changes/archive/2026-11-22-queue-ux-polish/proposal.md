# Proposal

## Why

A real session against the installed binary found six papercuts and one style
bug. The edit rule lost its save and move hints when the new actions were
added. A suggestion looks exactly like a draft and Enter silently does
nothing. An idle queue has no hint that ↑ opens it. The status path collapses
to single letters. `Ctrl+X` is invisible outside the editor, and deleting
gives no confirmation. The activity spinner inherits the italic of its row.

## What Changes

- The edit rule names save, steer or send, and delete, and fits 54 columns.
- A suggestion puts `→ accept` in the rule, and Enter fills the input without
  sending.
- An idle queue puts `↑ edit queue` in the rule.
- The status path keeps its last two folders whole before collapsing parents.
- The keys card lists `Ctrl+X`, and each deletion reports `queue: deleted n`.
- The spinner glyph stays upright; the rest of the thinking row stays italic.

## Keyboard and resources

No new keys. One more keys-card line, one info row per deletion.

## Capabilities

### Modified Capabilities

- `tui`: queue editing hints, suggestion affordance, status path, spinner style.
