# Proposal

## Why

The OpenSpec jj workspace protocol ends with
`rm -rf ~/Projects/.workspaces/<repo>-<change>`. The project config denies
every `rm -rf *`, so the build agent cannot finish cleanup without a manual
override. Dead workspaces pile up in `~/Projects/.workspaces`.

## What Changes

- Allow `rm -rf` on paths inside `~/Projects/.workspaces/`, in both the `~`
  form and the absolute form.
- Keep `.apply-state` and any path containing `..` denied, so apply state and
  traversal out of the workspace root stay blocked.
- Every other `rm -rf` stays denied.

## Capabilities

None. Agent permission configuration, with `skip_specs: true`.

## Impact

- `opencode.jsonc`: the project `permissions` list only.
