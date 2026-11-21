# Proposal

## Why

`fetch` makes only plain GETs, so calling an API with a method, a body, or a
key sends the agent back to `curl` through `bash`: classified as a command,
summarised as a command, and with the key typed into the command line where
the model and the logs see it.

## What Changes

- `fetch` takes `method` (GET, POST, PUT, PATCH, DELETE), `headers`, and
  `body`. An object body is sent as JSON. The result adds `Location`,
  `Retry-After`, and rate-limit headers when present, and an error keeps its
  body.
- Credentials come from the config, bound to a host:
  `auth = api.github.com Authorization: Bearer $GITHUB_TOKEN`. imp adds the
  header only for that exact host, reads the variable when it sends, never
  follows a redirect with it, and treats the variable as a secret: stripped
  from the agent's commands and redacted from output.
- Approval in `normal`: a GET with credentials asks once per host per
  session; POST, PUT, PATCH, and DELETE ask every time, naming the method,
  host, path, body size, and whether credentials go with it. `strict` asks for
  every call; `yolo` never asks.

## Capabilities

### Modified Capabilities

- `tools`: fetch calls APIs.
- `permissions`: API calls ask by method and credentials.

## Impact

- `src/http.rs`, `src/agent.rs`, `src/config.rs`, `src/secrets.rs`,
  `src/main.rs`, `src/prompt.rs`, `README.md`.
