# Tasks: picker width follows its content

## 1. Width rule

- [x] 1.1 Test: `modal_width(200, 110)` is 110, `modal_width(200, 60)` is 72, `modal_width(200, 300)` is 120.
- [x] 1.2 Test: `modal_width(100, 110)` is 90, and `modal_width(50, 0)` is 46.
- [x] 1.3 Implement: `modal_width(cols, wanted)` in `src/tui.rs`, and update its two callers. The form passes 0.

## 2. Picker

- [ ] 2.1 Test: the `/keys` card at 160 columns shows its longest description without an ellipsis. Not yet written; the width rule is tested directly.
- [x] 2.2 Implement: the picker overlay computes `wanted` from its rows and passes it to `modal_width`.

## 3. Validation

- [x] 3.1 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 852 unit tests pass; the two tests/tty.rs failures predate this change.
