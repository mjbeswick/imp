---
name: rg
description: Search file contents with ripgrep. Use for finding text, symbols, TODOs, or which files mention something. Faster and safer than grep.
---

# rg

- `rg -n "pattern"` for line numbers; `rg -l` for filenames only.
- Narrow with `-t rust`, `-g '*.md'`, or a path argument.
- `rg --files` lists files; `rg -c` counts matches per file.
- Respects .gitignore by default; add `-u` to include ignored files.
- If output is huge, add `-m 5` or pipe through `head`.
