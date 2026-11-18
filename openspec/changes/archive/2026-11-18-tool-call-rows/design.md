# Design

## Approaches

1. **Summarise in the tool layer, draw one mutable row.** `describe(name, args)`
   next to the existing `summarise`, one row per call updated by `ToolEnd`, and
   Ctrl+O to expand. Chosen: both front-ends share the wording, and the row can
   be tested from `frame()`.
2. **Summarise in the UI.** The UI would need to know every tool's argument
   shape, duplicating the schema.
3. **Keep two rows and only clip harder.** Cheap, but the row still does not say
   what happened, and the duration stays wrong.

## Decisions

- One row per call. `Ev::ToolStart` pushes it and records the index;
  `Ev::ToolEnd` rewrites that entry with the tick, the duration, and the result
  on the same line. A second row only appears when the user expands.
- The summary comes from the parsed arguments, not the raw string: `bash` shows
  the command cut in the middle, `write` shows the path and the line count,
  `edit` the path and the count, `read` the paths, `search` the query or URL,
  `memory` the op and query, `ask` the question, `skill_load` the name.
- Timing moves into `dispatch`, around `run_tool` only, so a confirmation that
  took 30 seconds is not charged to a 3 ms command.
- Success comes from the tool's `Outcome`, not from the result text. A refusal
  carries its reason and renders red.
- Ctrl+O expands the last tool row into the full arguments and the full output.
  The 4 KB spill rule already keeps large output out of the transcript.
- The jail exception is `~/.config/imp` only: skills, agents, memory. The config
  file itself keeps its existing rule, which refuses agent edits in `normal` and
  asks in `yolo`.

## Open points

None. Colour follows the existing roles.
