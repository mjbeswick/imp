# Tasks: diff layout

## 1. Pairing

- [ ] 1.1 Test: pairing gives equal hunks, uneven hunks, empty old text, and empty new text correctly. Confirm it fails first.
- [ ] 1.2 Implement: the pairing function in `src/linediff.rs`.

## 2. Rendering

- [ ] 2.1 Test: at 119 columns the diff is inline; at 120 columns it is side by side.
- [ ] 2.2 Implement: the side-by-side renderer and the width switch.
- [ ] 2.3 Test: a long line is cut with an ellipsis in side-by-side mode only.
- [ ] 2.4 Test: the tool result sent to the model is unchanged in both layouts.

## 3. Validation

- [ ] 3.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Record the measured size.
- [ ] 3.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
