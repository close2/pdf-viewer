#!/usr/bin/env bash
# Compile the portable shipped surfaces and their tests under the same package selection locally
# and in CI. Native toolkit hosts are checked on Linux, where their development files exist.
set -euo pipefail

usage() {
    echo 'usage: tools/platform-check.sh [--packages] <x86_64-pc-windows-msvc|aarch64-apple-darwin>' >&2
    exit 64
}
list=false
if [ "${1:-}" = --packages ]; then list=true; shift; fi
[ "$#" -eq 1 ] || usage
target=$1
packages=(pdf-sandbox pdf-render viewer-confined viewer-ui viewer-ffi pdf-transform pdf-retrieve)
case "$target" in
    x86_64-pc-windows-msvc) ;;
    aarch64-apple-darwin) packages+=(pdf-vfs pdf-vfs-ffi) ;;
    *) usage ;;
esac
if "$list"; then
    printf '%s\n' "${packages[@]}"
    exit 0
fi
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
args=(check --all-targets --target "$target")
for package in "${packages[@]}"; do args+=(-p "$package"); done
# Encoded flags take precedence over RUSTFLAGS in Cargo. Preserve either form of caller flags,
# then append the mandatory warning policy to the form Cargo will actually use.
if [ "${CARGO_ENCODED_RUSTFLAGS+x}" ]; then
    separator=$'\x1f'
    export CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS}${CARGO_ENCODED_RUSTFLAGS:+$separator}-D${separator}warnings"
else
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-D warnings"
fi
exec cargo "${args[@]}"
