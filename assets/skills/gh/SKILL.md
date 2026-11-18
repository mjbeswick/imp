---
name: gh
description: Use the GitHub CLI for pull requests, issues, checks, and releases. Use for gh pr, gh issue, gh run, gh release, or repo metadata.
---

# gh

- Status first: `gh pr status`, `gh pr checks`, `gh run list -L 5`.
- Read: `gh pr view <n> --json title,body,files`, `gh issue list -L 20`.
- Never push, merge, or force anything without being asked.
- `gh api` for anything the subcommands do not cover; prefer `--jq` over piping.
- Auth problems: `gh auth status` before blaming the network.
