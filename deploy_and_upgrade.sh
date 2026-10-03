#!/bin/bash
#
# Testnet upgrade helper for boundless-events and boundless-profile.
#
# Usage: ./deploy_and_upgrade.sh <action> <events|profile> <arg|-> [network] [source]
#
# The network is always argument 4 (default testnet) and the source identity
# argument 5 (default: deployer_identity in deployments/<network>.json). Pass
# "-" as argument 3 when the action takes none. Contract ids come from
# deployments/<network>.json, which scripts/deploy/deploy.sh writes.
#
#   propose-upgrade <kind> <version>  build with --features testnet, upload,
#                                     propose_upgrade(wasm_hash, version)
#   apply-upgrade <kind> -            apply_upgrade(); testnet builds have no
#                                     timelock, so this can follow the proposal
#   cancel-pending-upgrade <kind> -   drop a queued proposal
#   migrate-events events -           migrate_events in pages of 8 until it
#                                     reports 0 remaining
#   migrate <kind> -                  migrate(); events refuses while
#                                     migrate_events has rows left
#   status <kind> -                   version, pending upgrade, migration marker
#
# Fresh deploys: scripts/deploy/deploy.sh. Mainnet: deploy_mainnet.sh and
# docs/upgrade-runbook.md.

set -euo pipefail

ACTION=${1:-}
CONTRACT_KIND=${2:-}
ARG3=${3:--}
NETWORK=${4:-testnet}
SOURCE_ACCOUNT=${5:-}

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
RECORD="$REPO_ROOT/deployments/$NETWORK.json"
UPGRADE_LOG="$REPO_ROOT/deployments/$NETWORK-upgrades.jsonl"

die() {
    echo -e "${RED}$*${NC}" >&2
    exit 1
}

usage() {
    sed -n '3,24p' "$0" | sed 's/^# \{0,1\}//' >&2
    exit 1
}

[ "$ACTION" != deploy ] || \
    die "deploy moved to ./scripts/deploy/deploy.sh <network>, which passes the constructor arguments"
[ -n "$ACTION" ] && [ -n "$CONTRACT_KIND" ] || usage

case "$NETWORK" in
    testnet | futurenet) ;;
    mainnet) die "mainnet is governed by the multisig: use ./deploy_mainnet.sh (docs/upgrade-runbook.md)" ;;
    *) die "network must be testnet or futurenet (got: $NETWORK)" ;;
esac

case "$CONTRACT_KIND" in
    events | profile) ;;
    *) die "unknown contract kind '$CONTRACT_KIND'; use events or profile" ;;
esac

command -v stellar >/dev/null 2>&1 || die "stellar CLI is not installed"
command -v jq >/dev/null 2>&1 || die "jq is not installed"
[ -f "$RECORD" ] || die "no deployment record at $RECORD; deploy with scripts/deploy/deploy.sh first"

CONTRACT_ID="$(jq -r --arg k "${CONTRACT_KIND}_contract" '.[$k] // empty' "$RECORD")"
[ -n "$CONTRACT_ID" ] || die "${CONTRACT_KIND}_contract missing from $RECORD"
if [ -z "$SOURCE_ACCOUNT" ]; then
    SOURCE_ACCOUNT="$(jq -r '.deployer_identity // empty' "$RECORD")"
fi
[ -n "$SOURCE_ACCOUNT" ] || die "pass the admin identity as argument 5 (no deployer_identity in $RECORD)"
WASM_PATH="$REPO_ROOT/target/wasm32v1-none/release/boundless_${CONTRACT_KIND}.wasm"

invoke() {
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        -- "$@"
}

read_call() {
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        --send no \
        -- "$@"
}

record_set() {
    local tmp
    tmp="$(mktemp)"
    jq --arg k "$1" --arg v "$2" '.[$k] = $v' "$RECORD" > "$tmp"
    mv "$tmp" "$RECORD"
}

log_op() {
    jq -nc \
        --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --arg op "$1" \
        --arg contract "$CONTRACT_KIND" \
        --arg contract_id "$CONTRACT_ID" \
        --arg version "${2:-}" \
        --arg wasm_hash "${3:-}" \
        '{ts: $ts, op: $op, contract: $contract, contract_id: $contract_id, version: $version, wasm_hash: $wasm_hash}' \
        >> "$UPGRADE_LOG"
}

