# Proposal

## Why

The keys card, the palette, the specs and the handlers each list keys on
their own. Today that shipped a palette row for Ctrl+T, which nothing
handles, and a keys card promising keys that never worked.

## What Changes

- A test sends every key on the keys card and runs every palette action, and
  fails when one leaves the app unchanged where it should act.
- Keys that only act in a state, such as Ctrl+X with a queued message open,
  are tested in that state.

## Keyboard and resources

No keys change. Test time only.
