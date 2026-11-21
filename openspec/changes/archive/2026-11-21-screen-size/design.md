# Design

The TUI stores the terminal rows in a global atomic at each frame, next to the
existing per-session width. The note reads both and tracks the pair it last
sent. A global suits the rows: there is one screen per process.

Agent commands read the same global and set `COLUMNS` and `LINES` when the size
is known. Typed `!` commands keep the user's environment untouched. Line mode
has no rows, so its note names only the width, as today.
