#!/bin/sh
# usage: scripts/deploy.sh <ssh-host>...
# Copies the source to ~/projects/imp on each device, builds it there with the
# device's own Rust, and installs ~/.local/bin/imp, keeping the old one as
# imp.prev. Needs rsync on both ends; sftp is not used.
set -e
cd "$(dirname "$0")/.."
for host in "$@"; do
    ssh "$host" 'mkdir -p ~/projects/imp ~/.local/bin'
    rsync -a --exclude target/ --exclude .jj/ --exclude .git/ --exclude .imp/ \
        --exclude .codegraph/ --exclude .claude/ ./ "$host:projects/imp/"
    ssh "$host" 'set -e
        cd ~/projects/imp
        flags=
        # Without the libcurl dev package, link the runtime library directly.
        if ! ls /usr/lib/*/libcurl.so /usr/lib/libcurl.so >/dev/null 2>&1; then
            lib=$(ls /usr/lib/*/libcurl.so.4 2>/dev/null | head -1)
            [ -n "$lib" ] || { echo "no libcurl on $(hostname)" >&2; exit 1; }
            mkdir -p target/curl-link
            ln -sf "$lib" target/curl-link/libcurl.so
            flags="-L $PWD/target/curl-link"
        fi
        RUSTFLAGS="$flags" ~/.cargo/bin/cargo build --release --quiet
        [ ! -f ~/.local/bin/imp ] || cp ~/.local/bin/imp ~/.local/bin/imp.prev
        install -m 755 target/release/imp ~/.local/bin/imp
        echo "$(hostname): $(wc -c < ~/.local/bin/imp) bytes"'
done
