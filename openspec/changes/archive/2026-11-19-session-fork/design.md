# Design

## Approaches

1. **Copy the files.** The transcript already is the history, so copying it
   preserves reasoning, tool calls, usage, and compaction exactly, with no
   re-encoding. Chosen.
2. **Rebuild from in-memory messages.** Loses log events such as usage and
   compaction summaries, and would duplicate the serializer.
3. **Mark a branch point inside one file.** Resume and the picker assume one
   file per session; this would need a format change.

## Decisions

- `Log::fork(&self)` picks a new timestamp id, retrying until the path is free,
  copies the transcript bytes, and writes a sidecar with the new id, the same
  cwd and model, and a title of `fork of <source title>` truncated to the
  usual length. An empty source title becomes `fork`.
- `Session::fork(&mut self)` requires an attached log, forks it, attaches the
  copy, logs a `fork` event naming the source, and returns both ids. History is
  not reloaded or cleared; the in-memory context is already correct.
- The transcript is copied between turns, so no append can interleave.
- Chat mode sends `Cmd::Fork`. The worker performs the fork, updates the shared
  session id used by the exit line, and replies with `fork: <new> (from <old>)`.
- Line mode calls `Session::fork` directly and prints the same line.
- The source file and sidecar are read-only in this operation; a failed copy
  leaves the running session attached to the source and reports the error.
- The slash-command list update is deferred until this lands, because
  `image-paste` and `cli-polish` also change the list. The command itself is
  added in the same implementation commit as that update.
- Tests use temp session directories: byte equality, distinct ids, source
  untouched, title and cwd carried, fork of a missing log refused, and a second
  fork creating a second distinct file.
