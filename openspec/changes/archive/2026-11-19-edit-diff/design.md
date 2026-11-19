# Design

## Approaches

1. **A detail field on the tool-end event, user only.** The agent builds the
   diff from arguments it already has, sends it beside the short result, and
   the model never sees it. Chosen: no extra tokens, no diff algorithm, and the
   rendering is a small addition.
2. **Put the diff in the tool result.** Zero new plumbing, but the model pays
   tokens to read back its own input on every edit.
3. **Re-read and diff the file after the write.** Produces a true unified diff,
   but needs an algorithm and hides which hunks the model intended.

## Decisions

- `Ui::ToolEnd` and `Ev::ToolEnd` gain `detail`, empty when there is none.
  `dispatch` fills it only for a successful `edit` or `write`.
- Edit detail comes from the old/new pairs: each old line prefixed `-`, each
  new line `+`, one blank line between pairs.
- Write detail is the written content as `+` lines. No old content is compared.
- The agent caps detail at 200 lines or 8 KB, whichever comes first, and notes
  the cut.
- The interface stores the full detail on the tool row and pushes a preview
  entry of at most 12 lines, ending with `… +N lines (Ctrl+O for all)`.
- Ctrl+O expands the last tool row, replacing the preview entry with the full
  detail as well as the arguments and output it already shows.
- Diff entries are painted line by line: `+` in the add colour, `-` in the
  delete colour, `@@` in the info colour, anything else dim. The prefix stays
  in the text, so colour-off output still reads as a diff.
- Two roles are added to the palette: add, a brighter green than the tool
  colour, and delete, the error red. The "at least six roles" rule still holds.
- Line mode prints the same detail under the row, with ANSI colour when stdout
  is a terminal and plain prefixes when it is not.
- A refused edit keeps the current error preview and prints no diff.
- Tests cover pair rendering, write additions, the 200-line agent cap, the
  12-line preview and its remainder note, Ctrl+O replacing the preview, colour
  mapping, colour-off prefixes, and line-mode output.
