---
name: jq
description: Query and reshape JSON with jq. Use for jq filters, extracting fields, formatting JSON output, or checking API responses.
---

# jq

- Extract: `jq '.items[] | {id, name}' file.json`
- Keys: `jq 'keys'`, types: `jq 'map(type)'`.
- Raw strings: `-r`; compact: `-c`; slurp many files: `-s`.
- Validate: `jq empty file.json` prints nothing on success.
- Do not edit files in place without a backup; write to a new file and diff.
