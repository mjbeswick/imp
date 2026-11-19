# Proposal

## Why

`App::new` takes the session root and then shadows it with
`std::env::current_dir()`. With `--cwd`, `/cwd` and typed `!` commands use the
launch directory until the first `/cd`, even though the session, the jail, and
the model's tools are already rooted elsewhere. A test run also reports the
parameter as unused.

## What Changes

- Use the root passed into `App::new` instead of re-reading the process
  directory.
- Assert the initial root in the command test so the shadow cannot return.

## Capabilities

None. This restores behaviour the `tui` and `tools` specs already require, so
the change uses `skip_specs: true`.

## Impact

- `src/tui.rs`: one line in `App::new`, one assertion in a test.
