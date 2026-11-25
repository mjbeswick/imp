# Tasks: picker width follows its content

## 1. Width rule

- [ ] 1.1 Test: `modal_width(200, 110)` is 110, `modal_width(200, 60)` is 72, `modal_width(200, 300)` is 120.
- [ ] 1.2 Test: `modal_width(100, 110)` is 90, and `modal_width(50, 0)` is 46.
- [ ] 1.3 Implement: `modal_width(cols, wanted)` in `src/tui.rs`, and update its two callers. The form passes 0.

## 2. Picker

- [ ] 2.1 Test: the `/keys` card at 160 columns shows its longest description without an ellipsis.
- [ ] 2.2 Implement: the picker overlay computes `wanted` from its rows and passes it to `modal_width`.

## 3. Validation

- [ ] 3.1 Run `cargo test` and `openspec validate --all --strict`, and record the outcome.
