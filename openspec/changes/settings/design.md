# Design

## Approaches

- **A form built on the picker (chosen).** Rows, filtering, scrolling and
  clicks already work; the value sits in the description column.
- **A separate form widget.** More code for the same rows.

## Details

A table names each setting's key, label and values. Stepping one calls
`Config::set` in the chat and sends `Cmd::SetConfig` so the session applies
it too; the start mode goes through `Cmd::SetPermissions` so the jail
follows. `save_setting` replaces the last uncommented `key=` line, or
appends one, and leaves every other line alone. Under test nothing is
written unless the config names a file.

The gradient interpolates RGB from `Select` to a mix of `Select` and
`Accent`, so text keeps its contrast at both ends.
