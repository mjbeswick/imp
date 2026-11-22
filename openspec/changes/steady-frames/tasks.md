# Tasks

## 1. Steady frames

- [x] 1.1 Write each row, then erase to its end; tests that no row is preceded by `ESC[2K` and that rows still clear old text
- [x] 1.2 Wrap each frame in synchronised output; a test checks the frame opens and closes with it
- [x] 1.3 Write only changed rows, full repaint on resize and with an overlay; tests that a countdown tick writes one row and a resize writes all
- [ ] 1.4 Check on the Hackberry that a confirmation no longer flickers
