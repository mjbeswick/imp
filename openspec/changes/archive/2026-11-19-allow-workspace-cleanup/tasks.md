# Tasks

## 1. Permissions

- [x] 1.1 Add the workspace allow rules after the existing `rm -rf *` deny, plus
  the `.apply-state` and `..` denies. Verify: on a scratch directory,
  `rm -rf ~/Projects/.workspaces/<name>` succeeds, and
  `rm -rf ~/Projects/.workspaces-probe` is denied.
