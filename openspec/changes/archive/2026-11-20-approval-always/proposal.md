# Proposal

## Why

The same command asks again and again within a session: `npm test`, `cargo
build`, `make`. Each prompt is a `y` the user has already given.

## What Changes

- Command confirmations offer `y yes · n no · a always`.
- `a` approves the command's shape for the rest of the session: the same
  program and first non-flag argument, so `npm test` covers
  `npm test -- --watch` but not `npm install`, and `rm -rf build` covers only
  `rm -rf build`.
- `a` is not offered in `strict`, for sudo, for compound commands with `&&`,
  `;`, `|`, or redirects, or for writes and memory saves.
- The deny list still applies first. Approvals are per session and are
  forgotten on `/new` and on exit; nothing is saved.
- Fetch and plain `curl` GET approvals already last for the session per host,
  so their prompt is unchanged.

## Capabilities

### Modified Capabilities

- `permissions`: an always answer per command shape.
- `tui`: the confirmation prompt shows the answer keys.

## Impact

- `src/perm.rs`, `src/agent.rs`, `src/tui.rs`, `src/main.rs`, `README.md`.
