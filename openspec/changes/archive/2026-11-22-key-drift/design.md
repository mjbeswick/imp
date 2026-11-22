# Design

## Approaches

- **Make the spec match the code (chosen).** Retry and a thinking key were
  never built; `/think` covers thinking. Ctrl+H is Backspace on most
  terminals, so it could never mean thinking.
- **Build Ctrl+R and Ctrl+T.** More keys for features nobody missed.

## Details

`Key::Ctrl('d')` gains an empty-draft guard. The palette row and its match
arm go. The tui requirement restores the Ctrl+Q and Ctrl+D wording that a
later change overwrote.

## Size

Expected: smaller.
