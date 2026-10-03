#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$ROOT/contracts/compatibility/fixtures"
MANIFEST="$FIXTURES/manifest.json"

hash_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

check_hash() {
    local file="$1" expected="$2" actual
    actual="$(hash_file "$file")"
    if [ "$actual" != "$expected" ]; then
        echo "hash mismatch for $file" >&2
        echo "expected: $expected" >&2
        echo "actual:   $actual" >&2
        exit 1
    fi
}

rustc --version | grep -q '^rustc 1\.93\.0 ' || {
    echo "Rust 1.93.0 is required" >&2
    exit 1
}
stellar --version | head -n 1 | grep -q '^stellar 28\.1\.0 ' || {
    echo "Stellar CLI 28.1.0 is required" >&2
    exit 1
}

while IFS=$'\t' read -r file expected; do
    check_hash "$FIXTURES/$file" "$expected"
done < <(jq -r '.wasm[] | [.file, .sha256] | @tsv' "$MANIFEST")

while IFS=$'\t' read -r file expected; do
    actual="$(
        stellar -q contract info meta --wasm "$FIXTURES/$file" --output json \
            | jq -er '.[] | select(.sc_meta_v0.key == "version") | .sc_meta_v0.val'
    )"
    if [ "$actual" != "$expected" ]; then
        echo "contractmeta version mismatch for $file" >&2
        echo "expected: $expected" >&2
        echo "actual:   $actual" >&2
        exit 1
    fi
done < <(jq -r '.wasm[] | [.file, .contractmeta_version] | @tsv' "$MANIFEST")

snapshot_file="$(jq -r '.snapshot.file' "$MANIFEST")"
snapshot_hash="$(jq -r '.snapshot.sha256' "$MANIFEST")"
check_hash "$FIXTURES/$snapshot_file" "$snapshot_hash"

snapshot_ledger="$(jq -r '.snapshot.ledger' "$MANIFEST")"
snapshot_timestamp="$(jq -r '.snapshot.timestamp' "$MANIFEST")"
snapshot_protocol="$(jq -r '.snapshot.protocol' "$MANIFEST")"
snapshot_network_id="$(jq -r '.snapshot.network_id' "$MANIFEST")"
events_contract="$(jq -r '.snapshot.events_contract' "$MANIFEST")"
profile_contract="$(jq -r '.snapshot.profile_contract' "$MANIFEST")"
jq -e \
    --argjson ledger "$snapshot_ledger" \
    --argjson timestamp "$snapshot_timestamp" \
    --argjson protocol "$snapshot_protocol" \
    --arg network_id "$snapshot_network_id" \
    --arg events "$events_contract" \
    --arg profile "$profile_contract" \
    '
        .sequence_number == $ledger
        and .timestamp == $timestamp
        and .protocol_version == $protocol
        and .network_id == $network_id
        and (.ledger_entries | length > 0)
        and (.ledger_entries | all(
            .live_until == 4294967295
            and (.entry.data.contract_data.contract == $events
                or .entry.data.contract_data.contract == $profile)
        ))
        and ([.ledger_entries[]
            | select(.entry.data.contract_data.key == "ledger_key_contract_instance")]
            | length == 2)
    ' "$FIXTURES/$snapshot_file" >/dev/null || {
    echo "snapshot provenance or selection invariant failed" >&2
    exit 1
}

# The replay upgrades into the release build of this tree, so build it first.
cd "$ROOT"
stellar contract build --locked
cargo test --locked -p boundless-storage-compatibility -- --include-ignored
