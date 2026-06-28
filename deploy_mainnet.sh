#!/bin/bash
#
# Boundless mainnet deploy script.
#
# This is a separate script from deploy_and_upgrade.sh because mainnet deploys
# need stronger guards (explicit confirmation, env var validation, paired
# deploy of profile + events, multi-sig rotation flow) than the testnet
# scripted path can carry safely.
#
# Companion: boundless-contract/docs/mainnet-deploy-runbook.md
#
# Usage:
#   ./deploy_mainnet.sh deploy-profile
#   ./deploy_mainnet.sh deploy-events
#   ./deploy_mainnet.sh register-token <token-sac-address>
#   ./deploy_mainnet.sh rotate-admin <new-multisig-address>
#   ./deploy_mainnet.sh upgrade-events <wasm-path>
#   ./deploy_mainnet.sh verify

set -euo pipefail

ACTION="${1:-}"
shift || true

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

NETWORK="mainnet"
NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
DEPLOYMENTS_DIR="$(cd "$(dirname "$0")" && pwd)/deployments"
DEPLOYMENT_FILE="$DEPLOYMENTS_DIR/mainnet.json"

err() {
    echo -e "${RED}error: $*${NC}" >&2
    exit 1
}

info() {
    echo -e "${YELLOW}$*${NC}"
}

ok() {
    echo -e "${GREEN}$*${NC}"
}

confirm_mainnet() {
    echo -e "${BOLD}You are about to operate on Stellar MAINNET.${NC}"
    echo "Network passphrase: $NETWORK_PASSPHRASE"
    echo ""
    read -rp "Type 'mainnet' to continue: " ack
    if [ "$ack" != "mainnet" ]; then
        err "aborted"
    fi
}

require_env() {
    local var="$1"
    if [ -z "${!var:-}" ]; then
        err "missing required env var: $var"
    fi
}

require_cli() {
    command -v stellar >/dev/null 2>&1 || err "stellar CLI not on PATH"
    command -v jq      >/dev/null 2>&1 || err "jq not on PATH"
    command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || \
        err "sha256sum or shasum not on PATH"
}

hash_wasm() {
    local f="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$f" | cut -d' ' -f1
    else
        shasum -a 256 "$f" | cut -d' ' -f1
    fi
}

ensure_deployments_dir() {
    mkdir -p "$DEPLOYMENTS_DIR"
    if [ ! -f "$DEPLOYMENT_FILE" ]; then
        echo "{\"network\":\"$NETWORK\",\"passphrase\":\"$NETWORK_PASSPHRASE\"}" > "$DEPLOYMENT_FILE"
    fi
}

deployment_get() {
    local key="$1"
    jq -r ".\"$key\" // empty" "$DEPLOYMENT_FILE"
}

deployment_set() {
    local key="$1" val="$2"
    local tmp
    tmp="$(mktemp)"
    jq ".\"$key\" = \"$val\"" "$DEPLOYMENT_FILE" > "$tmp"
    mv "$tmp" "$DEPLOYMENT_FILE"
}

deployment_set_raw() {
    local key="$1" val="$2"
    local tmp
    tmp="$(mktemp)"
    jq ".\"$key\" = $val" "$DEPLOYMENT_FILE" > "$tmp"
    mv "$tmp" "$DEPLOYMENT_FILE"
}

cmd_build_release() {
    info "Building contracts in release mode..."
    stellar contract build
    ok "Build complete."
}

cmd_deploy_profile() {
    require_env INITIAL_ADMIN_KEY
    require_env BOOTSTRAP_PROFILE_CREDITS
    require_cli
    confirm_mainnet
    ensure_deployments_dir

    if [ -n "$(deployment_get profile_contract)" ]; then
        err "profile contract already deployed: $(deployment_get profile_contract). Refusing to re-deploy."
    fi

    cmd_build_release

    local wasm="target/wasm32v1-none/release/boundless_profile.wasm"
    [ -f "$wasm" ] || err "missing wasm: $wasm"

    info "Deploying boundless-profile..."
    local profile_id
    profile_id=$(stellar contract deploy \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --wasm "$wasm" \
        -- \
        --admin "$(stellar keys address "$INITIAL_ADMIN_KEY")" \
        --default_bootstrap_credits "$BOOTSTRAP_PROFILE_CREDITS")

    ok "profile_contract=$profile_id"
    deployment_set profile_contract "$profile_id"
    deployment_set profile_wasm_hash "$(hash_wasm "$wasm")"
    deployment_set deployed_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}

cmd_deploy_events() {
    require_env INITIAL_ADMIN_KEY
    require_env FEE_ACCOUNT
    require_env INITIAL_GLOBAL_FEE_BPS
    require_cli
    confirm_mainnet
    ensure_deployments_dir

    local profile_id
    profile_id="$(deployment_get profile_contract)"
    [ -n "$profile_id" ] || err "deploy-profile first; profile_contract not set"

    if [ -n "$(deployment_get events_contract)" ]; then
        err "events contract already deployed: $(deployment_get events_contract). Refusing to re-deploy. Use upgrade-events instead."
    fi

    cmd_build_release

    local wasm="target/wasm32v1-none/release/boundless_events.wasm"
    [ -f "$wasm" ] || err "missing wasm: $wasm"

    info "Deploying boundless-events..."
    local events_id
    events_id=$(stellar contract deploy \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --wasm "$wasm" \
        -- \
        --admin "$(stellar keys address "$INITIAL_ADMIN_KEY")" \
        --fee_account "$FEE_ACCOUNT" \
        --fee_bps "$INITIAL_GLOBAL_FEE_BPS" \
        --profile_contract "$profile_id")

    ok "events_contract=$events_id"
    deployment_set events_contract "$events_id"
    deployment_set events_wasm_hash "$(hash_wasm "$wasm")"
    deployment_set fee_account "$FEE_ACCOUNT"
    deployment_set_raw initial_fee_bps "$INITIAL_GLOBAL_FEE_BPS"

    info "Wiring profile -> events..."
    stellar contract invoke \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --id "$profile_id" \
        -- set_events_contract \
        --new_addr "$events_id"
    ok "profile.events_contract = $events_id"
}

