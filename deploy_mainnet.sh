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
#   ./deploy_mainnet.sh upload-profile-wasm <wasm-path>
#   ./deploy_mainnet.sh upload-events-wasm <wasm-path>
#   ./deploy_mainnet.sh prepare-pause-profile <xdr-output>
#   ./deploy_mainnet.sh prepare-pause-events <xdr-output>
#   ./deploy_mainnet.sh prepare-propose-upgrade-profile <wasm-path> <new-version> <xdr-output>
#   ./deploy_mainnet.sh prepare-propose-upgrade-events <wasm-path> <new-version> <xdr-output>
#   ./deploy_mainnet.sh verify-proposed-upgrade-profile <wasm-path> <new-version>
#   ./deploy_mainnet.sh verify-proposed-upgrade-events <wasm-path> <new-version>
#   ./deploy_mainnet.sh prepare-apply-upgrade-profile <wasm-path> <expected-version> <xdr-output>
#   ./deploy_mainnet.sh prepare-apply-upgrade-events <wasm-path> <expected-version> <xdr-output>
#   ./deploy_mainnet.sh prepare-migrate-upgrade-profile <expected-version> <xdr-output>
#   ./deploy_mainnet.sh prepare-migrate-upgrade-events <expected-version> <xdr-output>
#   ./deploy_mainnet.sh prepare-cancel-upgrade-profile <xdr-output>
#   ./deploy_mainnet.sh prepare-cancel-upgrade-events <xdr-output>
#   ./deploy_mainnet.sh prepare-unpause-profile <expected-version> <xdr-output>
#   ./deploy_mainnet.sh prepare-unpause-events <expected-version> <xdr-output>
#   ./deploy_mainnet.sh verify-upgrade-profile <wasm-path> <expected-version>
#   ./deploy_mainnet.sh verify-upgrade-events <wasm-path> <expected-version>
#   ./deploy_mainnet.sh upgrade-status
#   ./deploy_mainnet.sh verify

set -euo pipefail

ACTION="${1:-}"
shift || true

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

NETWORK="${STELLAR_NETWORK:-}"
NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
REQUIRED_STELLAR_CLI_VERSION="27.0.0"
SDK27_EVENTS_VERSION="1.5.0"
SDK27_EVENTS_WASM_HASH="52ead9e4374d1fe17704649da59747095bb59839f3e826978013da03508a7e7c"
SDK27_PROFILE_VERSION="1.2.0"
SDK27_PROFILE_WASM_HASH="de2e90763462c67844bbf99707f67056f3aaa039d763c32366c445b73cf10802"
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
    echo "CLI network config: $NETWORK"
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
    [ -n "$NETWORK" ] || \
        err "set STELLAR_NETWORK to the configured mainnet network name"
    command -v stellar >/dev/null 2>&1 || err "stellar CLI not on PATH"
    command -v jq      >/dev/null 2>&1 || err "jq not on PATH"
    command -v awk     >/dev/null 2>&1 || err "awk not on PATH"
    command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || \
        err "sha256sum or shasum not on PATH"
    local stellar_cli_version
    stellar_cli_version="$(stellar --version | awk 'NR == 1 { print $2 }')"
    [ "$stellar_cli_version" = "$REQUIRED_STELLAR_CLI_VERSION" ] || \
        err "stellar CLI version mismatch: expected $REQUIRED_STELLAR_CLI_VERSION, got ${stellar_cli_version:-unknown}"
    if [ -n "${STELLAR_RPC_URL:-}" ] || [ -n "${STELLAR_NETWORK_PASSPHRASE:-}" ]; then
        err "unset STELLAR_RPC_URL and STELLAR_NETWORK_PASSPHRASE when using --network $NETWORK"
    fi

    local networks configured_rpc configured_passphrase
    networks="$(stellar network ls --long)"
    configured_rpc="$(printf '%s\n' "$networks" | awk -v target="$NETWORK" '
        $0 == "Name: " target { found = 1; next }
        found && /^RPC url: / { sub(/^RPC url: /, ""); print; exit }
        found && /^Name: / { exit }
    ')"
    configured_passphrase="$(printf '%s\n' "$networks" | awk -v target="$NETWORK" '
        $0 == "Name: " target { found = 1; next }
        found && /^Network passphrase: / {
            sub(/^Network passphrase: /, ""); print; exit
        }
        found && /^Name: / { exit }
    ')"
    [ "$configured_passphrase" = "$NETWORK_PASSPHRASE" ] || \
        err "network $NETWORK is not configured with the Stellar public-network passphrase"
    [ -n "$configured_rpc" ] && [[ "$configured_rpc" != "Bring Your Own:"* ]] || \
        err "network $NETWORK does not have a usable RPC URL"
}

hash_wasm() {
    local f="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$f" | cut -d' ' -f1
    else
        shasum -a 256 "$f" | cut -d' ' -f1
    fi
}

deployed_wasm_hash() {
    local contract_id="$1"
    stellar contract info hash \
        --network "$NETWORK" \
        --contract-id "$contract_id"
}

assert_local_wasm_hash() {
    local label="$1" wasm="$2" expected="$3"
    local actual
    actual="$(hash_wasm "$wasm")"
    [ "$actual" = "$expected" ] || \
        err "$label artifact hash mismatch: expected $expected, got $actual"
}

assert_deployed_wasm_hash() {
    local label="$1" contract_id="$2" expected="$3"
    local actual
    actual="$(deployed_wasm_hash "$contract_id")" || \
        err "could not read deployed $label WASM hash"
    [ "$actual" = "$expected" ] || \
        err "deployed $label WASM hash mismatch: expected $expected, got $actual"
}

assert_sdk27_profile_artifact() {
    local wasm="$1" version="$2"
    if [ "$version" = "$SDK27_PROFILE_VERSION" ]; then
        assert_local_wasm_hash "profile SDK 27" "$wasm" "$SDK27_PROFILE_WASM_HASH"
    fi
}

