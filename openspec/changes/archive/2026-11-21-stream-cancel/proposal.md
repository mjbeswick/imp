# Proposal

## Why

The cancel key sets a flag that the loop reads only between steps. The HTTP
request keeps streaming, and costing tokens, until the model finishes. A
stalled connection, such as a Pi Zero dropping Wi-Fi, holds the session for
up to the 10-minute request timeout, and the next message waits behind it.
The spec already says cancellation is immediate.

## What Changes

- Cancel aborts the HTTP request within a second, while streaming or stalled.
- A cancelled request is not retried; the turn reports `cancelled`.
- A request that receives less than one byte a second for 60 seconds ends as
  a transport error, which is retried once as before.

## Capabilities

### Modified Capabilities

- `agent-loop`: cancellation reaches the request; stalled streams end.

## Impact

- `src/http.rs`, `src/transport.rs`, `src/agent.rs`.
