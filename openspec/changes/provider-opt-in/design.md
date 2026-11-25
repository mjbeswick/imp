# Design: provider opt-in

## Context

- `Config::provider_configured` (`src/config.rs:239`) returns true for a built-in with no key variable,
  which is why Ollama and Custom always counted as set up.
- The `/model` list loop (`src/tui.rs`, in `model_items`) kept a provider when it was the active one, or
  when it was configured and not switched off. For Ollama and Custom that was always true.
- The `/provider` list (`src/tui.rs`, `provider_items`) built one row per built-in. A switched-off row
  showed "· off", an unconfigured one "· no key".
- The on/off setting is `provider.<name>.enabled` (`src/config.rs`, the `set` branch that also serves the
  config file loader in `from_str`). `false` adds the name to `disabled_providers`.

## Decisions

- **One list for switched-on providers.** `enabled_providers` holds the names set to `true` in
  `provider.<name>.enabled`. It is maintained in the same `set` branch as `disabled_providers`, so the
  config file and in-memory updates agree.
- **One rule for "offered".** `Config::provider_listed(provider, lookup)` decides whether a built-in is in
  the lists. Order of checks: the active provider is offered; a switched-off one is not; a keyless one is
  offered only when switched on; a keyed one is offered when its key is set. The `/model` loop calls it in
  place of its inline check.
- **Hidden rows are skipped, not greyed.** The `/provider` list skips a keyless built-in that is neither
  active nor switched on. This matches the request: they should not appear until set up.
- **Keyed rows are unchanged.** The "· off" and "· no key" labels stay for keyed built-ins.

## Risks

- A user who relied on Custom or Ollama appearing without setup will not see them after this change. The
  config line restores them.
- Space in `/provider` cannot switch on a hidden row, because the row is not listed. Switching on is done
  in the config. A picker control would need its own change.

## Assumptions

- No other code path lists built-ins by iterating providers directly. The search found only the two lists
  above and `configured_fallback`, which already ignores keyless providers.
