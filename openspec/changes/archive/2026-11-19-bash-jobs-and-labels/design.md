# Design

## Approaches

1. **Hand the running process to the job table.** Chosen: nothing reruns, and
   `job wait` already reads a log from a cursor.
2. **Raise the timeout.** Rejected: a stuck command would block the turn
   longer, and the model still could not do anything else meanwhile.
3. **A built-in file-walking tool.** Rejected: `find` itself was fast; the cost
   was the killed command and the round trips.

## Decisions

- `Tools::bash` writes stdout to a log under `out_dir/jobs/` instead of a pipe,
  and reads at most the capture cap back when the command ends, then removes
  the log.
- `Tools::bash_handover` runs the same way, but at the timeout it stores the
  child, command, and log in `Tools::handover` instead of killing. Cancel still
  kills.
- `Jobs::adopt(command, child, log)` registers it like `start` does.
- The agent's `bash` path uses `bash_handover`, adopts any handover, and
  returns `still running after 30s; now job {id}; log {path}; use job wait`.
- `describe` shows a non-empty `description` instead of `run \`command\``.
- The chat shows the full `run \`command\`` in a confirmation when the tool
  row's summary does not contain the command.

## Tests

- A command that outlives a short timeout is adopted, and `job wait` returns
  its later output and exit code.
- A typed command that outlives the timeout is still terminated.
- `describe` uses the description and falls back to the command.
- A labelled call's confirmation row names the command.
