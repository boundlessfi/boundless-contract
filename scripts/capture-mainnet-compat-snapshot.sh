#!/usr/bin/env bash
# Captures the mainnet storage of both contracts from the history archive, for
# the storage-compatibility replay. Read-only: it downloads archive buckets and
# never touches the network otherwise.
#
#   capture-mainnet-compat-snapshot.sh <output-json>
#       Recaptures the ledger pinned in the manifest and fails unless the result
#       is byte-identical to the reviewed fixture.
#
#   capture-mainnet-compat-snapshot.sh <output-json> <ledger>
#       Captures a new ledger (an archive checkpoint) and prints the values to
#       record in the manifest.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$ROOT/contracts/compatibility/fixtures"
MANIFEST="$FIXTURES/manifest.json"
OUTPUT="${1:-}"
NEW_LEDGER="${2:-}"

[ -n "$OUTPUT" ] || {
    echo "usage: $0 <output-json> [<ledger>]" >&2
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
# The fixture is only reproducible byte for byte with the CLI that wrote it.
stellar --version | head -n 1 | grep -q '^stellar 28\.1\.0 ' || {
    echo "Stellar CLI 28.1.0 is required" >&2
    exit 1
}

NETWORK_ID="$(jq -r '.snapshot.network_id' "$MANIFEST")"
EVENTS_ID="$(jq -r '.snapshot.events_contract' "$MANIFEST")"
PROFILE_ID="$(jq -r '.snapshot.profile_contract' "$MANIFEST")"
if [ -n "$NEW_LEDGER" ]; then
    LEDGER="$NEW_LEDGER"
else
    LEDGER="$(jq -r '.snapshot.ledger' "$MANIFEST")"
fi
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

# Keep contract data only. The deployed code is pinned separately as wasm
# fixtures, and uploading those is what makes the captured instances runnable.
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
    --arg network_id "$NETWORK_ID" \
    --arg events "$EVENTS_ID" \
    --arg profile "$PROFILE_ID" \
    '
        .sequence_number == $ledger
        and .network_id == $network_id
        and (.ledger_entries | all(
            .live_until == 4294967295
            and (.entry.data.contract_data.contract == $events
                or .entry.data.contract_data.contract == $profile)
        ))
        and ([.ledger_entries[]
            | select(.entry.data.contract_data.key == "ledger_key_contract_instance")]
            | length == 2)
    ' "$normalized_snapshot" >/dev/null || {
    echo "captured ledger does not match the expected selection" >&2
    exit 1
}

mv "$normalized_snapshot" "$OUTPUT"

if command -v sha256sum >/dev/null 2>&1; then
    actual_hash="$(sha256sum "$OUTPUT" | awk '{print $1}')"
else
    actual_hash="$(shasum -a 256 "$OUTPUT" | awk '{print $1}')"
fi

echo "snapshot:  $OUTPUT"
echo "ledger:    $(jq -r '.sequence_number' "$OUTPUT")"
echo "timestamp: $(jq -r '.timestamp' "$OUTPUT")"
echo "protocol:  $(jq -r '.protocol_version' "$OUTPUT")"
echo "entries:   $(jq -r '.ledger_entries | length' "$OUTPUT")"
echo "sha256:    $actual_hash"

if [ -z "$NEW_LEDGER" ]; then
    expected_hash="$(jq -r '.snapshot.sha256' "$MANIFEST")"
    [ "$actual_hash" = "$expected_hash" ] || {
        echo "expected:  $expected_hash" >&2
        echo "the regenerated snapshot differs from the reviewed fixture" >&2
        exit 1
    }
fi
