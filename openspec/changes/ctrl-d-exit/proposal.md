# Ctrl+D always exits

## Why

Ctrl+D exits only from an empty composer, and it keeps a draft otherwise. A user who presses Ctrl+D expects
imp to close, and the picker and draft rules make that unreliable. The user has asked for Ctrl+D to exit
from anywhere, so it works the same in every state.

## What Changes

- Ctrl+D exits at once, with or without a draft in the composer, and with a picker open.
- The draft-keeping rule for Ctrl+D is removed. **BREAKING** for a draft: text in the composer is discarded.
- Session delete moves from Ctrl+D to Ctrl+X in the session picker and in every picker's row actions, because
  an open picker can no longer use Ctrl+D. **BREAKING** for the pending `modal-shortcuts` mapping.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tui`: the keyboard-first requirement, for Ctrl+D.
- `picker`: the row-action key mapping (Ctrl+D becomes Ctrl+X). This delta is written after `modal-shortcuts`
  is archived, because its requirements do not exist in the main spec yet. See design.md.

## Out of Scope

- Ctrl+C behaviour. Its quit guard and draft rules stay.
- The confirmation prompts and the question prompt, beyond the rule that Ctrl+D exits from anywhere.
- Saving a draft before exit. The user chose to discard it.

## Impact

- Code: the Ctrl+D guard in `src/tui.rs` (around line 3739), the picker key routing, and the row-action table.
- Tests: `ctrl_d_keeps_a_draft` and the picker delete tests in `src/tui/tests.rs`.
- Specs: `openspec/specs/tui/spec.md` now, and `openspec/specs/picker/spec.md` after `modal-shortcuts` is archived.
