#!/bin/sh
# Fail the build when the binary grows past the budget.
set -e
LIMIT=${LIMIT:-776000}   # one 16 KB step above 760,000, raised for the local fetch tool
BIN=${BIN:-target/release/imp}
cargo build --release --quiet
SIZE=$(wc -c < "$BIN" | tr -d ' ')
printf 'imp: %s bytes (limit %s)\n' "$SIZE" "$LIMIT"
[ "$SIZE" -le "$LIMIT" ]
