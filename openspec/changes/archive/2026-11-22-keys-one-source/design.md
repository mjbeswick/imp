# Design

## Approaches

- **A test over the existing tables (chosen).** `help::KEYS` and
  `PALETTE_ACTIONS` stay where they are; the test reads both.
- **One key table driving card, palette and handlers.** Cleaner, but a
  rewrite of `handle_key_inner` for a problem a test catches.

## Details

The keys card rows hold display names such as `Option+Left, Option+Right`.
A small map turns each name into the `Key` values it means, so a new row
without a mapping fails the test too. Each key gets the state it needs, and
the test compares a snapshot of the app before and after.