assert_sdk27_events_artifact() {
    local wasm="$1" version="$2"
    if [ "$version" = "$SDK27_EVENTS_VERSION" ]; then
        assert_local_wasm_hash "events SDK 27" "$wasm" "$SDK27_EVENTS_WASM_HASH"
    fi
}

assert_profile_sdk27_wasm_live() {
    assert_deployed_wasm_hash "profile SDK 27" "$1" "$SDK27_PROFILE_WASM_HASH"
}

assert_events_sdk27_wasm_live() {
    assert_deployed_wasm_hash "events SDK 27" "$1" "$SDK27_EVENTS_WASM_HASH"
}

assert_version_argument() {
    local label="$1" actual="$2" expected="$3"
    [ "$actual" = "$expected" ] || \
        err "$label version must be $expected for this pinned mainnet flow; got $actual"
}

assert_unpause_version_argument() {
    local label="$1" actual="$2" upgraded="$3"
    case "$actual" in
        "1.1.0"|"$upgraded") ;;
        *) err "$label unpause version must be 1.1.0 for pre-apply recovery or $upgraded after upgrade; got $actual" ;;
    esac
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

events_contract_id() {
    local events_id="${EVENTS_ID:-}"
    if [ -z "$events_id" ] && [ -f "$DEPLOYMENT_FILE" ]; then
        events_id="$(deployment_get events_contract)"
    fi
    [ -n "$events_id" ] || \
        err "events contract id missing; set EVENTS_ID or restore deployments/mainnet.json"
    printf '%s\n' "$events_id"
}

profile_contract_id() {
    local profile_id="${PROFILE_ID:-}"
    if [ -z "$profile_id" ] && [ -f "$DEPLOYMENT_FILE" ]; then
        profile_id="$(deployment_get profile_contract)"
    fi
    [ -n "$profile_id" ] || \
        err "profile contract id missing; set PROFILE_ID or restore deployments/mainnet.json"
    printf '%s\n' "$profile_id"
}

events_read() {
    local events_id="$1"
    shift
    stellar contract invoke \
        --send no \
        --network "$NETWORK" \
        --source "$ADMIN_SOURCE" \
        --id "$events_id" \
        -- "$@"
}

profile_read() {
    local profile_id="$1"
    shift
    stellar contract invoke \
        --send no \
        --network "$NETWORK" \
        --source "$ADMIN_SOURCE" \
        --id "$profile_id" \
        -- "$@"
}

read_json_string() {
    jq -er 'if type == "string" then . else error("expected JSON string") end'
}

assert_events_version() {
    local events_id="$1" expected="$2"
    local actual
    actual="$(events_read "$events_id" version | read_json_string)" || \
        err "could not read the current events contract version"
    [ "$actual" = "$expected" ] || \
        err "events version mismatch: expected $expected, got $actual"
}

assert_profile_version() {
    local profile_id="$1" expected="$2"
    local actual
    actual="$(profile_read "$profile_id" version | read_json_string)" || \
        err "could not read the current profile contract version"
    [ "$actual" = "$expected" ] || \
        err "profile version mismatch: expected $expected, got $actual"
}

assert_admin_source_matches() {
    local events_id="$1"
    local actual_admin
    actual_admin="$(events_read "$events_id" get_admin | read_json_string)" || \
        err "could not read the current events admin"
    [ "$actual_admin" = "$ADMIN_SOURCE" ] || \
        err "ADMIN_SOURCE mismatch: contract admin is $actual_admin, got $ADMIN_SOURCE"
}

assert_profile_admin_source_matches() {
    local profile_id="$1"
    local actual_admin
    actual_admin="$(profile_read "$profile_id" get_admin | read_json_string)" || \
        err "could not read the current profile admin"
    [ "$actual_admin" = "$ADMIN_SOURCE" ] || \
        err "ADMIN_SOURCE mismatch: profile contract admin is $actual_admin, got $ADMIN_SOURCE"
}

assert_profile_events_binding() {
    local profile_id="$1" expected_events_id="$2"
    local actual_events_id
    actual_events_id="$(profile_read "$profile_id" get_events_contract | read_json_string)" || \
        err "could not read profile.get_events_contract"
    [ "$actual_events_id" = "$expected_events_id" ] || \
        err "profile events-contract binding changed: expected $expected_events_id, got $actual_events_id"
}

assert_no_pending_events_contract_rotation() {
    local profile_id="$1"
    local pending
    pending="$(profile_read "$profile_id" get_pending_events_contract)" || \
        err "could not read profile.get_pending_events_contract"
    [ "$pending" = "null" ] || \
        err "a profile events-contract rotation is pending; cancel or complete it before the upgrade"
}

assert_upgrade_governance_stable() {
    local profile_id="$1" events_id="$2"
    assert_admin_source_matches "$events_id"
    assert_profile_admin_source_matches "$profile_id"
    assert_profile_events_binding "$profile_id" "$events_id"
    assert_no_pending_events_contract_rotation "$profile_id"
}

assert_events_paused() {
    local events_id="$1"
    local paused
    paused="$(events_read "$events_id" is_paused)" || \
        err "could not read events.is_paused"
    [ "$paused" = "true" ] || \
        err "events contract is not paused; refusing to rely on a zero-event snapshot"
}

assert_events_unpaused() {
    local events_id="$1"
    local paused
    paused="$(events_read "$events_id" is_paused)" || \
        err "could not read events.is_paused"
    [ "$paused" = "false" ] || \
        err "events contract is still paused after unpause"
}

assert_profile_paused() {
    local profile_id="$1"
    local paused
    paused="$(profile_read "$profile_id" is_paused)" || \
        err "could not read profile.is_paused"
    [ "$paused" = "true" ] || \
        err "profile contract is not paused"
}

