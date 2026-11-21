# Proposal

## Why

Six test conversations against DeepSeek showed four problems with getting
data online:

- Every new host asks once per session, so with nobody answering, a weather
  question failed after two refused fetches.
- A feed comes back as raw XML and is cut off, so the agent fetched it again
  with `curl | grep | sed`, which asked again.
- The prompt carries no date, so "the most recent F1 race" was searched as
  2025 in November 2026 and answered a year out of date.
- Each step shows one line, but a successful step shows nothing of what came
  back, and long URLs fill the row.

## What Changes

- `fetch` and plain `curl` GETs run without asking in `normal`. A URL over 300
  characters still asks, since a URL can carry data out. `strict` asks for
  every fetch; `yolo` never asks.
- The prompt states today's date and says to search, then open the best
  source, for anything that changes, and to say how current it is.
- HTML comes back as compact markdown: headings, links, lists, and tables,
  without navigation, footers, asides, and forms. RSS and Atom come back as a
  list of title, date, and link.
- Verbosity has three levels. `low` is one line per step; `medium`, the new
  default, adds a short result such as `→ 200 json · 12 KB`; `high` adds the
  arguments and output. `verbosity` can be set in the config. Fetch rows show
  the host and path instead of the full URL.

## Capabilities

### Modified Capabilities

- `permissions`: reading the web does not ask.
- `tools`: fetch returns markdown and feed lists.
- `tui`: three verbosity levels.
- `inbox`: the separate-approvals scenario uses a command.
- `instructions`: the prompt carries the date.

## Impact

- `src/perm.rs`, `src/agent.rs`, `src/prompt.rs`, `src/tools.rs`,
  `src/config.rs`, `src/tui.rs`, `src/main.rs`, `src/help.rs`, `README.md`.
