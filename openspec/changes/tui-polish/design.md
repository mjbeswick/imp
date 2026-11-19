# Design

## Approaches

1. **Polish inside the one-column layout.** Chosen. Rules, spacing, and
   alignment give structure without panes, and keep the frame testable as a
   string.
2. **Side panel for status and tools.** Rejected. It breaks the one-column
   requirement, halves width on laptops, and adds a layout engine.
3. **Adopt a TUI crate.** Rejected. The binary has no runtime dependencies and
   a 1.5 MB ceiling; the existing frame code already does what is needed.

## Decisions

### Resume

- `Session` gains `display_entries()`, returning `(Role, String)` pairs from
  the restored history: user text, assistant text, and one finished tool row
  per call using the stored summary. Reasoning and system messages are skipped.
- A new `Ev::Replay(Vec<(Role, String)>)` clears the transcript and pushes the
  entries. The worker sends it after `resume_id` succeeds, and `run` sends it
  once at start when the session already has history.
- After the replay the worker sends `Ev::Usage` with
  `api::estimate_tokens(&history)` and zero cost figures, so the status line is
  honest and cost still counts only this process.
- The session picker title becomes `sessions`.

### Chat column

- The composer is preceded by one rule row: `─` repeated to the width, in the
  thinking role. It is part of the composer block, so body height falls by one.
- `frame` pushes one empty row before every user entry except the first.
- When `entries` is empty, the bottom body row shows
  ` imp · {root} · / for commands` in the thinking role, with `root` shortened
  by `~` for the home directory and cut from the left when narrow.
- Below 8 rows the rule is dropped so the transcript keeps space.

### Status line

- Left: the existing `model · tok · $ · ↑ · think` string, unchanged.
- Right: the session root, `~`-shortened. It is drawn only when at least two
  spaces remain between the halves, so narrow terminals lose it first.

### Tool rows

- The duration leaves the summary text. `frame` draws it right-aligned on the
  row's last line in the thinking role, after at least two spaces.
- Under 1000 ms it reads `12ms`; at or above it reads `1.4s`.
- When the summary leaves no room, the duration takes its own row under the
  indent rather than being cut.

### Pending prompts

- The pending row is painted in the accent role with a `▌` prefix, and the
  rule above it is painted in the accent role while it is open. Still one row.

### Picker

- Corners `╭╮╰╯`. Top edge: `╭─ {title} ─…─ {n}/{m} ─╮`, the position shown only
  when the list scrolls. Bottom edge carries the footer text the same way.
- Row two is the filter: ` › {filter}`, with the cursor after it.
- A rule `├─…─┤` separates the filter from the rows.
- Width is `clamp(cols * 80 / 100, 40, 72)`, never wider than `cols`.
- Height becomes `visible + 4`, one row less than now.
- The selected row keeps the `▸` marker and paints across the full inner width.
- Rows with a description fit the right column first and cut the label with
  `…`. `PickItem` for sessions puts the title in `label` and
  `{id}  {age}  {n} msgs` in `desc`; `store::format_session` stays for
  `--list`.

## Risks

- Tests that match exact frames will need updates. Each task updates only the
  tests its requirement touches.
- Replay of a very long session renders many rows. The frame already slices to
  the visible window, so cost is the wrap of each entry, as it is today.

## Tests

- Replay: resume a fixture log and assert user, assistant, and tool rows appear
  in order with a non-zero token count.
- Layout: frame at 80x24 holds the rule above the composer, a blank row before
  the second user entry, and the welcome row when empty.
- Status: at 100 columns the root ends at the last column; at 40 it is absent
  and the model is whole.
- Tool rows: 5 ms reads `5ms`, 1400 ms reads `1.4s`, and the duration ends at
  the last column.
- Picker: corners, title in the top edge, footer in the bottom edge, width 72
  at 200 columns and 40 at 50, and a long session title cut while the message
  count stays whole.
- No colour: every new element renders with no escape sequences.
