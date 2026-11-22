# Design

## Approaches

- **Shorten in steps, last folder whole as long as possible (chosen).** The
  last folder is the one you recognise; the parents mostly say how deep.
  Step 2 is the fish shell's style, which many users already read.
- **Cut the front of the path with `…` only.** One step, but `…/src/providers`
  and `…/src/providers` in two projects look the same. Kept as step 3, after
  the abbreviation that still tells them apart.
- **Abbreviate only parents longer than some width.** A threshold to tune,
  and the result changes shape as the terminal resizes. Rejected.
- **Move the directory to its own row when it does not fit.** Costs a
  transcript row, which the Hackberry cannot spare. Rejected.

## Code

`fit_path(path: &str, room: usize) -> Option<String>` in `tui.rs` takes the
output of `tilde` and tries each step, returning the first whose width is at
most `room`, or `None`. Step 4 reuses `tail`. `status_line` passes
`cols - left - 2` as the room and leaves the directory out on `None`.

The leading `~` or `/` stays on step 2. A path with one part skips to
step 4. Widths count characters, as the rest of the status line does.

## Size

Expected: under 1 KB. Measured: `__text` +1032, `__cstring` +13.
