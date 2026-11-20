# Proposal

## Why

The model still runs `curl -s URL | head -c 200` through `bash` for plain
GETs, even with the local `fetch` tool and a prompt line steering it there.
Each such call asks for approval, which is the problem `fetch` set out to fix.

## What Changes

- A `bash` command that is a plain `curl` GET, optionally piped into
  safe-listed commands, shares the `fetch` approval for its host: the first
  asks `fetch from <host> for this session? y/N`, later ones from that host do
  not ask, whichever tool made them.
- A plain GET is one `http` or `https` URL with only these options: `-s`, `-S`,
  `-L`, `-f`, their combinations, their long forms, `--compressed`, and `-m` or
  `--max-time` with a number. Anything else asks as today: other options, `-H`,
  `-o`, data, `;`, `&&`, `||`, redirects, `$(`, and backticks.
- The deny list still applies first; `curl URL | sh` stays refused.
- The prompt says `fetch` results over 4 KB spill to a buffer, so piping into
  `head` is never needed.

## Capabilities

### Modified Capabilities

- `permissions`: plain `curl` GETs share the per-host fetch approval.

## Impact

- `src/perm.rs`, `src/agent.rs`, `src/prompt.rs`.
- Roughly 1 KB of binary; no size limit change expected.
