# Design

## Context

Citations are to `main` at `uvxqvvls`, which holds the merged `composer-style` work.

- The quit guard is `Key::Ctrl('d') if self.input.is_empty() => self.quit = true` in `src/tui.rs` (around line
  3739). With text in the composer, Ctrl+D falls through and does nothing.
- The existing test `ctrl_d_keeps_a_draft` (`src/tui/tests.rs`, around line 1311) asserts the old draft rule.
  It is updated, not kept.
- The keyboard-first requirement in `openspec/specs/tui/spec.md` lists Ctrl+D and has the two Ctrl+D scenarios
  this change replaces.
- The pending `modal-shortcuts` change (applying now in its workspace) gives Ctrl+D the delete action inside
  open pickers: its task 1.4 and its footer example `delete ctrl+d`. Both conflict with this change.

## Decisions

1. **Exit at once.** Ctrl+D exits immediately, with or without a draft, with or without a picker, and with or
   without a confirmation open. The user chose this. The draft is discarded.
   Judgement call: "always" includes a running turn, so Ctrl+D quits mid-turn without the two-press guard that
   Ctrl+C has. Cost if wrong: a running turn is cut off; Ctrl+C remains the guarded way to stop.
2. **Picker precedence.** Ctrl+D is handled before the open picker's own keys, so it exits from a picker.
3. **Delete moves to Ctrl+X.** The session picker's delete action, and every row action that used Ctrl+D, uses
   Ctrl+X. Judgement call: Ctrl+X is free in the picker and the composer. Cost if wrong: the key is changed in one
   table.

## Sequencing

- The `tui` delta applies now, since the requirement exists in the main spec.
- The `picker` delta cannot be written until `modal-shortcuts` is archived. Its requirements
  ("Picker rows can carry shortcut actions" and "The modal footer shows shortcut keys") are only in that
  change's delta. Once it is archived, this change adds a MODIFIED delta for both, replacing `ctrl+d` with `ctrl+x`
  and the Ctrl+D scenarios with the new ones.
- The running `modal-shortcuts` apply implements the old Ctrl+D rule. Its result must be corrected after this
  change lands. Its workspace must not be merged with the old rule as the final state.

## Risks

- Risk: an accidental Ctrl+D discards a draft. The user accepted this.
- Risk: some terminals send Ctrl+D as end-of-input. The terminal check covers it.
- Assumption: Ctrl+X is not used by the terminal or by another binding. The key table test checks it.

## Expected bytes

The change removes a guard and adds a key rename. The size change is measured with `./scripts/size.sh` after
implementation, and no fixed figure is claimed here.
