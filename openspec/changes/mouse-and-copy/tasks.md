# Tasks

## 1. Clicks

- [ ] 1.1 Decode a left press as a click; a test feeds `CSI <0;5;3M` and gets the cell
- [ ] 1.2 Record click targets per frame for picker rows, footer hints, confirm chips, and question options; tests click each
- [ ] 1.3 `always` needs a second click; tests for one click, two clicks, and the `a` key

## 2. Copy

- [ ] 2.1 Copy labels on replies and fenced blocks; tests for both, 40-column fit, and `mouse=off`
- [ ] 2.2 Copy buffer, Ctrl+V paste, and OSC 52 output; tests for paste, empty buffer, and the sequence
- [ ] 2.3 `/copy` and `/copy code`, added to help under Conversation; tests for reply, code, and nothing yet

## 3. Release

- [ ] 3.1 Ctrl+G releases the mouse until the next key; tests for release, take-back, and `mouse=off`

## 4. Docs

- [ ] 4.1 README: clicks, copy, Ctrl+G, Ctrl+V, and the tmux and Terminal.app notes for OSC 52
