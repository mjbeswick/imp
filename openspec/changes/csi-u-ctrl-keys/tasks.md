# Tasks

- [x] 1.1 Decode CSI-u Ctrl letters. Verify: `decode` turns `CSI 99;5u` into `Key::Ctrl('c')` and `CSI 108;5u` into `Key::Ctrl('l')`.
- [x] 1.2 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
