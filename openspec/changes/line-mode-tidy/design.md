# Design

## Approaches

- **Track whether output stopped mid-line (chosen).** One flag on the line
  front-end, set by reply text and cleared by every row.
- **Never start a new line.** A row would run on after a partial reply.

## Details

`Cli` keeps `mid_line`; `print_ui_to` takes it and writes a newline before a
row only when it is set.
