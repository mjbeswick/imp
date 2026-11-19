# Design

## Approaches

1. **Split cap into spill and trim.** `cap` writes and formats; `trim` only
   formats. Chosen: read stops creating duplicates with one branch change.
2. **A size threshold per tool.** More knobs for no benefit; the 4 KB target is
   the same for the model either way.
3. **Drop spill entirely.** Loses the middle of one-shot command output, which
   is the case the mechanism exists for.

## Decisions

- One helper builds the head-tail text: 2 KB head, 1 KB tail, byte count, and a
  note. `cap` adds a spill path; `trim` says to use offset and limit.
- `Tools::read` calls `trim`. `bash`, job wait, and fetch keep `cap`.
- `confined` also accepts `self.out_dir`, canonicalised. The config directory
  exception stays. Nothing else widens.
- Pruning runs once in `main` after the output directory is known. It walks
  `out/` and `out/jobs/`, deleting files whose modification time is older than
  seven days; directories and errors are ignored.
- The seven-day rule uses file mtime, not name, so job logs and spilled results
  share one policy.
- Prompt lines become:
  `- Results over 4 KB spill to a file: you get a head, a tail, and the path. Read it, or search it with bash, when you need the rest.`
  `- read, edit, and write reach only the session directory. bash is not jailed, so keep commands inside the project unless the user asks otherwise.`
- The second line no longer says "paths" in general, so the model cannot
  assume the shell is jailed, and it uses the same vocabulary as the errors.
- Tests cover: trim writes no file, cap does, a read result names offset and
  limit, `confined` accepts `out_dir`, pruning deletes an aged file and keeps a
  fresh one, and the prompt contains both new lines and no longer says
  "working directory".
