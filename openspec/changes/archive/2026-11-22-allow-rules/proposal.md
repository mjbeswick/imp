# Proposal

## Why

An allow can only be created by catching a prompt and answering `a`. Once
that prompt is gone, so is the chance. A user who wants a rule for a command
they run often has to wait until it asks, and a declined command is forgotten
the moment it happens.

## What Changes

- The `/mode` picker ends with an `a add` row; it opens `allow = ` in the
  composer row, and Enter saves the typed shape for every session.
- The picker lists the shapes asked this session that are not already
  allowed, newest first, marked `allowed` or `declined`.
- `a` on an ask row saves it at once; Enter pre-fills the rule prompt with it.
- The config write stays in one place: the worker appends the `allow` line.
- Nothing is listed as an allow until it is saved.

## Keyboard and resources

`a` and Enter on the new rows, shown in the picker footer. At most five recent
shapes are shown.

## Capabilities

### Modified Capabilities

- `permissions`: adding rules, and reviewing what was asked.