assert_profile_unpaused() {
    local profile_id="$1"
    local paused
    paused="$(profile_read "$profile_id" is_paused)" || \
        err "could not read profile.is_paused"
    [ "$paused" = "false" ] || \
        err "profile contract is still paused after unpause"
}

assert_no_pending_events_upgrade() {
    local events_id="$1"
    local pending
    pending="$(events_read "$events_id" get_pending_upgrade)" || \
        err "could not read pending events upgrade"
    [ "$pending" = "null" ] || \
        err "an events upgrade is already pending; inspect it with upgrade-status"
}

assert_pending_events_upgrade() {
    local events_id="$1" expected_version="$2" expected_hash="$3"
    local pending actual_version actual_hash
    pending="$(events_read "$events_id" get_pending_upgrade)" || \
        err "could not read pending events upgrade"
    [ "$pending" != "null" ] || err "no events upgrade is pending"
    actual_version="$(printf '%s' "$pending" | jq -er '.new_version')" || \
        err "pending events upgrade did not contain new_version"
    actual_hash="$(printf '%s' "$pending" | jq -er '.wasm_hash')" || \
        err "pending events upgrade did not contain wasm_hash"
    [ "$actual_version" = "$expected_version" ] || \
        err "pending version mismatch: expected $expected_version, got $actual_version"
    [ "$actual_hash" = "$expected_hash" ] || \
        err "pending wasm mismatch: expected $expected_hash, got $actual_hash"
}

assert_no_pending_profile_upgrade() {
    local profile_id="$1"
    local pending
    pending="$(profile_read "$profile_id" get_pending_upgrade)" || \
        err "could not read pending profile upgrade"
    [ "$pending" = "null" ] || \
        err "a profile upgrade is already pending; inspect it with upgrade-status"
}

assert_pending_profile_upgrade() {
    local profile_id="$1" expected_version="$2" expected_hash="$3"
    local pending actual_version actual_hash
    pending="$(profile_read "$profile_id" get_pending_upgrade)" || \
        err "could not read pending profile upgrade"
    [ "$pending" != "null" ] || err "no profile upgrade is pending"
    actual_version="$(printf '%s' "$pending" | jq -er '.new_version')" || \
        err "pending profile upgrade did not contain new_version"
    actual_hash="$(printf '%s' "$pending" | jq -er '.wasm_hash')" || \
        err "pending profile upgrade did not contain wasm_hash"
    [ "$actual_version" = "$expected_version" ] || \
        err "pending profile version mismatch: expected $expected_version, got $actual_version"
    [ "$actual_hash" = "$expected_hash" ] || \
        err "pending profile wasm mismatch: expected $expected_hash, got $actual_hash"
}

assert_pending_events_version() {
    local events_id="$1" expected_version="$2"
    local pending actual_version
    pending="$(events_read "$events_id" get_pending_upgrade)" || \
        err "could not read pending events upgrade"
    [ "$pending" != "null" ] || err "no events upgrade is pending"
    actual_version="$(printf '%s' "$pending" | jq -er '.new_version')" || \
        err "pending events upgrade did not contain new_version"
    [ "$actual_version" = "$expected_version" ] || \
        err "pending events version mismatch: expected $expected_version, got $actual_version"
}

assert_profile_sdk27_staged_or_applied() {
    local profile_id="$1"
    local version pending
    version="$(profile_read "$profile_id" version | read_json_string)" || \
        err "could not read the current profile contract version"
    pending="$(profile_read "$profile_id" get_pending_upgrade)" || \
        err "could not read pending profile upgrade"
    assert_profile_paused "$profile_id"

    if [ "$version" = "1.1.0" ]; then
        assert_pending_profile_upgrade \
            "$profile_id" "$SDK27_PROFILE_VERSION" "$SDK27_PROFILE_WASM_HASH"
    elif [ "$version" = "1.2.0" ]; then
        [ "$pending" = "null" ] || \
            err "profile is 1.2.0 but still has a pending upgrade"
        assert_profile_sdk27_wasm_live "$profile_id"
    else
        err "profile must be staged at 1.1.0 or applied at 1.2.0; got $version"
    fi
}

assert_profile_sdk27_applied_paused() {
    local profile_id="$1"
    assert_profile_version "$profile_id" "1.2.0"
    assert_profile_sdk27_wasm_live "$profile_id"
    assert_profile_paused "$profile_id"
    assert_no_pending_profile_upgrade "$profile_id"
}

assert_profile_sdk27_migrated_paused() {
    local profile_id="$1"
    assert_profile_sdk27_applied_paused "$profile_id"
    assert_profile_migrated_version "$profile_id" "1.2.0"
}

assert_profile_sdk27_ready() {
    local profile_id="$1"
    assert_profile_version "$profile_id" "1.2.0"
    assert_profile_sdk27_wasm_live "$profile_id"
    assert_profile_migrated_version "$profile_id" "1.2.0"
    assert_no_pending_profile_upgrade "$profile_id"
    assert_profile_unpaused "$profile_id"
}

assert_no_events_exist() {
    local events_id="$1"
    local base first_event_id output status

    base="$(events_read "$events_id" id_base)" || \
        err "could not read events.id_base"
    [[ "$base" =~ ^[0-9]+$ ]] || \
        err "events.id_base returned a non-numeric value: $base"
    first_event_id=$((base + 1))

    set +e
    output="$(events_read "$events_id" get_event --event_id "$first_event_id" 2>&1)"
    status=$?
    set -e

    if [ "$status" -eq 0 ]; then
        err "event $first_event_id exists; abort the upgrade and implement a legacy-total migration"
    fi
    if [[ "$output" != *"Error(Contract, #30)"* ]]; then
        err "could not prove event $first_event_id is absent; refusing to continue: $output"
    fi

    ok "Zero-event guard confirmed: $first_event_id returned EventNotFound."
}

