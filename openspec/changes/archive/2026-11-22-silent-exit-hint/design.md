# Design

## Approaches

- **Explain after the REPL saw no input (chosen).** Only the genuinely silent
  case prints, so `echo "fix it" | imp` and `imp "fix it"` stay quiet.
- **Warn whenever stdin is not a tty.** Noisy for every pipeline, and wrong
  for the case that works.
- **Exit non-zero.** The run did what it was told; nothing failed. A wrong
  error code would break scripts.

## Details

`repl` counts the entries it reads. `main` turns that count plus the prompt
and tty flags into an optional one-line hint, printed to stderr after the
resume line. The decision is a pure function so it can be tested without a
process.

## Size

Expected: under 500 bytes.
