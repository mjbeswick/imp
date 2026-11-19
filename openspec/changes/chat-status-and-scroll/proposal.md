# Proposal

## Why

The status line spends its width on signals nobody reads during a turn: the
cache hit share and the permission mode. Token counts print as five digits,
and cost prints six decimals. Meanwhile the transcript cannot be scrolled at
all: PgUp and PgDn decode to nothing, so old output is unreachable once it
leaves the pane. On exit, the session id, the only handle `imp -r` accepts, is
never shown.

## What Changes

- Remove the cache hit rate and the permission mode from the status line.
- Abbreviate context tokens with K and M, and render cost to two decimals from
  a running session total rather than the last response.
- Decode PgUp and PgDn (`CSI 5~` and `CSI 6~`), scroll a page at a time, hold
  the viewed rows while new output arrives, and show the count of unseen rows.
- Resume following the tail on PgDn at the newest row or on the next composer
  edit, and clear the count when following.
- Print the session id whenever the app exits with a session open, for chat,
  line mode, and a one-shot prompt, as `imp: resume with imp -r <id>` on
  stderr, after the terminal is restored.
- Update the key list in help and the README.

## Capabilities

### Modified Capabilities

- `tui`: the status line drops the cache share and permission mode, abbreviates
  tokens, and reports a two-decimal session cost; the transcript scrolls with
  PgUp and PgDn and reports unseen rows; every front-end prints its session id
  on exit.

## Impact

- `src/tui.rs`: `Status`, `App::status_line`, `App::frame`, `App::handle_key`,
  `Key`, `decode`, `control_sequence`, and `run` returning the final id.
- `src/main.rs`: exit path prints the session id for every front-end.
- `src/pricing.rs` and `tui::status_line`: cost accumulation across turns.
- `src/help.rs`: the key list.
- `README.md`: the keys table.
- `openspec/specs/tui/spec.md`: status, scroll, and exit requirements.
