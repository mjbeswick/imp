# Design

## Approaches

- **Rows in the `/mode` picker (chosen).** Reuses the picker and its
  per-row actions; no new command.
- **A `/permissions` command.** Clearer name, but a twenty-fourth command.
  Rejected by the user.

## Picker

Allow rows are `PickItem`s with `actions: [('d', "delete")]`, which the
footer already shows. The session set is the agent's `always` set; the
always set is `cfg.allow`. `d` sends a revoke command to the worker, which
removes the key and, for an always allow, rewrites the config without that
one `allow` line.

## Size

Expected: under 1 KB.
