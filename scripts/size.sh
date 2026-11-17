#!/bin/sh
# Fail the build when the binary grows past the budget.
set -e
LIMIT=${LIMIT:-1572864}   # 1.5 MB
BIN=${BIN:-target/release/mote}
cargo build --release --quiet
SIZE=$(wc -c < "$BIN" | tr -d ' ')
printf 'mote: %s bytes (limit %s)\n' "$SIZE" "$LIMIT"
[ "$SIZE" -le "$LIMIT" ]
