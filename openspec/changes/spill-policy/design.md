# Design

## Approaches

1. **A bounded memory buffer with a virtual address.** Chosen: no files for
   command output, no retention, no jail widening, and the full text already
   exists in memory before truncation.
2. **Disk spill with a jail exception and pruning.** Survives restart, but adds
   litter and the path the prompt told the model to read was broken.
3. **No recovery at all.** Simplest, but loses the middle of a one-shot command.

## Decisions

- `Tools` owns `buffers: HashMap<u64, String>`, an order list, a byte total, a
  next id, and a 16 MB cap. Storing an output evicts least-recently-used ids
  until it fits; a single output is already capped at 1 MB at capture.
- `cap` keeps its name for command, job-wait, and fetch results. On overflow it
  stores the text and returns the 2 KB head, 1 KB tail, byte count, and
  `imp://out/<n>` with a hint to read a range. `trim` is the same head and tail
  without storing, and says to use offset and limit.
- `read` checks for the `imp://out/` prefix before path resolution. A virtual
  read applies the same offset, limit, line numbers, and 200 KB ceiling as a
  file, and reports `buffer imp://out/3 expired` when the id is gone. It never
  touches the jail or `read_paths`.
- The model cannot grep a virtual buffer with `bash`; the hint says to read a
  range, or re-run the command narrowed. This is honest and avoids promising a
  search path that does not exist.
- Job logs remain `out/jobs/job-<n>.log`. `confined` also accepts the output
  directory, so `read` opens them. Only that directory is added.
- A startup prune walks `out/jobs`, deleting files older than seven days by
  mtime and ignoring errors. No other out files are written any more.
- `/new` clears the buffers in both front-ends; old addresses then report
  expiry. Buffers do not survive exit, which is accepted.
- Prompt lines:
  `- Results over 4 KB go to a buffer: you get a head, a tail, and imp://out/<n>. Read that address with offset and limit when you need more.`
  `- read, edit, and write reach only the session directory. bash is not jailed, so keep commands inside the project unless the user asks otherwise.`
- Tests cover: storing and slicing, offset and limit, expiry after eviction,
  the 16 MB cap order, read never buffering, job logs readable, pruning an aged
  and a fresh file, buffers cleared on new, and the two prompt lines.