is_zero_event_guarded_target() {
    [ "$1" = "$SDK27_EVENTS_VERSION" ]
}

assert_migrated_version() {
    local events_id="$1" expected="$2"
    local migrated_version
    migrated_version="$(events_read "$events_id" get_migrated_to_version | read_json_string)" || \
        err "could not read migrated-to version"
    [ "$migrated_version" = "$expected" ] || \
        err "migration marker mismatch: expected $expected, got $migrated_version"
}

assert_profile_migrated_version() {
    local profile_id="$1" expected="$2"
    local migrated_version
    migrated_version="$(profile_read "$profile_id" get_migrated_to_version | read_json_string)" || \
        err "could not read profile migrated-to version"
    [ "$migrated_version" = "$expected" ] || \
        err "profile migration marker mismatch: expected $expected, got $migrated_version"
}

prepare_events_invoke() {
    local events_id="$1" output_file="$2"
    shift 2
    local output_dir
    output_dir="$(dirname "$output_file")"
    [ -d "$output_dir" ] || err "XDR output directory does not exist: $output_dir"
    assert_admin_source_matches "$events_id"

    local tmp
    tmp="$(mktemp)"
    if ! stellar contract invoke \
        --network "$NETWORK" \
        --source-account "$ADMIN_SOURCE" \
        --id "$events_id" \
        --build-only \
        -- "$@" \
        | stellar tx simulate \
            --network "$NETWORK" \
            --source-account "$ADMIN_SOURCE" \
            > "$tmp"; then
        rm -f "$tmp"
        err "could not build and simulate the multisig transaction"
    fi
    [ -s "$tmp" ] || {
        rm -f "$tmp"
        err "prepared transaction was empty"
    }
    mv "$tmp" "$output_file"
    ok "Prepared simulated XDR: $output_file"
    echo "Sign it sequentially with the required multi-sig quorum, then submit with:"
    echo "  stellar tx send \"<signed-xdr>\" --network $NETWORK"
}

prepare_profile_invoke() {
    local profile_id="$1" output_file="$2"
    shift 2
    local output_dir
    output_dir="$(dirname "$output_file")"
    [ -d "$output_dir" ] || err "XDR output directory does not exist: $output_dir"
    assert_profile_admin_source_matches "$profile_id"

    local tmp
    tmp="$(mktemp)"
    if ! stellar contract invoke \
        --network "$NETWORK" \
        --source-account "$ADMIN_SOURCE" \
        --id "$profile_id" \
        --build-only \
        -- "$@" \
        | stellar tx simulate \
            --network "$NETWORK" \
            --source-account "$ADMIN_SOURCE" \
            > "$tmp"; then
        rm -f "$tmp"
        err "could not build and simulate the profile multisig transaction"
    fi
    [ -s "$tmp" ] || {
        rm -f "$tmp"
        err "prepared profile transaction was empty"
    }
    mv "$tmp" "$output_file"
    ok "Prepared simulated XDR: $output_file"
    echo "Sign it sequentially with the required multi-sig quorum, then submit with:"
    echo "  stellar tx send \"<signed-xdr>\" --network $NETWORK"
}

append_upgrade_log() {
    local action="$1" version="$2" wasm_hash="${3:-}"
    local upgrades_log="$DEPLOYMENTS_DIR/mainnet-upgrades.jsonl"
    jq -nc \
        --arg timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --arg action "$action" \
        --arg version "$version" \
        --arg wasm_hash "$wasm_hash" \
        '{timestamp: $timestamp, action: $action, version: $version, wasm_hash: $wasm_hash}' \
        >> "$upgrades_log"
}

cmd_build_release() {
    info "Building contracts in release mode..."
    stellar contract build --locked
    ok "Build complete."
}

cmd_deploy_profile() {
    require_env INITIAL_ADMIN_KEY
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
        --admin "$(stellar keys address "$INITIAL_ADMIN_KEY")")

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
        err "events contract already deployed: $(deployment_get events_contract). Refusing to re-deploy. Use the guarded upgrade flow in the mainnet runbook."
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

cmd_upload_profile_wasm() {
    local wasm="${1:-}"
    [ -n "$wasm" ] || err "usage: upload-profile-wasm <path-to-new-wasm>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    require_env UPLOAD_SOURCE
    require_env ADMIN_SOURCE
    require_cli
    confirm_mainnet
    ensure_deployments_dir

    local profile_id expected_hash uploaded_hash
    profile_id="$(profile_contract_id)"
    assert_profile_version "$profile_id" "1.1.0"
    assert_profile_admin_source_matches "$profile_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_local_wasm_hash "profile SDK 27" "$wasm" "$SDK27_PROFILE_WASM_HASH"
    uploaded_hash="$(stellar contract upload \
        --source-account "$UPLOAD_SOURCE" \
        --network "$NETWORK" \
        --optimize=false \
        --wasm "$wasm")"
    [ "$uploaded_hash" = "$expected_hash" ] || \
        err "uploaded profile wasm hash mismatch: expected $expected_hash, got $uploaded_hash"

    append_upgrade_log "profile-uploaded" "" "$uploaded_hash"
    ok "Uploaded exact profile WASM: $uploaded_hash"
}

cmd_prepare_pause_profile() {
    local output_file="${1:-}"
    [ -n "$output_file" ] || err "usage: prepare-pause-profile <xdr-output>"
    require_env ADMIN_SOURCE
    require_cli

    local profile_id events_id
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    assert_events_version "$events_id" "1.1.0"
    assert_events_paused "$events_id"
    assert_no_events_exist "$events_id"
    assert_no_pending_events_upgrade "$events_id"
    assert_profile_version "$profile_id" "1.1.0"
    assert_profile_unpaused "$profile_id"
    assert_no_pending_profile_upgrade "$profile_id"
    prepare_profile_invoke "$profile_id" "$output_file" pause
}

