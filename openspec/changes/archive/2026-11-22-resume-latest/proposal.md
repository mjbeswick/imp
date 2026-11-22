# Proposal

## Why

`imp -r` alone fails with `--resume needs a value`. The user expects it to
resume the newest session, as `-c` does.

## What Changes

- `-r`/`--resume` with nothing after it, or with a flag next, resumes the
  newest session.
- A non-flag word after `-r` is still read as a session id, so
  `imp -r fix the parser` fails on the id instead of resuming the newest
  session and sending the rest as a message.
- Help shows `-r, --resume [ID]`. `-c` stays.

## Keyboard and resources

No new keys. No runtime cost.

## Capabilities

### Modified Capabilities

- `session`: `-r` without an id.
