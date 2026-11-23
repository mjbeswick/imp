# Design

## Approaches

- **A rule-based `auto` (chosen).** Free, instant, predictable, and the same
  parser that already classifies the safe list.
- **A model judges each command.** One more request per unlisted command, one
  to three seconds each on a Pi, and a judge that tool output can steer.
  Possible later as an opt-in; not now.
- **Project allow files.** A repository could grant itself permissions.
  Rejected.

## The built-in list

`cargo` build, check, test, clippy, fmt and doc; `npm`, `pnpm` and `yarn`
test, run, lint and build; `make`; `pytest`; `go` build, test, vet and fmt;
local `git`: status, diff, log, show, add, commit, branch and stash. The same
shape rules as the safe list apply: pipes between listed commands, no `&&`,
`;`, `$(…)` or redirects outside `2>&1`. Not listed: installs, `git push`,
`git checkout`, `rm`, `curl`, anything privileged.

## What auto is not

A build or test runs the project's own code, which can do anything. `auto`
removes prompts for the loop, not the risk, so it suits projects you trust.
`/settings` and the `/mode` picker list it with the modes.

## Order

Shift+Tab visits `normal`, `auto`, `yolo`, `strict`. The old order put strict
first, which most people never want.
