# Design

## Approaches

- **Rows in the existing `/mode` picker (chosen).** Same surface as listing
  and revoking; no new command and no new key.
- **A `/permissions` command.** A second view of the same list, and a
  twenty-sixth command. Rejected.
- **Editing the config by hand with `/edit`.** No such command, and the shape
  syntax is the part people get wrong.

## Details

Ask rows come from one new `Hooks::answered(shape, answer)` call at the
confirmation site, which the worker forwards as `Ev::ShapeAsked { shape,
allowed }`. The app keeps the last ten, newest first, and shows at most five
that are not already allowed, after the allow rows.

`a` on the add row or on an ask row opens `Pending::AllowRule`, or saves at
once on an ask row. Saving sends `Cmd::AllowAlways(shape)`; the worker calls
`cfg.allow_always`, which appends the line and updates its own list, and the
app adds the row locally. A failed write comes back as an error row and the
allow is not shown.

The rule prompt refuses an empty shape and one already allowed.

## Size

Expected: under 3 KB.
