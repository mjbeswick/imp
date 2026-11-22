# Proposal

## Why

The key specs and the code disagree. Ctrl+D quits with a draft and loses it.
The palette's thinking row replays Ctrl+T, which nothing handles. The tui key
requirement still lists Ctrl+R, Ctrl+T and Ctrl+H, which never existed, and
its Ctrl+C scenarios predate the two-press quit. The inbox spec and README
still say Ctrl+P goes to the previous session.

## What Changes

- Ctrl+D quits only from an empty draft; with text it does nothing.
- The palette drops its thinking row; `/think` is already listed there.
- The tui key requirement lists the keys the code handles and its Ctrl+C
  scenarios match the two-press quit. Ctrl+R, Ctrl+T and Ctrl+H go.
- The keys card example names a real fallback: `Shift+Enter, Alt+Enter, Ctrl+J`.
- The inbox requirement cycles with Ctrl+N alone. README follows.

## Keyboard and resources

No new keys. Three promised keys leave the spec; none worked.

## Capabilities

### Modified Capabilities

- `tui`: key list, Ctrl+D guard, keys card example.
- `inbox`: session cycling on Ctrl+N alone.
