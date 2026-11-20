# Proposal

## Why

`/sessions` filters titles only, so a session is hard to find by what was
said in it. It also does not show which live sessions are still working.
Separately, a session closed to make room leaves its background jobs running
with nothing tracking them.

## What Changes

- When the filter matches no title, Enter searches the text of every stored
  session and lists the hits with a snippet. Enter on a hit resumes it.
- A live session whose agent is working shows a spinner next to its count.
- Background jobs end when their session closes to make room. Exit is unchanged.
- No `/close`. The four-session pool and Ctrl+O cover it; still 21 commands.

## Capabilities

### Modified Capabilities

- `picker`: session search and the working spinner.
- `tools`: jobs end with their session.

## Impact

- `src/store.rs`, `src/tui.rs`, `src/jobs.rs`, `README.md`.
