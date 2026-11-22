# Tasks

## 1. Keys

- [x] 1.1 Decode Shift+arrows, Shift+Home/End, Delete, and Shift+Delete; tests for each sequence and for Ctrl+arrow staying plain
- [x] 1.2 Decode Home and End as tmux, screen, and rxvt send them (`CSI 1~ 4~ 7~ 8~`); test

## 2. Selection

- [x] 2.1 The anchor, Shift moves, and clearing on other keys; Delete with no selection; tests
- [x] 2.2 Backspace and Delete remove the selection; typing, newline, and Ctrl+V replace it; tests
- [x] 2.3 Draw the selection reversed across rows; tests at 54 columns and with colour off
