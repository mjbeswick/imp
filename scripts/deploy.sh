#!/bin/sh
# usage: scripts/deploy.sh [--native] <ssh-host>...
# Builds imp here for each device with cargo-zigbuild, pinned to the device's
# glibc and linked against its libcurl, then copies it to ~/.local/bin/imp,
# keeping the old one as imp.prev. No compiler runs on the device.
# --native instead copies the source to ~/projects/imp and builds it there
# with the device's own Rust. Needs only ssh; sftp is not used.
set -e
cd "$(dirname "$0")/.."
native=
[ "$1" = "--native" ] && { native=1; shift; }

native_build() {
    ssh "$1" 'mkdir -p ~/projects/imp ~/.local/bin'
    rsync -a --exclude target/ --exclude .jj/ --exclude .git/ --exclude .imp/ \
        --exclude .codegraph/ --exclude .claude/ ./ "$1:projects/imp/"
    ssh "$1" 'set -e
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
}

cross_build() {
    # The device says its architecture and glibc; its libcurl is the sysroot.
    set -- "$1" $(ssh "$1" 'uname -m; ldd --version | sed -n "1s/.* //p"')
    host=$1 arch=$2 glibc=$3
    sysroot=target/sysroot/$arch
    if [ ! -f "$sysroot/libcurl.so" ]; then
        mkdir -p "$sysroot"
        ssh "$host" 'cat "$(ls /usr/lib/*/libcurl.so.4 /usr/lib/libcurl.so.4 2>/dev/null | head -1)"' \
            > "$sysroot/libcurl.so"
        [ -s "$sysroot/libcurl.so" ] || { rm -f "$sysroot/libcurl.so"; echo "no libcurl on $host" >&2; exit 1; }
    fi
    # libcurl's own dependencies resolve on the device, not here.
    RUSTFLAGS="-L $PWD/$sysroot -C link-arg=-Wl,--allow-shlib-undefined" \
        cargo zigbuild --release --quiet --target "$arch-unknown-linux-gnu.$glibc"
    ssh "$host" 'set -e
        mkdir -p ~/.local/bin
        cat > ~/.local/bin/imp.new
        chmod 755 ~/.local/bin/imp.new
        ~/.local/bin/imp.new --version >/dev/null
        [ ! -f ~/.local/bin/imp ] || cp ~/.local/bin/imp ~/.local/bin/imp.prev
        mv ~/.local/bin/imp.new ~/.local/bin/imp
        echo "$(hostname): $(wc -c < ~/.local/bin/imp) bytes"' \
        < "target/$arch-unknown-linux-gnu/release/imp"
}

for host in "$@"; do
    if [ -n "$native" ]; then native_build "$host"; else cross_build "$host"; fi
done
