# Proposal

## Why

`imp -r` with no terminal and no prompt resumes the session, reads an empty
REPL to its end, and exits 0 without a word. It reads like a crash or a hang,
and the resumed session is never shown. The same happens to any command run
with stdin at end of file.

## What Changes

- A run with no prompt, no interactive terminal, and no input prints one line
  saying the chat needs a terminal and how to give it a prompt.
- The line goes to stderr, after the resume line, and the exit code stays 0.
- A real pipe that carries input is untouched; a command that acts is not
  told it did nothing.

## Keyboard and resources

None. One line on stderr, only when nothing happened.

## Capabilities

### Modified Capabilities

- `setup`: a run with nothing to do says why.
