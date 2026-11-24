# Design: provider manager

## Comparable implementation

The provider picker and provider form already exist: `provider_items` builds the list in
`src/tui.rs`, `PickKind::Provider` handles choices, `PickKind::ProviderModel` and the
provider form (`ProviderForm`) handle editing. `skill-toggles` and `provider-toggles` are the
pattern for a toggle that is saved with `save_setting` and reloaded.

## Decisions

**Providers list is the existing `/provider` picker, extended.** Judgement call: extend the
picker rather than add a second screen, so one command stays one screen. Cost if wrong: the
picker gains more keys, which the keys card must list.

**Keys.** Enter opens the config modal. `u` switches active, `space` toggles enabled,
`d` deletes a named provider, `n` adds one. Built-in providers reject `d` with a message.
Judgement call: `space` for enable matches the skills picker's toggle intent, but the
skills picker uses Enter, so space here is new. Cost if wrong: a rebind.

**Config modal is the existing provider form.** It already edits base, model, key variable,
and dialect, and already saves. This change adds Cancel that discards edits, if the form
does not already have it, and checks that a typed key goes to the keys file.

**Delete removes the named provider's lines** from the managed providers block. If the
deleted provider is active, the active provider falls back to DeepSeek, the default.

**Persistence.** Every save goes through `save_setting` or the managed-block writer, so
comments survive, as the existing tests require.

## Validation

- Unit tests per key action: enter opens config, `u` switches, `space` toggles and saves,
  `d` deletes a named provider and refuses a built-in, `n` opens a new config.
- Config modal tests: save writes the change; cancel leaves the config and keys file
  untouched; a typed key is written to the keys file only.
- `cargo test`, `openspec validate --all --strict`, `./scripts/size.sh`.
