# Tasks: provider toggles

## 1. Config

- [ ] 1.1 Test: `provider.<name>.enabled = false` is read as disabled; absent or true is enabled. Confirm it fails first.
- [ ] 1.2 Implement: `disabled_providers` and `provider_disabled(name)` in `Config::set`.
- [ ] 1.3 Test: the switch round-trips through `save_setting` and `Config::load`, keeping comments.

## 2. Configured check

- [ ] 2.1 Test: a built-in provider with no key variable set is configured only if it needs no key.
- [ ] 2.2 Implement: a `configured` check using the same lookup as `resolve_key_env`.

## 3. /model list

- [ ] 3.1 Test: an unconfigured built-in (OpenAI with no key) is absent from `model_items`; a configured one is listed with `provider::model` values.
- [ ] 3.2 Test: a disabled provider is absent; the active provider is kept even if unconfigured.
- [ ] 3.3 Implement: filter in `model_items`.

## 4. Validation

- [ ] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Record the size.
- [ ] 4.2 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
