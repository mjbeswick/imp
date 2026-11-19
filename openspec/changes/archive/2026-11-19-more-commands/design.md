# Design

## Approaches

1. **Commands that reuse existing layers.** Chosen: `/shell` uses the attached
   runner, `/model` and `/provider` use the picker/form path, and `/stash` is
   an in-memory list.
2. **Separate binaries or config editors.** Too much surface for four commands.
3. **Persisting everything to the config.** Would rewrite user comments and
   blur session settings with durable ones.

## Decisions

- `/shell`: chat mode drops the alternate screen and cooked mode through the
  existing attached-command path, runs `$SHELL` (fallback `/bin/sh`), then
  re-enters and redraws. Line mode runs it directly.
- `/model`: a session field. Bare opens the picker with the active model, the
  provider profile default, and the first few names from
  `~/.local/state/imp/models`; choosing writes the name to the front of that
  file. Typed names apply directly.
- `/provider`: built-ins are fixed. Custom profiles live between
  `# >>> providers` and `# <<< providers` in the config, one line per field:
  `provider.<name>.base`, `.model`, `.key_env`, `.dialect`. The writer replaces
  only that block and leaves every other byte. `use` sets the runtime provider;
  deleting the active profile is refused.
- A custom profile named in `provider=` is resolved before the built-in enum.
- `/stash`: `Vec<String>` on App/Session, session-bound and not persisted. Bare
  lists `1. text`; `pop n` removes and sets the composer with caret at the end;
  `delete n` removes; `pop all` joins with blank lines; invalid indices report
  the valid range.
- Command count is nineteen; help and README list all four.

## Tests

- Shell suspension is exercised through the attached-runner hook.
- Model picker order and recents persistence.
- Provider block rewrite preserves a commented config; add/edit/delete/use;
  delete-active refused.
- Stash add/list/pop/delete/pop-all with composer contents.
