# Tasks

- [x] 1.1 Return the page body without the heading outline. Verify: a test fetch returns the body on its first line and never contains `outline:`.
- [ ] 1.2 Tell the model to fetch with `search` instead of `curl`. Verify: the baseline prompt names both and a test asserts it.
- [ ] 1.3 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
