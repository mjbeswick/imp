# Proposal

## Why

Running several agents means a terminal tab each and constant switching to
see which one finished or is waiting for a `y`. One imp should run several
sessions and show, in one place, which need the user.

## What Changes

- Up to four live sessions in one process. `/new` and resuming from
  `/sessions` open another live session instead of replacing the current one.
- Ctrl+N and Ctrl+P move to the next and previous session.
- An inbox, opened with Ctrl+O or `/inbox`, lists every live session with its
  state, `needs approval`, `replied`, `working`, or `idle`, and the last line.
  `y`, `n`, and `a` answer a pending approval in place; Enter opens the
  session; `x` closes an idle one.
- **BREAKING** Expanding the last tool output moves from Ctrl+O to Ctrl+E.
- Sessions out of view show a `2 waiting` count in the status line, ring the
  bell once when they need approval, and their confirmations do not time out.
- `?` on an empty composer opens a keys card; every overlay shows its keys in
  its bottom edge.
- Output buffers share one memory and spill budget across sessions.
- imp sessions only; other tools are out of scope.

## Capabilities

### New Capabilities

- `inbox`: live sessions, switching, the inbox, and attention.

### Modified Capabilities

- `tui`: Ctrl+E, `/inbox`, and the keys card.
- `tools`: one buffer budget across sessions.

## Impact

- `src/tui.rs` (split per-session state from the screen), `src/agent.rs`,
  `src/tools.rs`, `src/help.rs`, `README.md`.
- Depends on `permission-mode` and `approval-always`, which archive first.
