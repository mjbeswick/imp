# Design

## Approaches

1. **Extend `read`.** One schema, one dispatch branch, and a directory is just
   another path in a call that already batches. Chosen: it removes the
   discovery turn without adding a tool the model must choose between.
2. **Add `ls`.** Clearer naming, but a new schema, a new tool, and an amendment
   to the rule that forbids list tools.
3. **Add `tree`.** Good orientation output, but it still cannot return file
   contents, so a read always follows.

## Decisions

- `Tools::read` gains `depth`. Files ignore it; directories clamp it to 1-3 and
  default to 1.
- A directory is resolved through the same path jail as a file, so a listing
  cannot escape the session root.
- Entries are sorted by name. Directories get a trailing `/`. Symlinks get `@`
  and are never followed, so a loop cannot hang the walker.
- Skipped names are `.git`, `.jj`, `target`, `node_modules`, `dist`, `build`,
  and `.venv`. Other dotfiles are shown, because `.gitignore` and `.github`
  matter.
- Each directory is capped at 200 entries and says how many were cut. The whole
  result also stops at a 3.5 KB listing budget, below the existing 4 KB spill
  threshold, so a listing is never replaced by a head-tail spill.
- Output nests with two spaces per level:

```
src/
  api.rs
  tools/
    mod.rs
```

- Files and directories in one call are emitted in the order requested, each
  under its `== path ==` header, matching the current read format.
- Files still add the path to `read_paths`; directories do not, so listing a
  directory does not authorise overwriting a file inside it.
- The tool description names directories and `depth`, and the system prompt
  gains one sentence: ask for every path and directory you need in one call.
- Tests cover sorting, markers, depth, ignores, symlinks, the per-directory
  cap, the global budget, a missing directory, a jailed directory, and a mixed
  call. A scripted mixed call proves the round trip is one.
