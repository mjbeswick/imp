# Design

## Context

The picker frame is drawn in `src/tui.rs`. Citations are to `main` at `49c1b217`.

- The frame is built from an `edge` closure that sets a label and a right-hand text into a border line
  (`src/tui.rs` 7095–7111). The top edge uses it for the title and the `n/m` position (7126). A `│` wall
  wraps the filter row, which reads ` › <filter>` (7131–7133). A `├─┤` rule follows it (7134–7136).
  The bottom edge holds `picker.footer()` (7149).
- `Picker::footer()` builds the bottom hint line from the picker kind (`src/tui.rs` 2501–2525): `Enter choose`,
  `Tab/→ open`, `← up`, `Esc close`, or a settings or keys variant.
- Footer hints are click targets. `src/tui.rs` 749–750 documents that a hint replays its key when clicked.
  Those targets are built from the footer text, so removing the footer also removes them.
- `mark_current` (`src/tui.rs` 2263–2267) writes `current`, or `current · <about>`, into the right column.
  `picker_row` (2272) draws the marker `▸` for the cursor row (2274) and the right column for each row.
- `picker_lines` (2344) wraps and clips rows to `inner`. The row width logic is unaffected by the frame change.
- The current behaviour is specified in `openspec/specs/picker/spec.md`: bordered overlay, footer edge
  scenarios, `current` in the right column, and the `/cd` and `/image` title scenarios.

## Decisions

1. **Frame.** Drop the four corners, the side walls, the `├─┤` rule, and the bottom edge. The block keeps
   the existing width and height calculation, minus the two edge rows and the rule. Keep the existing
   height rule so the short-screen cut still applies.
2. **Title row.** One row holds the filter and the labels. The left side is the filter when it has text,
   and the picker title in muted text when empty. The cursor sits after the filter text, as now. The right
   side is `n/m` when the list scrolls, then `esc`. Browsers keep the path in the placeholder, so `cd ~/…`
   still shows where they are.
3. **Current marker.** Add a two-column gutter. The current entry shows `■`, and its right column keeps the
   `current` word. The `current` word is kept so the existing "One picker serves command values" scenarios
   (which say `current`) stay valid. Judgement call: the image shows no `current` word; keeping it costs a
   small deviation from the image, and removing it would force a change to the command-values requirement.
4. **Highlight.** The cursor row is painted across the full inner width with the existing selection role,
   and a `▎` accent bar sits in the gutter. This replaces the `▸` marker.
5. **Key hints.** Remove the bottom hint line. The title row shows `esc`. Row-action keys (session rename,
   and similar) sit before `esc`. Judgement call: `Enter choose` and `Tab/→ open` are no longer shown on
   screen. Cost if wrong: first-time users need to learn the keys; the mockup omits them.
6. **Clicks.** The keys in the title row are click targets, as the footer keys were: `esc` closes the picker,
   and a row-action key replays its key. Row clicks are unchanged. The old footer hints (`Enter choose`,
   `Tab/→ open`, `← up`) are no longer drawn and are no longer click targets.

## Risks and assumptions

- Assumption: `■` (U+25A0), `▎` (U+258E) and `▸` render in the terminals the user runs. The mockup shows
  `■`, so this holds for the user's terminal. The `▎` glyph is new in the picker, so it is checked in the
  visual test before archiving.
- Risk: the removed hints were the only on-screen cue for `Tab` and `←` in the browsers. The browsers keep
  their key behaviour; only the cue is removed.
- Risk: the eight-row cap stays, although the mockup shows about fifteen rows. This is out of scope.
- Assumption: the title placeholder is muted with the existing dim role; the filter text uses the normal
  text role.

## Expected bytes

Expected change: a few hundred bytes of code, roughly flat. `./scripts/size.sh` reports the measured figure.

## Panel background (added after the first apply)

7. **Panel.** The picker block gets a background. The theme table has no terminal background, and adding a
   per-theme slot would need eleven new colours. The panel is therefore derived from the theme's `Text`
   slot: a theme whose text is dark (the same luma test the theme tests use for light themes) gets a
   near-white panel, and a dark theme gets a near-black one. The mono theme gets no fill. Judgement call:
   the derived colours are not tuned per theme. Cost if wrong: a theme may look too pale or too dark; a
   per-theme slot fixes that later.

## Fixed height (added after the panel)

8. **Height.** The picker's height is a fixed number of item rows, set by the terminal height:
   `clamp(rows * 3 / 5 - 1, 5, 20)`, and never more than `rows - 4`. The window is no longer eight rows.
   The picker keeps this value on itself, set on each frame (the frame already takes `&mut self`), so the
   paging keys and the scroll follow use the same window the screen shows. Short lists pad with blank rows.
   A wrapped description takes a row from the bottom of the window, so the box never grows. Judgement call:
   the three-fifths figure and the bounds of five and twenty are chosen to match the reference image.
   Cost if wrong: the box may be taller or shorter than the user expects; the formula is one function.

## Padding (added after the fixed height)

9. **Padding.** The box loses its one-column left margin and gains one blank row above the title row. The
   box keeps its width: the inner width becomes the box width less one, and the one-column right margin
   remains. Judgement call: the top padding is painted with the panel background, so it reads as part of
   the box. Cost if wrong: the padding can be removed by dropping one line in the frame.

## Bottom padding (added after the top padding)

10. **Bottom padding.** One blank row below the last item row, so the box has padding above and below. The
    height becomes the window plus three: padding, title, items, padding. The padding row is painted with the
    panel like the top one. Judgement call: one row is the same measure as the top padding. Cost if wrong: one
    line in the frame.

## Gap under the input and right padding (added after the bottom padding)

11. **Gap and right edge.** "Space under input" is read as the blank row between the title row (the filter,
    the input of the picker) and the first item, matching the reference image. The one-column right margin
    goes: the content fills the full box width. The gap adds one row, so the height is the window plus four,
    and the window is capped at the rows less five so the box still fits. Judgement call: the reading of
    "under input" as the title-row gap rather than a gap below the box, which would sit above the composer.
    Cost if wrong: one row moves.
