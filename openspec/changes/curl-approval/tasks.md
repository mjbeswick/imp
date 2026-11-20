# Tasks

- [x] 1.1 Add `perm::plain_get(cmd) -> Option<host>` recognising a plain `curl` GET piped only into safe-listed commands. Verify: unit tests for accepted and refused shapes.
- [x] 1.2 Route plain GETs in `bash` through the shared per-host approval in `normal`, keeping `strict` and `yolo` as for `fetch`. Verify: approval count test across `fetch` and `curl`.
- [ ] 1.3 Add the spill line to the prompt and run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