cmd_register_token() {
    local token="${1:-}"
    [ -n "$token" ] || err "usage: register-token <token-sac-address>"
    require_env INITIAL_ADMIN_KEY
    require_cli
    confirm_mainnet

    local events_id
    events_id="$(deployment_get events_contract)"
    [ -n "$events_id" ] || err "events contract not deployed"

    info "Registering token $token..."
    stellar contract invoke \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --id "$events_id" \
        -- register_supported_token \
        --token "$token"

    local supported
    supported=$(stellar contract invoke \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --id "$events_id" \
        -- is_supported_token \
        --token "$token")
    [ "$supported" = "true" ] || err "token registration not confirmed; got: $supported"

    ok "Token registered: $token"

    # Append to registered_tokens array.
    local tmp
    tmp="$(mktemp)"
    jq ".registered_tokens = ((.registered_tokens // []) + [\"$token\"] | unique)" \
        "$DEPLOYMENT_FILE" > "$tmp"
    mv "$tmp" "$DEPLOYMENT_FILE"
}

cmd_rotate_admin() {
    local new_admin="${1:-}"
    [ -n "$new_admin" ] || err "usage: rotate-admin <new-multisig-address>"
    require_env INITIAL_ADMIN_KEY
    require_cli
    confirm_mainnet

    local events_id profile_id
    events_id="$(deployment_get events_contract)"
    profile_id="$(deployment_get profile_contract)"
    [ -n "$events_id" ]  || err "events contract not deployed"
    [ -n "$profile_id" ] || err "profile contract not deployed"

    info "Setting pending admin on events..."
    stellar contract invoke \
        --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$events_id" \
        -- set_admin --new_admin "$new_admin"

    info "Setting pending admin on profile..."
    stellar contract invoke \
        --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$profile_id" \
        -- set_admin --new_admin "$new_admin"

    echo ""
    echo -e "${BOLD}Now the new admin ($new_admin) must call accept_admin on both contracts.${NC}"
    echo "This step requires the multi-sig quorum to sign. After both accept ops"
    echo "land, run 'verify' to confirm get_admin returns the new address."
}

cmd_upgrade_events() {
    local wasm="${1:-}"
    [ -n "$wasm" ] || err "usage: upgrade-events <path-to-new-wasm>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    require_env ADMIN_SOURCE
    require_cli
    confirm_mainnet

    local events_id
    events_id="$(deployment_get events_contract)"
    [ -n "$events_id" ] || err "events contract not deployed"

    info "Uploading new wasm..."
    local wasm_hash
    wasm_hash=$(stellar contract upload \
        --source-account "$ADMIN_SOURCE" \
        --network "$NETWORK" \
        --wasm "$wasm")

    info "Upgrading events contract to $wasm_hash..."
    stellar contract invoke \
        --network "$NETWORK" \
        --source "$ADMIN_SOURCE" \
        --id "$events_id" \
        -- upgrade \
        --new_wasm_hash "$wasm_hash"

    # Append to upgrade log.
    local upgrades_log="$DEPLOYMENTS_DIR/mainnet-upgrades.jsonl"
    echo "{\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\",\"wasm_hash\":\"$wasm_hash\",\"wasm_path\":\"$wasm\"}" \
        >> "$upgrades_log"

    deployment_set events_wasm_hash "$(hash_wasm "$wasm")"
    ok "Events contract upgraded to $wasm_hash."
}

cmd_verify() {
    require_env INITIAL_ADMIN_KEY  # for the read-source; any key works for invoke-as-read
    require_cli

    local events_id profile_id
    events_id="$(deployment_get events_contract)"
    profile_id="$(deployment_get profile_contract)"
    [ -n "$events_id" ]  || err "events contract not in deployment file"
    [ -n "$profile_id" ] || err "profile contract not in deployment file"

    info "Verifying mainnet state..."

    echo "events.get_admin:"
    stellar contract invoke --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$events_id" -- get_admin

    echo "events.get_fee_bps:"
    stellar contract invoke --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$events_id" -- get_fee_bps

    echo "events.get_fee_account:"
    stellar contract invoke --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$events_id" -- get_fee_account

    echo "events.is_paused:"
    stellar contract invoke --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$events_id" -- is_paused

    echo "profile.get_admin:"
    stellar contract invoke --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$profile_id" -- get_admin

    ok "Verification reads complete. Compare against the runbook expectations."
}

case "$ACTION" in
    "deploy-profile")
        cmd_deploy_profile
        ;;
    "deploy-events")
        cmd_deploy_events
        ;;
    "register-token")
        cmd_register_token "$@"
        ;;
    "rotate-admin")
        cmd_rotate_admin "$@"
        ;;
    "upgrade-events")
        cmd_upgrade_events "$@"
        ;;
    "verify")
        cmd_verify
        ;;
    "")
        err "no action specified. See mainnet-deploy-runbook.md for usage."
        ;;
    *)
        err "unknown action: $ACTION. See mainnet-deploy-runbook.md for usage."
        ;;
esac
