# Proposal

## Why

`/reload` re-reads instructions and skills but not the config, so a change
such as `allow_sudo=true` needs a restart. Unknown keys are ignored silently,
so `sudo = true` would do nothing and say nothing.

## What Changes

- `/reload` also re-reads the config file and applies each key whose value
  changed since it was last read, listing them: `allow_sudo: false → true`.
- Keys the file did not change keep their session value, so `/model`,
  Shift+Tab and `/think` choices survive a reload.
- A key removed from the file is named as needing a restart.
- Unknown keys are still ignored, but named once at startup and on reload.

## Keyboard and resources

No keys change. One file read per reload.
