# Tasks

- [x] 1.1 Run fetch and plain curl GETs without asking in `normal`, asking for URLs over 300 characters. Verify: tests for normal, long URL, strict, yolo, and curl.
- [x] 1.2 Put today's date and the fresh-data rule in the prompt, and shorten the fetch line. Verify: tests for the date line and the baseline budget.
- [x] 1.3 Return HTML as markdown and feeds as lists. Verify: tests for headings, links, lists, tables, dropped navigation, RSS, and Atom.
- [x] 1.4 Add `medium` verbosity with result gists and short fetch labels, in both front-ends and the config. Verify: tests for each level and gist.
- [x] 1.5 Rerun the six conversations without answering, update `README.md`, run the checks, record the size, and reinstall. Verify: no prompts, no curl, a current F1 answer, and `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
