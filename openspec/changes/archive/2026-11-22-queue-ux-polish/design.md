# Design

## Approaches

- **Rule line carries the hints (chosen).** The rule is already the place
  that says what Enter does, and it costs no extra rows.
- **An arrow after the suggestion text.** Rejected: the spec deliberately
  keeps no key hint after a suggestion, and the rule can carry it instead.
- **A separate help footer for the queue.** Rejected: another row for one
  hint.
- **Splitting the thinking row for the spinner.** Chosen: the row becomes two
  spans, the glyph plain and the text italic, so the glyph never tilts.

## Details

The rule, when idle, prefers the suggestion hint, then the queue hint:
`→ accept` while a suggestion is shown, else `↑ edit queue` while messages
wait. While editing, it reads `editing n/m · Enter save · ^Q steer · ^X del`,
with `steer` becoming `send` when no turn runs.

`submit` with an empty draft takes a suggestion into the composer instead of
returning. It still does not send.

`fit_path` keeps the last two folders whole, cuts the earlier parents to one
character (two for dot folders), then collapses the parents to `…`, then cuts
the last folder from the front.

Deleting reports the 1-based index that went, matching `/queue delete`.

## Size

Expected: under 1 KB.
