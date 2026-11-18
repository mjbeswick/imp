# Design

## Approaches

1. **A shared column count, injected per change.** The UI already measures the
   terminal each frame. Publish that number in an atomic the session reads when
   a turn starts, and add one system line only when it moved. Chosen: costs
   about 15 tokens, keeps the cached prompt prefix intact.
2. **Put the width in the baseline prompt.** Correct once, then wrong on every
   resize, and each rebuild invalidates the prompt cache for a full price turn.
3. **A `terminal_size` tool.** A round trip per table, and the model has to
   remember to call it.
4. **A process global.** No plumbing, but tests then share mutable state and
   flake under parallel runs.

## Decisions

- `Session` gains `cols: Arc<AtomicU16>` and `last_cols: Option<u16>`. The TUI
  clones its copy in; line mode passes 0 unless `COLUMNS` is set.
- At the top of `turn`, read the width. If it differs from `last_cols` and is
  above 20, add a system message before the user message and remember it.
- The note is one line and names only columns: `Environment: the terminal is
  118 columns wide. Keep code, tables, and rules inside it.`
- The baseline prompt gains one line under Limits saying the same thing, so a
  model that ignores the note still knows the rule exists.
- No rows. Rows do not affect content width and would cost a second line.