cmd_prepare_propose_upgrade_profile() {
    local wasm="${1:-}" new_version="${2:-}" output_file="${3:-}"
    [ -n "$wasm" ] || \
        err "usage: prepare-propose-upgrade-profile <path-to-new-wasm> <new-version> <xdr-output>"
    [ -n "$new_version" ] || \
        err "usage: prepare-propose-upgrade-profile <path-to-new-wasm> <new-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-propose-upgrade-profile <path-to-new-wasm> <new-version> <xdr-output>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "profile upgrade" "$new_version" "$SDK27_PROFILE_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local profile_id events_id wasm_hash
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    wasm_hash="$(hash_wasm "$wasm")"
    assert_sdk27_profile_artifact "$wasm" "$new_version"
    assert_events_version "$events_id" "1.1.0"
    assert_events_paused "$events_id"
    assert_no_events_exist "$events_id"
    assert_no_pending_events_upgrade "$events_id"
    assert_no_pending_profile_upgrade "$profile_id"
    assert_profile_paused "$profile_id"
    if [ "$new_version" = "1.2.0" ]; then
        assert_profile_version "$profile_id" "1.1.0"
    fi

    prepare_profile_invoke "$profile_id" "$output_file" propose_upgrade \
        --new_wasm_hash "$wasm_hash" \
        --new_version "$new_version"
    echo "Expected profile version: $new_version"
    echo "Expected profile WASM:    $wasm_hash"
}

cmd_verify_proposed_upgrade_profile() {
    local wasm="${1:-}" expected_version="${2:-}"
    [ -n "$wasm" ] || \
        err "usage: verify-proposed-upgrade-profile <path-to-new-wasm> <expected-version>"
    [ -n "$expected_version" ] || \
        err "usage: verify-proposed-upgrade-profile <path-to-new-wasm> <expected-version>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "profile upgrade" "$expected_version" "$SDK27_PROFILE_VERSION"
    require_env ADMIN_SOURCE
    require_cli
    ensure_deployments_dir

    local profile_id events_id expected_hash
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_sdk27_profile_artifact "$wasm" "$expected_version"
    assert_events_version "$events_id" "1.1.0"
    assert_events_paused "$events_id"
    assert_no_events_exist "$events_id"
    assert_no_pending_events_upgrade "$events_id"
    assert_pending_profile_upgrade "$profile_id" "$expected_version" "$expected_hash"
    assert_profile_paused "$profile_id"
    if [ "$expected_version" = "1.2.0" ]; then
        assert_profile_version "$profile_id" "1.1.0"
    fi

    append_upgrade_log "profile-proposal-verified" "$expected_version" "$expected_hash"
    ok "Queued profile upgrade matches the expected version and exact WASM."
}

cmd_prepare_apply_upgrade_profile() {
    local wasm="${1:-}" expected_version="${2:-}" output_file="${3:-}"
    [ -n "$wasm" ] || \
        err "usage: prepare-apply-upgrade-profile <path-to-new-wasm> <expected-version> <xdr-output>"
    [ -n "$expected_version" ] || \
        err "usage: prepare-apply-upgrade-profile <path-to-new-wasm> <expected-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-apply-upgrade-profile <path-to-new-wasm> <expected-version> <xdr-output>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "profile upgrade" "$expected_version" "$SDK27_PROFILE_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local profile_id events_id expected_hash
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_sdk27_profile_artifact "$wasm" "$expected_version"
    assert_events_version "$events_id" "1.1.0"
    assert_events_paused "$events_id"
    assert_no_events_exist "$events_id"
    assert_pending_events_upgrade \
        "$events_id" "$SDK27_EVENTS_VERSION" "$SDK27_EVENTS_WASM_HASH"
    assert_pending_profile_upgrade "$profile_id" "$expected_version" "$expected_hash"
    assert_profile_paused "$profile_id"
    if [ "$expected_version" = "1.2.0" ]; then
        assert_profile_version "$profile_id" "1.1.0"
    fi

    prepare_profile_invoke "$profile_id" "$output_file" apply_upgrade
}

cmd_prepare_migrate_upgrade_profile() {
    local expected_version="${1:-}" output_file="${2:-}"
    [ -n "$expected_version" ] || \
        err "usage: prepare-migrate-upgrade-profile <expected-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-migrate-upgrade-profile <expected-version> <xdr-output>"
    assert_version_argument "profile upgrade" "$expected_version" "$SDK27_PROFILE_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local profile_id events_id migrated_version
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    assert_events_version "$events_id" "1.5.0"
    assert_events_sdk27_wasm_live "$events_id"
    assert_events_paused "$events_id"
    assert_no_pending_events_upgrade "$events_id"
    assert_profile_version "$profile_id" "$expected_version"
    if [ "$expected_version" = "$SDK27_PROFILE_VERSION" ]; then
        assert_profile_sdk27_wasm_live "$profile_id"
    fi
    assert_profile_paused "$profile_id"
    assert_no_pending_profile_upgrade "$profile_id"

    migrated_version="$(profile_read "$profile_id" get_migrated_to_version | jq -r '. // empty')" || \
        err "could not read profile migrated-to version"
    [ "$migrated_version" != "$expected_version" ] || \
        err "profile migration marker is already $expected_version; do not replay migrate"
    prepare_profile_invoke "$profile_id" "$output_file" migrate
}

cmd_prepare_cancel_upgrade_profile() {
    local output_file="${1:-}"
    [ -n "$output_file" ] || \
        err "usage: prepare-cancel-upgrade-profile <xdr-output>"
    require_env ADMIN_SOURCE
    require_cli

    local profile_id events_id pending_version
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    pending_version="$(profile_read "$profile_id" get_pending_upgrade | jq -r '.new_version // empty')" || \
        err "could not read pending profile upgrade"
    [ -n "$pending_version" ] || err "no profile upgrade is pending"
    if [ "$pending_version" = "1.2.0" ]; then
        assert_profile_version "$profile_id" "1.1.0"
        assert_profile_paused "$profile_id"
        assert_events_version "$events_id" "1.1.0"
        assert_events_paused "$events_id"
        assert_no_events_exist "$events_id"
    fi
    prepare_profile_invoke "$profile_id" "$output_file" cancel_pending_upgrade
    echo "Cancelling queued profile version: $pending_version"
    echo "This transaction does not unpause the profile contract."
}

