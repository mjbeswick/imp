# Design

## Raw mode belongs to imp, not to `stty`

Today raw mode shells out to `stty`, and every restore path spawns a process
from a drop, a panic hook, or worse from a signal handler. The child inherits a
terminal that is already in the wrong mode, so a single failure leaves it
broken for the shell that started imp.

Replace it with `tcgetattr`/`tcsetattr` on file descriptor 0. imp keeps the
saved `termios`, sets `raw`-equivalent flags itself, and restores the saved
struct through one function that is safe to call more than once:

- `RawMode::drop` for the normal path.
- The panic hook, before printing the message.
- `SIGINT` and `SIGTERM` handlers, before writing the escape sequences.

The escape sequences stay as they are. Only the terminal-mode half moves off
`stty`.

## Line mode does not trust the terminal's Enter mapping

`stdin.read_line` waits for `\n`. Enter may arrive as `\r` when the CR-to-NL
mapping is off, which is exactly the state a previous imp run can leave behind.

Read line-mode entries by byte instead. An entry ends on either `\r` or `\n`,
and `\r\n` counts as one ending. End of file with no bytes returns `None`, so
the caller can exit. The reader stays generic over `BufRead`, so tests drive it
with a cursor and no terminal.

## Esc writes the session flag, not the queue

The worker thread is blocked inside `Session::turn`, so it cannot receive
`Cmd::Cancel` while a turn runs. Only the atomic flag reaches it.

`Session::cancel` is already a public `Arc<AtomicBool>`. Take a clone before
the session moves into the worker and store it on `App`. Esc, and Ctrl+C while
streaming, store `true` directly. `Cmd::Cancel` goes away.

Cancellation is checked at each step boundary, before each tool call, and by
the bash tool, so a cancelled turn stops after the current step rather than
mid-token. That is the behaviour the spec states.

## The restore test drives a real terminal

No assertion on `termios` is meaningful without a terminal. Add an integration
test that opens a pty with `openpty`, starts the built binary on the slave,
sends Ctrl+D to quit, and compares the slave's `termios` before and after. The
test fails against the current build, which is the point.

## Rejected

- Keep `stty` and only fix line mode: leaves the shell broken for every other
  program.
- Patch `ICRNL` alone at startup: hides the damage instead of fixing it, and
  still breaks if the process dies hard.
- Poll the command channel from inside the turn loop: more moving parts than
  setting the flag the turn already reads.
