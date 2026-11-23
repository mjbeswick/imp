# Design

## Approaches

- **Apply only the keys the file changed (chosen).** Session choices made by
  command survive without tracking them, and a deliberate file edit wins.
- **Reload everything, then replay session overrides.** Needs a record of
  every command that touches the config.

## Details

`Config::from_str` keeps the file's `key = value` pairs and the keys `set`
did not know. Reload diffs the old and new pairs, calls `set` for each
changed key on the session's config, copies named providers when a
`provider.*` key changed, and sends the same pairs to the chat, which applies
them to its own copy. The rest of `/reload` is unchanged.