cmd_prepare_unpause_profile() {
    local expected_version="${1:-}" output_file="${2:-}"
    [ -n "$expected_version" ] || \
        err "usage: prepare-unpause-profile <expected-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-unpause-profile <expected-version> <xdr-output>"
    assert_unpause_version_argument "profile" "$expected_version" "$SDK27_PROFILE_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local profile_id events_id
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    assert_profile_version "$profile_id" "$expected_version"
    assert_profile_paused "$profile_id"
    assert_no_pending_profile_upgrade "$profile_id"
    if [ "$expected_version" = "1.2.0" ]; then
        assert_profile_sdk27_wasm_live "$profile_id"
        assert_profile_migrated_version "$profile_id" "$expected_version"
        assert_events_version "$events_id" "1.5.0"
        assert_events_sdk27_wasm_live "$events_id"
        assert_events_paused "$events_id"
        assert_no_pending_events_upgrade "$events_id"
        assert_migrated_version "$events_id" "1.5.0"
    elif [ "$expected_version" = "1.1.0" ]; then
        assert_events_version "$events_id" "1.1.0"
        assert_events_paused "$events_id"
        assert_no_pending_events_upgrade "$events_id"
        assert_no_events_exist "$events_id"
    fi
    prepare_profile_invoke "$profile_id" "$output_file" unpause
}

cmd_verify_upgrade_profile() {
    local wasm="${1:-}" expected_version="${2:-}"
    [ -n "$wasm" ] || \
        err "usage: verify-upgrade-profile <path-to-new-wasm> <expected-version>"
    [ -n "$expected_version" ] || \
        err "usage: verify-upgrade-profile <path-to-new-wasm> <expected-version>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "profile upgrade" "$expected_version" "$SDK27_PROFILE_VERSION"
    require_env ADMIN_SOURCE
    require_cli
    ensure_deployments_dir

    local profile_id events_id expected_hash
    profile_id="$(profile_contract_id)"
    events_id="$(events_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_sdk27_profile_artifact "$wasm" "$expected_version"
    assert_profile_version "$profile_id" "$expected_version"
    if [ "$expected_version" = "$SDK27_PROFILE_VERSION" ]; then
        assert_profile_sdk27_wasm_live "$profile_id"
    fi
    assert_profile_migrated_version "$profile_id" "$expected_version"
    assert_no_pending_profile_upgrade "$profile_id"
    assert_profile_unpaused "$profile_id"
    assert_events_version "$events_id" "1.5.0"
    assert_events_sdk27_wasm_live "$events_id"
    assert_migrated_version "$events_id" "1.5.0"
    assert_no_pending_events_upgrade "$events_id"
    assert_events_unpaused "$events_id"

    deployment_set profile_contract "$profile_id"
    deployment_set profile_wasm_hash "$expected_hash"
    deployment_set profile_version "$expected_version"
    append_upgrade_log "profile-upgrade-verified" "$expected_version" "$expected_hash"
    ok "Profile $expected_version is migrated, has no pending upgrade, and is unpaused."
}

cmd_upload_events_wasm() {
    local wasm="${1:-}"
    [ -n "$wasm" ] || err "usage: upload-events-wasm <path-to-new-wasm>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    require_env UPLOAD_SOURCE
    require_env ADMIN_SOURCE
    require_cli
    confirm_mainnet
    ensure_deployments_dir

    local events_id expected_hash uploaded_hash
    events_id="$(events_contract_id)"
    assert_events_version "$events_id" "1.1.0"
    assert_admin_source_matches "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_local_wasm_hash "events SDK 27" "$wasm" "$SDK27_EVENTS_WASM_HASH"
    uploaded_hash="$(stellar contract upload \
        --source-account "$UPLOAD_SOURCE" \
        --network "$NETWORK" \
        --optimize=false \
        --wasm "$wasm")"
    [ "$uploaded_hash" = "$expected_hash" ] || \
        err "uploaded wasm hash mismatch: expected $expected_hash, got $uploaded_hash"

    append_upgrade_log "uploaded" "" "$uploaded_hash"
    ok "Uploaded exact events WASM: $uploaded_hash"
}

cmd_prepare_pause_events() {
    local output_file="${1:-}"
    [ -n "$output_file" ] || err "usage: prepare-pause-events <xdr-output>"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    assert_profile_version "$profile_id" "1.1.0"
    assert_profile_unpaused "$profile_id"
    assert_no_pending_profile_upgrade "$profile_id"
    assert_events_version "$events_id" "1.1.0"
    assert_events_unpaused "$events_id"
    assert_no_pending_events_upgrade "$events_id"
    assert_no_events_exist "$events_id"
    prepare_events_invoke "$events_id" "$output_file" pause
}

cmd_prepare_propose_upgrade_events() {
    local wasm="${1:-}" new_version="${2:-}" output_file="${3:-}"
    [ -n "$wasm" ] || \
        err "usage: prepare-propose-upgrade-events <path-to-new-wasm> <new-version> <xdr-output>"
    [ -n "$new_version" ] || \
        err "usage: prepare-propose-upgrade-events <path-to-new-wasm> <new-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-propose-upgrade-events <path-to-new-wasm> <new-version> <xdr-output>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "events upgrade" "$new_version" "$SDK27_EVENTS_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id wasm_hash
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    wasm_hash="$(hash_wasm "$wasm")"
    assert_sdk27_events_artifact "$wasm" "$new_version"
    assert_no_pending_events_upgrade "$events_id"

    if is_zero_event_guarded_target "$new_version"; then
        assert_events_version "$events_id" "1.1.0"
        assert_events_paused "$events_id"
        assert_no_events_exist "$events_id"
        if [ "$new_version" = "1.5.0" ]; then
            assert_profile_sdk27_staged_or_applied "$profile_id"
        fi
    fi

    prepare_events_invoke "$events_id" "$output_file" propose_upgrade \
        --new_wasm_hash "$wasm_hash" \
        --new_version "$new_version"
    echo "Expected version: $new_version"
    echo "Expected WASM:    $wasm_hash"
}

