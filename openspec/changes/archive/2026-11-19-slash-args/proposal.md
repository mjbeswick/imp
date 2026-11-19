# Proposal

## Why

The composer only treats a slash line as a command when it has no space. Every
command with a value (`/cd /tmp`, `/image shot.png`, `/agent reviewer`,
`/theme mono`, `/verbosity high`) is sent to the model as ordinary text, so
`/cd` appears not to work.

## What Changes

- Run every composer input that starts with `/` through the command parser.
- Keep the existing unknown-command suggestion for unrecognised names.
- Add a test that `/cd /tmp` reaches the command handler.

## Capabilities

None. This restores behaviour the command specs already describe, so the change
uses `skip_specs: true`.

## Impact

- `src/tui.rs`: one condition in `submit`, one test.
