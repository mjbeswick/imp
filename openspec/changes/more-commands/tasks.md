# Tasks

- [ ] 1.1 Add `/shell`. Suspend and restore the interface around `$SHELL`, fall back to `/bin/sh`, and run it directly in line mode. Verify: the attached-runner hook is called and the screen redraws.
- [ ] 1.2 Add `/model`. Set a typed model for the session; bare opens the picker with current, profile default, and recents; record choices in the state file. Verify: a typed model applies and a chosen one moves to the front.
- [ ] 1.3 Add `/provider`. Parse and rewrite only the managed config block; list built-ins and custom; add, edit, delete, use; refuse deleting the active profile. Verify: comments survive a rewrite and each verb behaves.
- [ ] 1.4 Add `/stash`. Session-bound list; add, list, pop, delete, pop all into the composer. Verify: pop moves text without sending, pop all joins, and an invalid index is refused.
- [ ] 1.5 Update the command table, `--help`, README, and the nineteen-command spec. Verify: `/help` lists nineteen.
- [ ] 1.6 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
