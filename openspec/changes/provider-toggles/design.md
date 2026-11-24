# Design: provider toggles

## Comparable implementation

`skill-toggles` is the pattern: a `<kind>.<name>.enabled` config key read by `Config::set`,
a `disabled_*` list with a `*_disabled(name)` accessor, and a filter applied where the list
is built. The `/model` list is built in `model_items` in `src/tui.rs`, and the built-in
providers come from `Provider::profile` in `src/providers/mod.rs`.

## Decisions

**Configured means: a key variable is set, or the profile needs no key.** The check uses
the same lookup as `resolve_key_env`. Judgement call: reuse that lookup so the list and the
key gate cannot disagree. Cost if wrong: a provider whose key is only in the config
`api_key` would be hidden; the check includes that case.

**Config key: `provider.<name>.enabled = false`.** It follows the `provider.<name>.*` form
already used for named providers. Judgement call: the same prefix as the named providers,
so one syntax covers both. Cost if wrong: a rename touches the parser only.

**Filter in `model_items`.** Built-in providers that are unconfigured or disabled are
skipped there. The active provider is always kept, so the current session is never
hidden. Judgement call: keep the active one even if unconfigured, so the key gate can
still explain what is missing. Cost if wrong: the active row shows a missing key, which is
the correct message.

**Key check uses the environment lookup, not a network call.** No provider is contacted
to decide whether it is listed.

## Validation

- Unit tests: an unconfigured built-in is absent from `model_items`; a configured one is
  present with `provider::model` values; `provider.<name>.enabled = false` removes it; the
  active provider is kept.
- Config round-trip for `provider.<name>.enabled`.
- `cargo test`, `openspec validate --all --strict`, `./scripts/size.sh`.
