# Proposal

## Why

The model reaches for `curl` through `bash` for JSON APIs, RSS feeds, and
devices on the local network, and every call asks for approval. The existing
`fetch` runs on DeepSeek's servers: it exists only on DeepSeek, returns web
pages as markdown, and cannot reach a local address such as `192.168.0.61`.

## What Changes

- A `fetch` tool that runs on this machine for every provider, through the
  libcurl imp already links: GET only, `http` and `https` only, 15 second
  timeout, bodies past 4 KB spill like other tool results.
- JSON, XML, RSS, and plain text come back as they are; HTML comes back as its
  title and readable text.
- The first fetch from a host asks once, `fetch from <host> for this session?
  y/N`; later fetches from that host do not ask. `yolo` never asks; `strict`
  asks every time.
- `search` keeps its `search` operation and drops `fetch`, so there is one way
  to fetch.
- The system prompt tells the model to use `fetch` for plain GETs and `bash`
  `curl` only for anything else.
- The binary size limit rises from 760,000 to 776,000 bytes, one 16 KB step,
  since the binary is 4 KB under the current one.

## Capabilities

### Modified Capabilities

- `tools`: the local `fetch` tool.
- `permissions`: one approval per host.

## Impact

- `src/agent.rs`, `src/tools.rs`, `src/perm.rs`, `src/prompt.rs`,
  `scripts/size.sh`, `README.md`, `AGENTS.md`.
