# Design

## Approaches

- **Judge the clause (chosen).** `-exec ls -l {} +` is exactly as safe as
  `ls -l`, so reuse `safe_pipeline` on what follows the flag until `;` or `+`.
- **Keep refusing every exec.** Simple, but leaves the reported prompt.
- **A config list of allowed exec programs.** Moves the risk to a file and
  needs a new key; the safe list already exists.
- **Allow `fd --exec` too.** Its clause has no terminator, so the boundary is
  the end of the command; a smaller gain for more parsing. Left asking.

## Details

`safe_pipeline`'s `find` arm drops `-exec` and `-execdir` from its forbidden
flags and requires every clause to be safe. A clause runs from the flag to the
next `;`, `\;`, or `+`; no terminator fails closed. The clause is judged as a
pipeline, so nested `find` and writing flags such as `sort -o` are caught.

`shape` returns `None` for any command naming `-exec`, `-execdir`, `-ok`,
`-okdir`, `--exec`, or `xargs`, so no always is saved that would later cover a
writer.

## Size

Expected: under 500 bytes.
