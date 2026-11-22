# Design

## Approaches

- **Optional value, strict (chosen).** The next argument is the id unless it
  is missing or starts with `-`.
- **Accept only id-shaped words.** Friendlier for `imp -r fix it`, but a
  mistyped id would resume the newest session and send the typo as a
  message. Rejected by the user.

## Parsing

In `main.rs`, `-r` takes `args[i + 1]` when it exists and does not start
with `-`; otherwise it sets `continue_last`. `next()` is unchanged for other
flags.

## Size

Expected: under 100 bytes.
