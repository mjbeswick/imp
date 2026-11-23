# Proposal

## Why

Confirmations of what the user just did, such as `copied 42 characters`,
`thinking: on`, `theme: gruvbox-dark`, `verbosity: high` and `model: x`,
land in the transcript as accent rows. They pile up and push the
conversation away, and none of them is worth reading twice.

## What Changes

- Those confirmations show as a toast in the status line for three seconds,
  then the status line returns.
- Errors, warnings and anything the user must act on stay in the transcript.
- Line mode prints them as before; it has no status line.

## Keyboard and resources

No keys change. No floating box: on the Hackberry's small screen it would
cover the transcript. Expiry costs one status-line repaint.
