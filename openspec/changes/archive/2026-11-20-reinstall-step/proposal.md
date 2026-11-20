# Proposal

## Why

The user runs `~/.local/bin/imp`, which stayed on an old build while new
features landed, so a working markdown table looked broken.

## What Changes

- `AGENTS.md` adds a step: after every committed change, reinstall with
  `install -m 755 target/release/imp ~/.local/bin/imp`.

## Impact

- `AGENTS.md` only; no behaviour change.
