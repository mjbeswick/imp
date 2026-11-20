#!/bin/sh
# Report the binary size and its change since the last recorded size.
# The goal is as small as possible, not a fixed limit, so this never fails.
set -e
BIN=${BIN:-target/release/imp}
LAST_FILE=$(dirname "$0")/size.txt
cargo build --release --quiet
SIZE=$(wc -c < "$BIN" | tr -d ' ')
LAST=$(cat "$LAST_FILE" 2>/dev/null || echo "$SIZE")
printf 'imp: %s bytes (%+d since %s)\n' "$SIZE" "$((SIZE - LAST))" "$LAST"
[ "$1" = "--record" ] && echo "$SIZE" > "$LAST_FILE"
exit 0
