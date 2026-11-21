# Proposal

## Why

A test of imp as a coding agent, four tasks in a scratch Python project,
found:

- The prompt never names the session directory. In one task the model's
  first command was `cd` into another repository; it then edited a different
  project. A write outside the jail was also asked about before being refused.
- `2>/dev/null` and `2>&1` caused about half the prompts, though they write
  no file.
- Every test run, such as `python3 -m unittest 2>&1 | tail -5`, asked, and
  `always` was never offered, because the command has a pipe.
- Files read with `cat` did not count as read, so a later `write` was
  refused twice.

## What Changes

- The prompt names the session directory. A `cd` that leaves it asks.
- A write or edit the jail refuses is refused at once, without a prompt. A
  write in the scratch folder does not ask.
- Redirecting stdout or stderr to `/dev/null`, or stderr to stdout, is not a
  write.
- `always` covers a command piped only into safe filters, by the first
  command's shape.
- A file printed whole by a safe `cat` counts as read.

## Capabilities

### Modified Capabilities

- `permissions`: quiet redirects, cd out of the directory, piped always.
- `tools`: jail refusals first, scratch writes, cat reads.
- `conversation-context`: the prompt names the session directory.
