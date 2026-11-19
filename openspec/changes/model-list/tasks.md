# Tasks

- [x] 1.1 Add the provider model source. A GET beside the POST, a catalog per provider, endpoint and response parsing for OpenAI-shaped, Anthropic, and Gemini lists, and a `list_models` entry point with a five-second timeout. Verify: fixed JSON parses to names for each shape, and a bad response is an error.
- [x] 1.2 Fetch from bare `/model`. Seed the picker from the catalog, send `Cmd::ListModels` with the provider, base, and key, and merge `Ev::Models` into the open picker while preserving the filter. Verify: an open picker gains the live rows, a failed fetch keeps the catalog, and a provider switch swaps the catalog.
- [ ] 1.3 Update the README and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
