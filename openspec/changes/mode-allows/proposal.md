# Proposal

## Why

`[s] session` allows live only in memory and cannot be listed or revoked.
`[a] always` allows can be removed only by editing `allow = …` lines in the
config. The user wants to see and undo them without a new command.

## What Changes

- The `/mode` picker lists the three modes, then one row per allow, marked
  `session` or `always`.
- `d` on an allow row removes it: from the session's set, or its `allow`
  line from the config.
- Choosing a mode row works as now. Choosing an allow row does nothing.
- No new slash command; the count stays at twenty-three.

## Keyboard and resources

One letter, shown in the picker footer. With `mouse-and-copy`, the footer
hint is clickable too. The config is rewritten only when an `always` row is
deleted.

## Capabilities

### Modified Capabilities

- `permissions`: listing and revoking allows from `/mode`.
