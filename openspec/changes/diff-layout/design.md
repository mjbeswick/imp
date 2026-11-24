# Design: diff layout

## Comparable implementation

`hunks` in `src/linediff.rs` builds the inline diff rows for an `edit` or `write` result,
and `agent.rs` calls it where the tool row is drawn. The 12-line preview and Ctrl+E
expansion are in the TUI spec ("File changes show a diff"). The new layout adds a second
renderer beside `hunks` and does not change its output.

## Decisions

**Threshold: 120 columns.** Side by side needs two columns of at least 58 characters each,
plus a gutter. Judgement call: 120 columns is the usual wide-terminal width and gives each
side a readable width. Cost if wrong: a terminal between 100 and 120 columns shows inline
where side by side would fit; the constant is one line to change.

**Pairing.** Removed and added lines are paired in order within each hunk. An unpaired
removed or added line shows against an empty cell. Judgement call: pairing by order, not by
similarity, keeps the code small. Cost if wrong: a moved line appears as a remove and an add,
which is what inline shows too.

**Wrap.** A line longer than its column is cut with an ellipsis in side-by-side mode only.
Ctrl+E shows the complete inline diff in both modes, so no content is lost. Judgement call:
truncation only in the wide view.

**Model output unchanged.** The diff stays out of the tool result, as the spec requires.

**Size.** The renderer adds code, not assets. `scripts/size.sh` reports the growth.

## Validation

- Unit tests on the pairing function: equal hunks, uneven hunks, empty old text, empty new text.
- Render test at 119 columns (inline) and 120 columns (side by side).
- Render test: a long line is cut with an ellipsis in side-by-side mode only.
- `cargo test`, `openspec validate --all --strict`, `./scripts/size.sh`.
