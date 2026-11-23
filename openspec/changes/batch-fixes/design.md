# Design

## Approaches

- **Small fixes in place (chosen).** Each gap is a line or two at its source.
- **Reopen each change.** Six archives to touch for the same edits.

## Details

`fresh_session` builds every new pane and applies `reload_from(&self.cfg)`.
`load_history` and `save_history` return early under test, as `theme::save`
does. A clock test compares with `date` under whatever `TZ` is set.
