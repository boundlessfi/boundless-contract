#!/usr/bin/env bash
# Builds the release wasm so that the same commit and toolchain give the same
# hash on any machine.
#
# rustc embeds source paths for panic locations, which by default include the
# builder's home directory (cargo registry, rustup toolchain) and checkout path.
# Those are remapped to fixed prefixes here. The rust-src path is mapped to
# /rustc/<commit>, the form the prebuilt standard library already uses, so a
# machine with the rust-src component installed and one without agree.
#
# Usage: scripts/build-release.sh [extra `stellar contract build` args]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
sysroot="$(cd "$ROOT" && rustc --print sysroot)"
commit="$(cd "$ROOT" && rustc -vV | sed -n 's/^commit-hash: //p')"
[ -n "$commit" ] || { echo "could not read the rustc commit hash" >&2; exit 1; }

# This replaces target.wasm32v1-none.rustflags from .cargo/config.toml, so it
# must carry the same flags.
export CARGO_TARGET_WASM32V1_NONE_RUSTFLAGS="-C target-feature=+reference-types \
--remap-path-prefix=$cargo_home=/cargo \
--remap-path-prefix=$sysroot/lib/rustlib/src/rust=/rustc/$commit \
--remap-path-prefix=$ROOT=/build"

cd "$ROOT"
stellar contract build --locked "$@"

for wasm in target/wasm32v1-none/release/*.wasm; do
    if strings "$wasm" | grep -qE "$HOME|/home/|/Users/"; then
        echo "$wasm still contains a machine path" >&2
        exit 1
    fi
done
