# Design

## Approaches

1. **Animate the existing tool row.** Chosen: the action already has a row with
   a leading glyph, and the overlay-free transcript can update one entry in
   place.
2. **A spinner on the composer prompt.** Still detached from the action, and it
   competes with the caret.
3. **An overlay indicator.** The interface allows no second border or widget.

## Decisions

- `ToolRow` gains `running: bool`.

  - Start sets it true and writes the row with the current frame.
  - Finish sets it false and writes `✓` or `✗`.
  - The row stays after finishing so Ctrl+O can still expand it.

- `spinner_due` returns true only while the last tool row is running and 100 ms
  have passed. The tick advances `self.spinner` and rewrites that entry's text
  with the new frame. The turn's streaming state no longer drives the clock.
- The status line drops its leading spinner and its idle `·`. It starts at the
  model name, so it reads `deepseek-flash · 0 tok · $0.00`.
- Frames stay the existing eight-glyph set. The current frame is used at
  ToolStart so the row never flashes a different mark.
- Tests: the row's glyph changes across ticks, the status line contains no
  spinner glyph while streaming or running, and the finish still replaces the
  row with `✓`/`✗`.

## Tests

- Advance the clock past 100 ms during a running tool and assert the entry text
  changed to the next frame.
- Assert the status line has no glyph from the frame set.
- Finish the tool and assert the entry keeps the same index and starts with
  `✓`.
- Assert `spinner_due` is false when no tool runs, even while streaming.
