# Proposal

## Why

A read-only `find … -exec ls -l {} +` asks for confirmation because any
`-exec` clause is treated as arbitrary. The program it runs is on the safe
list, so the prompt is noise. The same shape rule is also too coarse: an
always on `find .` would silently permit `find . -exec rm` later.

## What Changes

- `find` with `-exec` or `-execdir` runs without a prompt when every program
  in every clause is safe and read-only, judged by the same pipeline rule.
- A writer, a shell, `-ok`, `-delete`, a print-to-file flag, or a missing
  clause terminator still asks.
- No `s` or `a` is offered for a command that runs another program through
  `-exec`, `-execdir`, or `xargs`, so one approval cannot arm a writer.
- `fd --exec` keeps asking; only `find` clauses are judged.

## Keyboard and resources

None. The classifier does the work.

## Capabilities

### Modified Capabilities

- `permissions`: which exec clauses run unasked, and where always is offered.
