#!/bin/sh
# The gate. Run before every commit; CI runs the same file.
set -e
cd "$(dirname "$0")/.."

printf 'drift       '
python3 scripts/check-drift.py

printf 'openspec    '
if openspec validate --all --strict --no-interactive >/dev/null 2>&1; then echo "ok"; else
  echo "FAIL"; openspec validate --all --strict --no-interactive; exit 1
fi

printf 'tests       '
log=$(mktemp)
if cargo test --quiet >"$log" 2>&1; then
  grep -E '^test result' "$log" | tail -1
else
  echo "FAIL"; tail -20 "$log"; rm -f "$log"; exit 1
fi
rm -f "$log"

printf 'size        '
./scripts/size.sh | tail -1
