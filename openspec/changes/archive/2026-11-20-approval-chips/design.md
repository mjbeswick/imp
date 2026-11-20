# Design

## Decisions

- **Same shape key.** `allow` lines hold the same shape `perm::shape` makes
  for the session always, so `s` and `a` cover the same commands and differ
  only in how long.
- **Append, never rewrite.** `a` appends one `allow = <shape>` line to the
  config file, so comments and other keys are untouched. Undo is editing the
  file by hand; no `/allow` command until one is asked for.
- **Shared list.** `Config.allow` is an `Arc<Mutex<Vec<String>>>`, kept shared
  by `detached()`, so one session's `a` applies to every live session.
- **No file in tests.** `Config.file` is set only by `Config::load`; without
  it, `a` behaves as `s` plus the in-memory list.
- **Strict asks.** The shape is only computed in `normal`, so neither list
  applies in `strict`; `yolo` never asks anyway.
- **Chips.** The row is built from painted pieces: question in the accent,
  key letters bold accent, labels and countdown muted. Without colour it is
  the same text with no escapes. The question is cut first when narrow.

## Keyboard

One letter per answer. `s` and `a` sit beside each other on the home row of
the Hackberry's keyboard.

## Resources

One short list shared by all sessions. Expected cost about 3 KB of binary.
