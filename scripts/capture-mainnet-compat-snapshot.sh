#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$ROOT/contracts/compatibility/fixtures"
MANIFEST="$FIXTURES/manifest.json"
OUTPUT="${1:-}"

[ -n "$OUTPUT" ] || {
    echo "usage: $0 <output-json>" >&2
    exit 1
}
[ ! -e "$OUTPUT" ] || {
    echo "refusing to overwrite existing file: $OUTPUT" >&2
    exit 1
}

command -v stellar >/dev/null 2>&1 || {
    echo "stellar CLI not on PATH" >&2
    exit 1
}
command -v jq >/dev/null 2>&1 || {
    echo "jq not on PATH" >&2
    exit 1
}
stellar --version | head -n 1 | grep -q '^stellar 27\.0\.0 ' || {
    echo "Stellar CLI 27.0.0 is required" >&2
    exit 1
}

LEDGER="$(jq -r '.snapshot.ledger' "$MANIFEST")"
TIMESTAMP="$(jq -r '.snapshot.timestamp' "$MANIFEST")"
PROTOCOL="$(jq -r '.snapshot.protocol' "$MANIFEST")"
NETWORK_ID="$(jq -r '.snapshot.network_id' "$MANIFEST")"
EVENTS_ID="$(jq -r '.snapshot.events_contract' "$MANIFEST")"
PROFILE_ID="$(jq -r '.snapshot.profile_contract' "$MANIFEST")"
EXPECTED_HASH="$(jq -r '.snapshot.sha256' "$MANIFEST")"
ARCHIVE_URL="${STELLAR_ARCHIVE_URL:-https://history.stellar.org/prd/core-live/core_live_001/}"
NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"

raw_snapshot="$(mktemp "${TMPDIR:-/tmp}/boundless-mainnet-snapshot.XXXXXX")"
normalized_snapshot="$(mktemp "${TMPDIR:-/tmp}/boundless-mainnet-normalized.XXXXXX")"
trap 'rm -f "$raw_snapshot" "$normalized_snapshot"' EXIT

stellar snapshot create \
    --ledger "$LEDGER" \
    --output json \
    --out "$raw_snapshot" \
    --archive-url "$ARCHIVE_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    --address "$EVENTS_ID" \
    --address "$PROFILE_ID"

jq \
    --arg events "$EVENTS_ID" \
    --arg profile "$PROFILE_ID" \
    '
        .ledger_entries |= (
            map(select(
                .entry.data.contract_data.contract == $events
                or .entry.data.contract_data.contract == $profile
            ))
            | map(.live_until = 4294967295)
            | sort_by(
                .entry.data.contract_data.contract,
                (.entry.data.contract_data.key | tojson)
            )
        )
    ' "$raw_snapshot" > "$normalized_snapshot"

jq -e \
    --argjson ledger "$LEDGER" \
    --argjson timestamp "$TIMESTAMP" \
    --argjson protocol "$PROTOCOL" \
    --arg network_id "$NETWORK_ID" \
    --arg events "$EVENTS_ID" \
    --arg profile "$PROFILE_ID" \
    '
        .sequence_number == $ledger
        and .timestamp == $timestamp
        and .protocol_version == $protocol
        and .network_id == $network_id
        and (.ledger_entries | length == 10)
        and (.ledger_entries | all(
            .live_until == 4294967295
            and (.entry.data.contract_data.contract == $events
                or .entry.data.contract_data.contract == $profile)
        ))
        and ([.ledger_entries[]
            | select(.entry.data.contract_data.key == "ledger_key_contract_instance")]
            | length == 2)
    ' "$normalized_snapshot" >/dev/null || {
    echo "captured ledger does not match the pinned snapshot selection" >&2
    exit 1
}

mv "$normalized_snapshot" "$OUTPUT"

if command -v sha256sum >/dev/null 2>&1; then
    actual_hash="$(sha256sum "$OUTPUT" | awk '{print $1}')"
else
    actual_hash="$(shasum -a 256 "$OUTPUT" | awk '{print $1}')"
fi

echo "snapshot: $OUTPUT"
echo "sha256:   $actual_hash"
[ "$actual_hash" = "$EXPECTED_HASH" ] || {
    echo "expected: $EXPECTED_HASH" >&2
    echo "the regenerated snapshot differs from the reviewed fixture" >&2
    exit 1
}