cmd_verify_proposed_upgrade_events() {
    local wasm="${1:-}" expected_version="${2:-}"
    [ -n "$wasm" ] || \
        err "usage: verify-proposed-upgrade-events <path-to-new-wasm> <expected-version>"
    [ -n "$expected_version" ] || \
        err "usage: verify-proposed-upgrade-events <path-to-new-wasm> <expected-version>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "events upgrade" "$expected_version" "$SDK27_EVENTS_VERSION"
    require_env ADMIN_SOURCE
    require_cli
    ensure_deployments_dir

    local events_id profile_id expected_hash
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_sdk27_events_artifact "$wasm" "$expected_version"
    assert_pending_events_upgrade "$events_id" "$expected_version" "$expected_hash"

    if is_zero_event_guarded_target "$expected_version"; then
        assert_events_version "$events_id" "1.1.0"
        assert_events_paused "$events_id"
        assert_no_events_exist "$events_id"
        if [ "$expected_version" = "1.5.0" ]; then
            assert_profile_sdk27_staged_or_applied "$profile_id"
        fi
    fi

    append_upgrade_log "proposal-verified" "$expected_version" "$expected_hash"
    ok "Queued events upgrade matches the expected version and exact WASM."
}

cmd_prepare_apply_upgrade_events() {
    local wasm="${1:-}" expected_version="${2:-}" output_file="${3:-}"
    [ -n "$wasm" ] || \
        err "usage: prepare-apply-upgrade-events <path-to-new-wasm> <expected-version> <xdr-output>"
    [ -n "$expected_version" ] || \
        err "usage: prepare-apply-upgrade-events <path-to-new-wasm> <expected-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-apply-upgrade-events <path-to-new-wasm> <expected-version> <xdr-output>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "events upgrade" "$expected_version" "$SDK27_EVENTS_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id expected_hash
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_sdk27_events_artifact "$wasm" "$expected_version"
    assert_pending_events_upgrade "$events_id" "$expected_version" "$expected_hash"

    if is_zero_event_guarded_target "$expected_version"; then
        assert_events_version "$events_id" "1.1.0"
        assert_events_paused "$events_id"
        assert_no_events_exist "$events_id"
        if [ "$expected_version" = "1.5.0" ]; then
            assert_profile_sdk27_applied_paused "$profile_id"
        fi
    fi

    prepare_events_invoke "$events_id" "$output_file" apply_upgrade
}

cmd_prepare_migrate_upgrade_events() {
    local expected_version="${1:-}" output_file="${2:-}"
    [ -n "$expected_version" ] || \
        err "usage: prepare-migrate-upgrade-events <expected-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-migrate-upgrade-events <expected-version> <xdr-output>"
    assert_version_argument "events upgrade" "$expected_version" "$SDK27_EVENTS_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id migrated_version
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    assert_events_version "$events_id" "$expected_version"
    if [ "$expected_version" = "$SDK27_EVENTS_VERSION" ]; then
        assert_events_sdk27_wasm_live "$events_id"
    fi
    assert_no_pending_events_upgrade "$events_id"
    assert_events_paused "$events_id"

    migrated_version="$(events_read "$events_id" get_migrated_to_version | jq -r '. // empty')" || \
        err "could not read migrated-to version"
    [ "$migrated_version" != "$expected_version" ] || \
        err "migration marker is already $expected_version; do not replay migrate"
    if [ "$expected_version" = "1.5.0" ]; then
        assert_profile_sdk27_migrated_paused "$profile_id"
    fi
    prepare_events_invoke "$events_id" "$output_file" migrate
}

cmd_prepare_cancel_upgrade_events() {
    local output_file="${1:-}"
    [ -n "$output_file" ] || \
        err "usage: prepare-cancel-upgrade-events <xdr-output>"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id pending_version
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    pending_version="$(events_read "$events_id" get_pending_upgrade | jq -r '.new_version // empty')" || \
        err "could not read pending events upgrade"
    [ -n "$pending_version" ] || err "no events upgrade is pending"
    if [ "$pending_version" = "1.5.0" ]; then
        assert_events_version "$events_id" "1.1.0"
        assert_events_paused "$events_id"
        assert_no_events_exist "$events_id"
        assert_profile_version "$profile_id" "1.1.0"
        assert_profile_paused "$profile_id"
    fi
    prepare_events_invoke "$events_id" "$output_file" cancel_pending_upgrade
    echo "Cancelling queued version: $pending_version"
    echo "This transaction does not unpause the contract."
}

cmd_prepare_unpause_events() {
    local expected_version="${1:-}" output_file="${2:-}"
    [ -n "$expected_version" ] || \
        err "usage: prepare-unpause-events <expected-version> <xdr-output>"
    [ -n "$output_file" ] || \
        err "usage: prepare-unpause-events <expected-version> <xdr-output>"
    assert_unpause_version_argument "events" "$expected_version" "$SDK27_EVENTS_VERSION"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    assert_events_version "$events_id" "$expected_version"
    if [ "$expected_version" = "$SDK27_EVENTS_VERSION" ]; then
        assert_events_sdk27_wasm_live "$events_id"
    fi
    assert_events_paused "$events_id"
    assert_no_pending_events_upgrade "$events_id"
    if is_zero_event_guarded_target "$expected_version"; then
        assert_migrated_version "$events_id" "$expected_version"
    fi
    if [ "$expected_version" = "1.5.0" ]; then
        assert_profile_sdk27_ready "$profile_id"
    elif [ "$expected_version" = "1.1.0" ]; then
        assert_no_events_exist "$events_id"
        assert_profile_version "$profile_id" "1.1.0"
        assert_profile_unpaused "$profile_id"
        assert_no_pending_profile_upgrade "$profile_id"
    fi
    prepare_events_invoke "$events_id" "$output_file" unpause
}

