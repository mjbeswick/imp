# Design

## Approaches

1. **Read the served model from the stream.** DeepSeek sends a `model` field on
   response chunks; parse it once per turn and keep the latest value. Chosen:
   the line then reflects what actually answered, with no table to maintain.
2. **Probe a models endpoint at launch.** Costs a request before the first turn
   and still may not name the snapshot that serves it.
3. **Keep a local name-to-version table.** Goes stale and duplicates what the
   API already reports.

For verbosity:

1. **A session field threaded into rendering.** `App` and the line-mode printer
   read one level; low is the default. Chosen: no config write, and the level
   can change mid-session.
2. **A config key.** Persists across sessions, which the request did not ask
   for, and needs a reload to change.
3. **A per-call tool flag.** Puts presentation in the model's hands.

## Decisions

- `api::Accum` gains `model: Option<String>`, filled from the first non-empty
  `model` field in a chunk. `Session` keeps it beside `last_usage`; the status
  line shows `name (served)` only when the two differ and the served value is
  known.
- `tool_row_text` puts a failure preview after `\n`, matching the row's
  two-space indent. The painter wraps the whole row at the terminal width with
  the same continuation indent, so no line escapes the column.
- Bold marks are applied after colour: paint the row, then wrap the leading
  mark in `\x1b[1m` and `\x1b[22m`. The row's colour survives because only the
  weight is reset. With colour off, the bold sequence is omitted too.
- `/verbosity` is parsed in `run_command`. `high` sets `App::verbose`; `low`
  clears it; a bare command reports the level; anything else lists the two
  valid levels. Existing rows are not redrawn.
- In the full-screen interface, `verbose` makes `tool_row_text` include the
  arguments and the capped output on indented lines, the same content Ctrl+O
  shows for one call. Ctrl+O still toggles the last call.
- In line mode, `print_ui` carries the same flag. `high` prints the arguments
  under the start line and the preview under the end line; `low` keeps today's
  output.
- `--cwd` is parsed before the config is loaded. The path is resolved against
  the launch directory, canonicalised, and rejected with `not a directory` when
  it is missing. That value becomes the session root, so the path jail,
  `AGENTS.md` lookup, memory, and `bash` all move with it.
- `tools::bash` sets `current_dir(&self.root)` explicitly. Typed `!` commands go
  through the same tool, so they inherit it.
- The redraw rework stays in `render-on-change`; this change builds on it and
  does not touch the render loop.
