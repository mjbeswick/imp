# Design

## Decisions

1. Hard rename. No aliases, no fallback reads. One name everywhere.
2. `MOTE_*` becomes `IMP_*`. Nine variables: model, base URL, permissions,
   allow sudo, prune at, compact at, max tokens, colour, skills dir.
3. `~/.config/mote` and `~/.local/state/mote` become `imp`. Existing data is
   moved by hand, once, before first run.
4. Project-local `.mote/` becomes `.imp/`. The repository marker check accepts
   `.git`, `.jj`, or `.imp`.
5. The permission guard that refuses edits to the agent's own config names the
   new path.
6. The repository directory is renamed last, after this change is archived and
   the workspace is gone. jj workspace git pointers are relative to the repo
   directory, so renaming under a live workspace breaks them.

## Migration

```sh
mv ~/.config/mote ~/.config/imp
mv ~/.local/state/mote ~/.local/state/imp
```

Names of files inside do not change. Sessions keep their ids.

## Verification

- `cargo test` proves env precedence, path construction, and loader roots.
- `imp -l` lists the migrated sessions.
- `openspec validate --all --strict` proves the specs match the rename.
- `./scripts/size.sh` stays under budget.

## Out of Scope

- Data migration code. One `mv` is cheaper and easier to inspect.
- Renaming session ids, bookmarks, or commit messages.
