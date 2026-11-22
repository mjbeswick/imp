# Proposal

## Why

The README disagrees with the build. It says Ctrl+P switches sessions, claims
a 572 KB binary against 972 KB today, and says both "0 runtime dependencies"
and "needs libcurl". Two dead items, `Ev::Stop` and `push_diff`, warn on every
build.

## What Changes

- The "Several sessions" section names Ctrl+N for sessions and Ctrl+P for
  the palette.
- The headline numbers are measured again with the README's own method.
- "0 runtime dependencies" becomes "libcurl only", matching the install note.
- `Ev::Stop` and `push_diff` are deleted.

## Keyboard and resources

No keys change. Fewer bytes by the dead code.
