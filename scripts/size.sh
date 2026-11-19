#!/bin/sh
# Fail the build when the binary grows past the budget.
set -e
LIMIT=${LIMIT:-760000}   # headroom for the picker and multi-provider adapters
BIN=${BIN:-target/release/imp}
cargo build --release --quiet
SIZE=$(wc -c < "$BIN" | tr -d ' ')
printf 'imp: %s bytes (limit %s)\n' "$SIZE" "$LIMIT"
[ "$SIZE" -le "$LIMIT" ]