cmd_verify_upgrade_events() {
    local wasm="${1:-}" expected_version="${2:-}"
    [ -n "$wasm" ] || \
        err "usage: verify-upgrade-events <path-to-new-wasm> <expected-version>"
    [ -n "$expected_version" ] || \
        err "usage: verify-upgrade-events <path-to-new-wasm> <expected-version>"
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    assert_version_argument "events upgrade" "$expected_version" "$SDK27_EVENTS_VERSION"
    require_env ADMIN_SOURCE
    require_cli
    ensure_deployments_dir

    local events_id profile_id expected_hash
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"
    assert_upgrade_governance_stable "$profile_id" "$events_id"
    expected_hash="$(hash_wasm "$wasm")"
    assert_sdk27_events_artifact "$wasm" "$expected_version"
    assert_events_version "$events_id" "$expected_version"
    if [ "$expected_version" = "$SDK27_EVENTS_VERSION" ]; then
        assert_events_sdk27_wasm_live "$events_id"
    fi
    assert_migrated_version "$events_id" "$expected_version"
    assert_no_pending_events_upgrade "$events_id"
    assert_events_unpaused "$events_id"
    if [ "$expected_version" = "1.5.0" ]; then
        assert_profile_sdk27_ready "$profile_id"
    fi

    deployment_set events_contract "$events_id"
    deployment_set events_wasm_hash "$expected_hash"
    deployment_set events_version "$expected_version"
    append_upgrade_log "upgrade-verified" "$expected_version" "$expected_hash"
    ok "Events $expected_version is migrated, has no pending upgrade, and is unpaused."
}

cmd_upgrade_status() {
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id
    events_id="$(events_contract_id)"
    profile_id="$(profile_contract_id)"

    echo "events.version:"
    events_read "$events_id" version
    echo "events.wasm_hash:"
    deployed_wasm_hash "$events_id"
    echo "events.is_paused:"
    events_read "$events_id" is_paused
    echo "events.get_pending_upgrade:"
    events_read "$events_id" get_pending_upgrade
    echo "events.get_migrated_to_version:"
    events_read "$events_id" get_migrated_to_version
    echo "profile.version:"
    profile_read "$profile_id" version
    echo "profile.wasm_hash:"
    deployed_wasm_hash "$profile_id"
    echo "profile.is_paused:"
    profile_read "$profile_id" is_paused
    echo "profile.get_pending_upgrade:"
    profile_read "$profile_id" get_pending_upgrade
    echo "profile.get_events_contract:"
    profile_read "$profile_id" get_events_contract
    echo "profile.get_pending_events_contract:"
    profile_read "$profile_id" get_pending_events_contract
    echo "profile.get_migrated_to_version:"
    profile_read "$profile_id" get_migrated_to_version
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
    "upload-profile-wasm")
        cmd_upload_profile_wasm "$@"
        ;;
    "upload-events-wasm")
        cmd_upload_events_wasm "$@"
        ;;
    "prepare-pause-profile")
        cmd_prepare_pause_profile "$@"
        ;;
    "prepare-pause-events")
        cmd_prepare_pause_events "$@"
        ;;
    "prepare-propose-upgrade-profile")
        cmd_prepare_propose_upgrade_profile "$@"
        ;;
    "prepare-propose-upgrade-events")
        cmd_prepare_propose_upgrade_events "$@"
        ;;
    "verify-proposed-upgrade-profile")
        cmd_verify_proposed_upgrade_profile "$@"
        ;;
    "verify-proposed-upgrade-events")
        cmd_verify_proposed_upgrade_events "$@"
        ;;
    "prepare-apply-upgrade-profile")
        cmd_prepare_apply_upgrade_profile "$@"
        ;;
    "prepare-apply-upgrade-events")
        cmd_prepare_apply_upgrade_events "$@"
        ;;
    "prepare-migrate-upgrade-profile")
        cmd_prepare_migrate_upgrade_profile "$@"
        ;;
    "prepare-migrate-upgrade-events")
        cmd_prepare_migrate_upgrade_events "$@"
        ;;
    "prepare-cancel-upgrade-profile")
        cmd_prepare_cancel_upgrade_profile "$@"
        ;;
    "prepare-cancel-upgrade-events")
        cmd_prepare_cancel_upgrade_events "$@"
        ;;
    "prepare-unpause-profile")
        cmd_prepare_unpause_profile "$@"
        ;;
    "prepare-unpause-events")
        cmd_prepare_unpause_events "$@"
        ;;
    "verify-upgrade-profile")
        cmd_verify_upgrade_profile "$@"
        ;;
    "verify-upgrade-events")
        cmd_verify_upgrade_events "$@"
        ;;
    "upgrade-status")
        cmd_upgrade_status
        ;;
    "upgrade-events")
        err "upgrade-events is removed; use the guarded prepare/sign/send flow in mainnet-deploy-runbook.md"
        ;;
    "propose-upgrade-events"|"apply-upgrade-events"|"migrate-upgrade-events"|"cancel-upgrade-events"|"unpause-events"|\
    "propose-upgrade-profile"|"apply-upgrade-profile"|"migrate-upgrade-profile"|"cancel-upgrade-profile"|"unpause-profile")
        err "$ACTION cannot safely submit as the 2-of-3 admin; use the matching prepare-* action, sign sequentially, and send"
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
