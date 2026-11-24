# Tasks: external skills

## 1. Validation test

- [ ] 1.1 Test: each `optional-skills/*/SKILL.md` parses with `name` and `description`, and its body is under 4096 bytes. Confirm it fails before the skills exist.

## 2. Skills

- [ ] 2.1 Write `optional-skills/browse/SKILL.md`: `fetch` first, obscura for JavaScript pages, install hint, no stealth modes. Test 1.1 passes.
- [ ] 2.2 Write `optional-skills/show-image/SKILL.md`: detect the renderer, use `show`. Test 1.1 passes.
- [ ] 2.3 Write `optional-skills/show-chart/SKILL.md`: detect the renderer, use `show`. Test 1.1 passes.

## 3. Documentation

- [ ] 3.1 Add a README section: the three skills, how to copy one into `~/.config/imp/skills/`, and the install hint for each program.

## 4. Validation

- [ ] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Expected: size change zero.
- [ ] 4.2 Scan `optional-skills/` for token-like strings and for double square brackets or redaction prefixes. Expected: no hits.
- [ ] 4.3 Manual: copy one skill into a scratch config directory outside the repo and confirm the loader lists it.
- [ ] 4.4 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