propose_upgrade() {
    local new_version="$ARG3" meta_version wasm_hash
    [ "$new_version" != "-" ] || \
        die "propose-upgrade needs the new version as argument 3, e.g. ./deploy_and_upgrade.sh propose-upgrade events 2.1.0 testnet"

    echo -e "${YELLOW}Building boundless-$CONTRACT_KIND with --features testnet...${NC}"
    "$REPO_ROOT/scripts/build-release.sh" --package "boundless-$CONTRACT_KIND" --features testnet
    meta_version="$(stellar contract info meta --wasm "$WASM_PATH" --output json 2>/dev/null \
        | jq -r '[.[] | .sc_meta_v0? | select(.key == "version") | .val][0] // empty')"
    [ "$meta_version" = "$new_version" ] || \
        die "the build reports version ${meta_version:-none} in its contractmeta, not $new_version"

    wasm_hash="$(stellar contract upload \
        --source-account "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        --optimize=false \
        --wasm "$WASM_PATH")"
    echo "Uploaded wasm: $wasm_hash"

    invoke propose_upgrade --new_wasm_hash "$wasm_hash" --new_version "$new_version"
    log_op propose_upgrade "$new_version" "$wasm_hash"
    echo -e "${GREEN}Proposed $CONTRACT_KIND $new_version.${NC} Testnet builds have no timelock; run apply-upgrade next."
}

apply_upgrade() {
    local pending version wasm_hash
    pending="$(read_call get_pending_upgrade)"
    [ "$pending" != "null" ] || die "no $CONTRACT_KIND upgrade is pending"
    version="$(printf '%s' "$pending" | jq -r '.new_version')"
    wasm_hash="$(printf '%s' "$pending" | jq -r '.wasm_hash')"

    invoke apply_upgrade
    record_set "${CONTRACT_KIND}_version" "$version"
    record_set "${CONTRACT_KIND}_wasm_hash" "$wasm_hash"
    record_set upgraded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    log_op apply_upgrade "$version" "$wasm_hash"
    echo -e "${GREEN}Applied $CONTRACT_KIND $version.${NC}"
    if [ "$CONTRACT_KIND" = events ]; then
        echo "Next: migrate-events, then migrate."
    else
        echo "Next: migrate."
    fi
}

cancel_pending_upgrade() {
    invoke cancel_pending_upgrade
    log_op cancel_pending_upgrade
    echo -e "${GREEN}Pending $CONTRACT_KIND upgrade cancelled.${NC}"
}

migrate_events() {
    [ "$CONTRACT_KIND" = events ] || die "migrate-events applies to the events contract only"
    local remaining calls=0
    while :; do
        remaining="$(invoke migrate_events --max_events 8 | tr -d '"')"
        [[ "$remaining" =~ ^[0-9]+$ ]] || die "migrate_events returned '$remaining'"
        calls=$((calls + 1))
        echo "migrate_events call $calls: $remaining remaining"
        [ "$remaining" -gt 0 ] || break
    done
    log_op migrate_events "" ""
    echo -e "${GREEN}migrate_events done after $calls call(s).${NC} Run migrate next."
}

migrate() {
    local version
    invoke migrate
    version="$(read_call version | tr -d '"')"
    record_set "${CONTRACT_KIND}_migrated_to_version" "$version"
    log_op migrate "$version"
    echo -e "${GREEN}$CONTRACT_KIND migrated to $version.${NC}"
}

show_status() {
    echo -e "${YELLOW}boundless-$CONTRACT_KIND on $NETWORK: $CONTRACT_ID${NC}"
    echo "version:             $(read_call version || echo '(read failed)')"
    echo "migrated_to_version: $(read_call get_migrated_to_version || echo '(read failed)')"
    echo "pending_upgrade:     $(read_call get_pending_upgrade || echo '(read failed)')"
    echo "is_paused:           $(read_call is_paused || echo '(read failed)')"
    if [ "$CONTRACT_KIND" = events ]; then
        echo "release_validator:   $(read_call get_release_validator || echo '(read failed)')"
    fi
}

case "$ACTION" in
    propose-upgrade)        propose_upgrade ;;
    apply-upgrade)          apply_upgrade ;;
    cancel-pending-upgrade) cancel_pending_upgrade ;;
    migrate-events)         migrate_events ;;
    migrate)                migrate ;;
    status)                 show_status ;;
    *)
        echo -e "${RED}Unknown action '$ACTION'.${NC}" >&2
        usage
        ;;
esac
