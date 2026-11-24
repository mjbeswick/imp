# Tasks: external skills

## 1. Validation test

- [ ] 1.1 Test: each `optional-skills/*/SKILL.md` parses with `name` and `description`, and its body is under 4096 bytes. Confirm it fails before the skills exist.

## 2. Skills

- [ ] 2.1 Write `optional-skills/browse/SKILL.md`: `fetch` first, obscura for JavaScript pages, install hint, no stealth modes. Test 1.1 passes.
- [ ] 2.2 Write `optional-skills/image/SKILL.md`: chafa with `show`, install hint `brew install chafa`. Test 1.1 passes.
- [ ] 2.3 Write `optional-skills/mermaid/SKILL.md`: graphs-tui with `show`, install hint `cargo install graphs-tui`. Test 1.1 passes.

## 3. Documentation

- [ ] 3.1 Add a README section: the three skills in `optional-skills/`, that the agent installs one on request by copying it to `~/.config/imp/skills/`, and the install hint for each program.

## 4. Validation

- [ ] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Expected: size change zero.
- [ ] 4.2 Scan `optional-skills/` for token-like strings and for placeholder text. Expected: no hits.
- [ ] 4.3 Manual: copy one skill into a scratch config directory outside the repo and confirm the loader lists it. Running chafa or graphs-tui is not possible here (not installed); report that as not run.
- [ ] 4.4 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
