#!/bin/sh
# The gate. Run before every commit; CI runs the same file.
set -e
cd "$(dirname "$0")/.."

printf 'drift       '
changed=$(jj diff --name-only 2>/dev/null)
if [ -z "$changed" ]; then
  echo "ok (nothing changed)"
else
  code=$(printf '%s\n' "$changed" | grep -vE '^(target/|\.jj/|\.git/|openspec/)' | grep -c . || true)
  specs=$(printf '%s\n' "$changed" | grep -c '^openspec/changes/' || true)
  active=$(openspec list --json 2>/dev/null | grep -c '"name":' || true)
  if [ "$code" -gt 0 ] && [ "$specs" -eq 0 ]; then
    echo "FAIL: source changed with no update under openspec/changes/"
    printf '%s\n' "$changed" | grep -vE '^(target/|\.jj/|\.git/|openspec/)' | head -8 | sed 's/^/  changed: /'
    echo "  Fix: tick the task you are doing, or add a change that covers it."
    exit 1
  fi
  if [ "$code" -gt 0 ] && [ "$active" -eq 0 ]; then
    echo "FAIL: source changed but no OpenSpec change is active"
    exit 1
  fi
  echo "ok ($code source, $specs spec, $active active)"
fi

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
