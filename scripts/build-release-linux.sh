#!/usr/bin/env bash
# Builds the release wasm in the Linux x86-64 environment CI uses, so anyone can
# reproduce CI's hash for a commit from any machine. This is the build that
# goes on mainnet.
#
# A native build on another host (macOS, arm64) produces the same code in a
# different function order, and so a different hash: cargo folds the host
# triple into the hashes behind symbol names, and the linker orders by them.
#
# Usage: scripts/build-release-linux.sh [extra `stellar contract build` args]
# Output: target/release-linux/*.wasm
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/release-linux"
IMAGE="rust:1.93.0@sha256:bbde3ca426faeebab3eb58b8005d3d6da70607817a14bb92a55c94611d4d905f"
CLI_VERSION="28.1.0"
CLI_SHA256="c1680deee94301d33ada7a17f98411e642a4248c727afbd2e43050d345746462"

command -v docker >/dev/null 2>&1 || { echo "docker not on PATH" >&2; exit 1; }
if [ -n "$(git -C "$ROOT" status --porcelain 2>/dev/null)" ]; then
    echo "warning: the working tree has uncommitted changes; they are built too" >&2
fi
echo "building $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo 'unknown commit') in $IMAGE"

rm -rf "$OUT" && mkdir -p "$OUT"
docker run --rm --platform linux/amd64 \
    -v "$ROOT":/src:ro -v "$OUT":/out \
    -e CLI_VERSION="$CLI_VERSION" -e CLI_SHA256="$CLI_SHA256" -e DEBIAN_FRONTEND=noninteractive \
    "$IMAGE" bash -euo pipefail -c '
        apt-get update -qq >/dev/null
        apt-get install -y -qq binutils libdbus-1-3 >/dev/null
        tar="stellar-cli-${CLI_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
        curl -fsSL -o "/tmp/$tar" \
            "https://github.com/stellar/stellar-cli/releases/download/v${CLI_VERSION}/$tar"
        echo "${CLI_SHA256}  /tmp/$tar" | sha256sum -c - >/dev/null
        tar -xzf "/tmp/$tar" -C /usr/local/bin stellar
        rustup target add wasm32v1-none >/dev/null 2>&1
        mkdir /build
        tar -C /src --exclude=./target --exclude=./.git -cf - . | tar -C /build -xf -
        cd /build
        ./scripts/build-release.sh "$@" >/tmp/build.log 2>&1 || { cat /tmp/build.log; exit 1; }
        cp target/wasm32v1-none/release/*.wasm /out/
    ' bash "$@"

for wasm in "$OUT"/*.wasm; do
    echo "$(shasum -a 256 "$wasm" | awk "{print \$1}")  ${wasm#"$ROOT"/}"
done
