# Proposal: provider picker

## Why

The provider picker opened by `/provider` has two faults. First, the selected row
expands to two lines while the rest stay on one, so the list changes height as the
selection moves. Second, Enter on a provider row switches the active provider at
runtime instead of opening its editor in the modal. The code comment at the Enter
branch says Enter opens the form and `u` switches, so the behaviour contradicts the
intent.

## What Changes

- Every provider row keeps the same wrapped layout, whether selected or not, so the
  list does not change height when the selection moves.
- Enter on any provider row, built-in or custom, opens its editor in the modal.
- `u` still switches the active provider. `e` still opens the editor.

## Out of Scope

- The `/provider` command line forms (`add`, `edit`, `delete`, `use`).
- The model picker and every other picker.
- The provider profile fields and their validation.

## Impact

- `src/tui.rs`: `picker_lines` and the provider Enter branch.
- `src/tui/tests.rs`: tests for row height and Enter on `custom`.
