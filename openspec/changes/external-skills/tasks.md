# Tasks: external skills

## 1. Validation test

- [x] 1.1 Test: each `optional-skills/*/SKILL.md` parses with `name` and `description`, and its body is under 4096 bytes. Confirm it fails before the skills exist.

## 2. Skills

- [x] 2.1 Write `optional-skills/browse/SKILL.md`: `fetch` first, obscura for JavaScript pages, install hint, no stealth modes. Test 1.1 passes.
- [x] 2.2 Write `optional-skills/image/SKILL.md`: chafa with `show`, install hint `brew install chafa`. Test 1.1 passes.
- [x] 2.3 Write `optional-skills/mermaid/SKILL.md`: graphs-tui with `show`, install hint `cargo install graphs-tui`. Test 1.1 passes.

## 3. Documentation

- [x] 3.1 Add a README section: the three skills in `optional-skills/`, that the agent installs one on request by copying it to `~/.config/imp/skills/`, and the install hint for each program.

## 4. Validation

- [x] 4.1 Run `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh`. Expected: size change zero.
- [x] 4.2 Scan `optional-skills/` for token-like strings and for placeholder text. Expected: no hits.
- [x] 4.3 Manual check: copied the image skill into a scratch folder outside the repo, the loader lists it (test an_installed_optional_skill_is_listed_from_its_folder), and chafa 1.18.3 rendered a sample image to 19 lines. Installed chafa with brew after approval.
- [x] 4.4 Reinstall with `cargo build --release && install -m 755 target/release/imp ~/.local/bin/imp`.
