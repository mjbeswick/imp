# Design

## Approaches

1. **Anchor the view to a transcript row, keep a follow flag.** `App` stores
   `follow: bool` and `top: usize`. Following draws the last page; detached
   draws from `top` and clamps it to `total - body_rows`. Chosen: new output
   cannot move a detached view, and a resize only clamps the anchor.
2. **Keep an offset from the tail and grow it on every append.** Requires
   knowing the wrapped row count when output arrives, which depends on the
   terminal width, so it either over- or under-counts after a resize.
3. **Snapshot the rendered rows.** Correct but duplicates the transcript and
   costs memory per scroll.

## Decisions

- Scroll keys are PgUp and PgDn only. `decode` maps `CSI 5~` and `CSI 6~` to
  `Key::PageUp` and `Key::PageDown`; other `CSI n~` forms stay `Unknown`.
  Mouse reporting is out of scope: it adds terminal state to restore.
- One page is `body_rows - 1` rows, with a minimum of one. `frame` records the
  body height so key handling can page without measuring the terminal.
- `PgDn` that reaches the newest row sets `follow = true`. Editing the
  composer also sets it. End keeps its current meaning, the composer caret,
  because the spec's earlier "End resumes following" collides with it.
- Unseen rows are `total - (top + body_rows)`, shown as `↑ N` on the status
  line while detached. No count while following.
- Clearing the view with Ctrl+L resets the anchor and returns to following.
- Token formatting is a pure helper: below 1,000 exact; K at one decimal with
  a trailing `.0` dropped; M the same above 1,000,000. Tests cover 940, 1,000,
  3,400, 12,345, and 1,200,000.
- Cost accumulates in `Status`: each `Ev::Usage` adds its hit, miss, and output
  counts instead of replacing them. The status line prices the running totals
  and formats `$N.NN`. Unknown models still omit cost.
- Cache hit and miss counts remain internal because cost needs them; only the
  percentage and the permission mode leave the line.
- The session id for chat mode travels back from the worker as a shared string
  updated on `/new` and on a successful `/sessions` resume, because the worker
  owns `Session`. `tui::run` returns it once the loop ends.
- `main` prints `<id>` to stderr for every front-end after that front-end
  returns, including one-shot prompts. Stderr keeps a piped answer clean.
  Commands that open no session print nothing.
- Cost is scoped to the current process. A resumed session starts its total at
  zero because transcripts do not store per-turn usage. Recording it in the
  sidecar is a later change if the running total needs to survive a restart.
