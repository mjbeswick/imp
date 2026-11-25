# Tasks: provider opt-in

## 1. Switched-on list

- [x] 1.1 Test: `provider_listed` is false for Custom and Ollama by default, true after `provider.<name>.enabled = true`, and false again after `= false`.
- [x] 1.2 Test: a config line `provider.ollama.enabled = true` read by `from_str` switches Ollama on and leaves Custom off.
- [x] 1.3 Test: a keyed built-in is listed only once its key is set.
- [x] 1.4 Implement: `enabled_providers`, its update in the `provider.<name>.enabled` branch, `provider_opted_in`, and `provider_listed` in `src/config.rs`.

## 2. Lists

- [x] 2.1 Test: with no opt-in, `/provider` has no Custom or Ollama row, and `/model` has no `custom::` or `ollama::` row.
- [x] 2.2 Test: with the opt-in, `/provider` lists the switched-on provider.
- [x] 2.3 Implement: `model_items` uses `provider_listed`, and `provider_items` skips a hidden keyless built-in in `src/tui.rs`.
- [x] 2.4 Update existing provider-row tests to switch Custom and Ollama on in their setup, since they reach those rows.

## 3. Docs and validation

- [x] 3.1 Update `README.md`, which lists the built-in providers, to say Ollama and Custom are switched on in the config.
- [x] 3.2 Run `cargo test` and `openspec validate --all --strict`, and record the outcome. Result: 830 unit tests pass. The two `tests/tty.rs` failures predate this change (they also fail on `main`).
