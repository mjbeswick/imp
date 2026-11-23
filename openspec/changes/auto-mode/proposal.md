# Proposal

## Why

`normal` asks about every command off a short safe list, so a build and test
loop prompts all day, and the only way out is `yolo`, which asks about
nothing, installs and pushes included. There is no step between them.

## What Changes

- A fourth mode, `auto`, runs without asking the development commands a
  project loop needs: builds, tests, linters, formatters, and local git. It
  asks about everything else, as `normal` does.
- Nothing else relaxes: the deny list, the path jail, the privilege setting,
  secret handling, and the file and `fetch` prompts are unchanged.
- Shift+Tab cycles `normal`, `auto`, `yolo`, `strict`: relaxing first, since
  strict is the rare choice. The status line and mode line name `auto`.

## Keyboard and resources

No new keys. One more stop on Shift+Tab. Classification is string matching
with no extra process or request.
