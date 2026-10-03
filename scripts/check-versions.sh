#!/usr/bin/env bash
#
# check-versions.sh: each contract must report one version everywhere.
#
# The deployed wasm carries contractmeta "version", the constructor stamps
# INITIAL_VERSION, and Cargo.toml names the crate version. When they drift,
# an upgrade proposal can name a version the code does not report.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FAIL=0

for contract in events profile; do
    dir="$ROOT/contracts/$contract"
    cargo=$(sed -nE 's/^version = "([^"]+)"/\1/p' "$dir/Cargo.toml" | head -1)
    initial=$(sed -nE 's/.*INITIAL_VERSION: &str = "([^"]+)".*/\1/p' "$dir/src/admin.rs" | head -1)
    meta=$(sed -nE 's/.*contractmeta!\(key = "version", val = "([^"]+)"\).*/\1/p' "$dir/src/lib.rs" | head -1)

    echo "$contract: Cargo.toml=$cargo INITIAL_VERSION=$initial contractmeta=${meta:-none}"
    if [[ -z "$cargo" || "$cargo" != "$initial" ]]; then
        echo "::error::$contract Cargo.toml ($cargo) and INITIAL_VERSION ($initial) differ" >&2
        FAIL=1
    fi
    if [[ -n "$meta" && "$meta" != "$initial" ]]; then
        echo "::error::$contract contractmeta ($meta) and INITIAL_VERSION ($initial) differ" >&2
        FAIL=1
    fi
done

exit "$FAIL"
