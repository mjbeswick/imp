#!/bin/sh
# usage: scripts/archive.sh [--check] <change>
# Archives an OpenSpec change, refusing when a requirement it modifies has
# changed in the main spec since the change was proposed: archiving would
# overwrite that newer text. --check reports without archiving.
set -e
cd "$(dirname "$0")/.."
check=
[ "$1" = "--check" ] && { check=1; shift; }
change=$1
dir=openspec/changes/$change
[ -d "$dir" ] || { echo "no change $change" >&2; exit 2; }

# The commit that first added the change folder.
base=$(jj log --no-graph --reversed -r "files(\"$dir\")" -T 'commit_id ++ "\n"' | head -1)
[ -n "$base" ] || { echo "$change is not committed yet" >&2; exit 2; }

# Print one requirement's block, from its header to the next header.
block() {
    awk -v want="### Requirement: $1" '
        $0 == want { on = 1; print; next }
        on && /^###? / { exit }
        on { print }'
}

stale=$(for delta in "$dir"/specs/*/spec.md; do
    [ -f "$delta" ] || continue
    cap=$(basename "$(dirname "$delta")")
    main=openspec/specs/$cap/spec.md
    [ -f "$main" ] || continue
    awk '/^## /{m = ($0 == "## MODIFIED Requirements")}
         m && /^### Requirement: /{sub(/^### Requirement: /, ""); print}' "$delta" |
    while IFS= read -r name; do
        now=$(block "$name" < "$main")
        then=$(jj file show -r "$base" "$main" 2>/dev/null | block "$name")
        if [ "$now" != "$then" ]; then
            echo "stale: $cap: $name changed since $change was proposed" >&2
            echo x
        fi
    done
done)

if [ -n "$stale" ]; then
    echo "rebase the change's MODIFIED blocks on the current spec, then retry" >&2
    exit 1
fi
[ -n "$check" ] && { echo "$change: no stale requirements"; exit 0; }
openspec archive "$change" --yes
