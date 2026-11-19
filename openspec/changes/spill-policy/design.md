# Design

## Approaches

1. **A bounded memory buffer with a virtual address.** Chosen: no files for
   command output, no retention, no jail widening, and the full text already
   exists in memory before truncation.
2. **Disk spill with a jail exception and pruning.** Survives restart, but adds
   litter and the path the prompt told the model to read was broken.
3. **No recovery at all.** Simplest, but loses the middle of a one-shot command.

## Decisions

- `Tools` owns a buffer map, an order list, a memory byte total, a spill byte
  total, a next id, and two caps: 16 MB of memory and 64 MB of unlinked spill.
- A buffer is either `Memory(String)` or `File(File)`. Storing an output puts
  it in memory when it fits. Otherwise the least-recently-used memory buffers
  are demoted to temporary files until it fits.
- Demotion creates a temp file, writes the text, seeks to zero, then unlinks it
  while the handle stays open. The id keeps working, nothing appears in a
  directory, no jail exception is needed, and the storage is freed when the
  handle closes or the process exits. A crash leaves nothing because the file
  was unlinked at creation.
- Past the 64 MB spill cap, the oldest file-backed buffer is closed and dropped;
  its id reports expiry.
- `read` checks for the `imp://out/` prefix before path resolution. A memory
  buffer is sliced directly. A file buffer is read through `&File` with
  `BufReader`, skipping to the offset and taking the limit, with the same line
  numbers and 200 KB ceiling. Neither touches the jail or `read_paths`.
- `cap` keeps its name for command, job-wait, and fetch results. On overflow it
  stores the text and returns the 2 KB head, 1 KB tail, byte count, and
  `imp://out/<n>` with a hint to read a range. `trim` is the same head and tail
  without storing, and says to use offset and limit.
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
- Tests cover: storing and slicing, offset and limit, demotion keeping an id
  readable, no file becoming visible, the spill cap expiring the oldest,
  read never buffering, job logs readable, pruning an aged and a fresh file,
  buffers cleared on new, and the two prompt lines.
