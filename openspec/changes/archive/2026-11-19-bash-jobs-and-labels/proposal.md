# Proposal

## Why

A model `bash` call that passes 30 seconds is killed, and its work is lost:
the model then reruns it in the background and waits again. Long pipelines also
fill the transcript, because the row shows the whole command, often across four
or five lines.

## What Changes

- A model `bash` call still running at 30 seconds becomes a background job
  instead of being killed. The result names the job and its log, and
  `job wait` continues from there. Typed `!` commands keep the timeout.
- `bash` takes an optional `description`, a few words on what the command does.
  The tool row shows it in place of the command; Ctrl+O and high verbosity
  still show the command.
- A confirmation for a labelled call names the command, since the row does not.

## Capabilities

### Modified Capabilities

- `tools`: timeout handover to jobs and the `description` argument.

## Impact

- `src/tools.rs`, `src/jobs.rs`, `src/agent.rs`, `src/tui.rs`.
