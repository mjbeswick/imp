# Design: picker width follows its content

## Context

- `modal_width(cols)` (`src/tui.rs`, line 1382) returns `cols - 4` below 60 columns, and `cols * 80 / 100`
  clamped to 40..72 otherwise. It is called by the provider form overlay (line 7185) and the picker overlay
  (line 7252).
- The picker's description column is sized from the longest label, and its description column gets what
  is left (`desc_column`, `src/tui.rs`). A wider modal therefore gives descriptions more room with no other
  change.

## Decisions

- **The rule.** `modal_width(cols, wanted)` returns `min(max(wanted, 72), cap)`, where `cap` is
  `min(120, cols * 90 / 100)`. Below 60 columns the old narrow rule still applies, so `wanted` is ignored.
- **What the picker wants.** `wanted` is the widest row: the label's width, plus two columns for the
  marker, two for the gap, the description's width, and two for the frame.
- **The form keeps 72.** The provider form passes `wanted = 0`, so its width is unchanged.

## Risks

- A very long description on a wide terminal widens the modal to the cap, 120 columns. Wider than the
  current 72 on large screens, which is the intent.
- The 90% cap leaves a margin, so the modal never fills the screen edge to edge.

## Assumptions

- Terminals at 60 columns and wider can show a 72-column modal. This is true today for the 72 cap.
