#!/bin/bash
#
# Mainnet deploy and upgrade tool for boundless-events and boundless-profile.
#
# The deploy commands sign with INITIAL_ADMIN_KEY, a throwaway deployer that
# hands admin to the multisig and is then destroyed. Every command that acts as
# the admin of a live contract only builds and simulates an unsigned XDR for
# the 2-of-3 multisig; nothing here submits an admin transaction.
#
# Procedures: docs/DEPLOYMENT.md (fresh deploy), docs/upgrade-runbook.md.
#
# Usage:
#   ./deploy_mainnet.sh deploy-profile
#   ./deploy_mainnet.sh deploy-events
#   ./deploy_mainnet.sh register-token <token-sac-address> <CODE:ISSUER|native>
#   ./deploy_mainnet.sh rotate-admin <new-multisig-address>
#   ./deploy_mainnet.sh upload-profile-wasm <wasm-path> <version>
#   ./deploy_mainnet.sh upload-events-wasm <wasm-path> <version>
#   ./deploy_mainnet.sh prepare-pause-profile <xdr-output>
#   ./deploy_mainnet.sh prepare-pause-events <xdr-output>
#   ./deploy_mainnet.sh prepare-propose-upgrade-profile <wasm-path> <version> <xdr-output>
#   ./deploy_mainnet.sh prepare-propose-upgrade-events <wasm-path> <version> <xdr-output>
#   ./deploy_mainnet.sh verify-proposed-upgrade-profile <wasm-path> <version>
#   ./deploy_mainnet.sh verify-proposed-upgrade-events <wasm-path> <version>
#   ./deploy_mainnet.sh prepare-apply-upgrade-profile <wasm-path> <version> <xdr-output>
#   ./deploy_mainnet.sh prepare-apply-upgrade-events <wasm-path> <version> <xdr-output>
#   ./deploy_mainnet.sh prepare-migrate-events-page <max-events> <xdr-output>
#   ./deploy_mainnet.sh prepare-migrate-upgrade-profile <version> <xdr-output>
#   ./deploy_mainnet.sh prepare-migrate-upgrade-events <version> <xdr-output>
#   ./deploy_mainnet.sh prepare-cancel-upgrade-profile <xdr-output>
#   ./deploy_mainnet.sh prepare-cancel-upgrade-events <xdr-output>
#   ./deploy_mainnet.sh prepare-unpause-profile <version> <xdr-output>
#   ./deploy_mainnet.sh prepare-unpause-events <version> <xdr-output>
#   ./deploy_mainnet.sh verify-upgrade-profile <wasm-path> <version>
#   ./deploy_mainnet.sh verify-upgrade-events <wasm-path> <version>
#   ./deploy_mainnet.sh upgrade-status
#   ./deploy_mainnet.sh verify
#
# Environment:
#   STELLAR_NETWORK   name of a configured network using the public passphrase
#   ADMIN_SOURCE      current admin G-address (the multisig); also the read source
#   EVENTS_ID, PROFILE_ID  default to the local record deployments/mainnet.json
#   UPLOAD_SOURCE     any funded CLI identity, for upload-*-wasm
#   INITIAL_ADMIN_KEY, FEE_ACCOUNT, INITIAL_GLOBAL_FEE_BPS  deploy commands only

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
REQUIRED_STELLAR_CLI_VERSION="28.1.0"
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
DEPLOYMENTS_DIR="$REPO_ROOT/deployments"
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

require_version_arg() {
    local version="$1"
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || \
        err "version must look like 2.0.0; got '$version'"
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
    stellar contract info hash --network "$NETWORK" --contract-id "$1"
}

# A wasm's kind is proven by an entry point only that contract exports, and its
# version by the contractmeta the build embeds, so a mislabelled or swapped
# artifact cannot be proposed under the wrong name.
assert_wasm_artifact() {
    local kind="$1" wasm="$2" version="$3"
    local marker meta_version
    [ -f "$wasm" ] || err "wasm file not found: $wasm"
    case "$kind" in
        events) marker=create_event ;;
        profile) marker=register_earnings ;;
    esac
    stellar contract info interface --wasm "$wasm" --output json 2>/dev/null \
        | jq -e --arg f "$marker" 'any(.[]; .function_v0?.name == $f)' >/dev/null || \
        err "$wasm is not a boundless-$kind build (its interface has no $marker)"
    meta_version="$(stellar contract info meta --wasm "$wasm" --output json 2>/dev/null \
        | jq -r '[.[] | .sc_meta_v0? | select(.key == "version") | .val][0] // empty')" || \
        err "could not read the contractmeta of $wasm"
    [ -n "$meta_version" ] || err "$wasm has no contractmeta version"
    [ "$meta_version" = "$version" ] || \
        err "$wasm reports version $meta_version in its contractmeta, not $version"
}

ensure_deployments_dir() {
    mkdir -p "$DEPLOYMENTS_DIR"
    if [ ! -f "$DEPLOYMENT_FILE" ]; then
        echo "{\"network\":\"mainnet\",\"passphrase\":\"$NETWORK_PASSPHRASE\"}" > "$DEPLOYMENT_FILE"
    fi
}

deployment_get() {
    jq -r --arg k "$1" '.[$k] // empty' "$DEPLOYMENT_FILE"
}

deployment_set() {
    local tmp
    tmp="$(mktemp)"
    jq --arg k "$1" --arg v "$2" '.[$k] = $v' "$DEPLOYMENT_FILE" > "$tmp"
    mv "$tmp" "$DEPLOYMENT_FILE"
}

deployment_set_raw() {
    local tmp
    tmp="$(mktemp)"
    jq --arg k "$1" --argjson v "$2" '.[$k] = $v' "$DEPLOYMENT_FILE" > "$tmp"
    mv "$tmp" "$DEPLOYMENT_FILE"
}

append_upgrade_log() {
    local action="$1" contract="$2" version="$3" wasm_hash="${4:-}"
    ensure_deployments_dir
    jq -nc \
        --arg timestamp "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        --arg action "$action" \
        --arg contract "$contract" \
        --arg version "$version" \
        --arg wasm_hash "$wasm_hash" \
        '{timestamp: $timestamp, action: $action, contract: $contract, version: $version, wasm_hash: $wasm_hash}' \
        >> "$DEPLOYMENTS_DIR/mainnet-upgrades.jsonl"
}

contract_id() {
    local kind="$1" var id=""
    case "$kind" in
        events) var=EVENTS_ID ;;
        profile) var=PROFILE_ID ;;
    esac
    id="${!var:-}"
    if [ -z "$id" ] && [ -f "$DEPLOYMENT_FILE" ]; then
        id="$(deployment_get "${kind}_contract")"
    fi
    [ -n "$id" ] || \
        err "$kind contract id missing; set $var or restore deployments/mainnet.json"
    printf '%s\n' "$id"
}

contract_read() {
    local id="$1"
    shift
    stellar contract invoke \
        --send no \
        --network "$NETWORK" \
        --source "$ADMIN_SOURCE" \
        --id "$id" \
        -- "$@"
}

read_json_string() {
    jq -er 'if type == "string" then . else error("expected JSON string") end'
}

assert_version() {
    local kind="$1" id="$2" expected="$3" actual
    actual="$(contract_read "$id" version | read_json_string)" || \
        err "could not read the $kind version"
    [ "$actual" = "$expected" ] || \
        err "$kind version mismatch: expected $expected, got $actual"
}

assert_admin_source_matches() {
    local kind="$1" id="$2" actual
    actual="$(contract_read "$id" get_admin | read_json_string)" || \
        err "could not read the $kind admin"
    [ "$actual" = "$ADMIN_SOURCE" ] || \
        err "ADMIN_SOURCE mismatch: $kind admin is $actual, got $ADMIN_SOURCE"
}

# Both admins must be the signer set preparing the XDR, and the profile must
# still trust this events contract with no rotation in flight; otherwise a
# prepared envelope could act on a contract the multisig no longer governs.
assert_governance_stable() {
    local profile_id="$1" events_id="$2" binding pending
    assert_admin_source_matches events "$events_id"
    assert_admin_source_matches profile "$profile_id"
    binding="$(contract_read "$profile_id" get_events_contract | read_json_string)" || \
        err "could not read profile.get_events_contract"
    [ "$binding" = "$events_id" ] || \
        err "profile events-contract binding changed: expected $events_id, got $binding"
    pending="$(contract_read "$profile_id" get_pending_events_contract)" || \
        err "could not read profile.get_pending_events_contract"
    [ "$pending" = "null" ] || \
        err "a profile events-contract rotation is pending; cancel or complete it first"
}

assert_paused() {
    local kind="$1" id="$2" paused
    paused="$(contract_read "$id" is_paused)" || err "could not read $kind.is_paused"
    [ "$paused" = "true" ] || err "$kind contract is not paused"
}

assert_unpaused() {
    local kind="$1" id="$2" paused
    paused="$(contract_read "$id" is_paused)" || err "could not read $kind.is_paused"
    [ "$paused" = "false" ] || err "$kind contract is paused"
}

assert_no_pending_upgrade() {
    local kind="$1" id="$2" pending
    pending="$(contract_read "$id" get_pending_upgrade)" || \
        err "could not read the pending $kind upgrade"
    [ "$pending" = "null" ] || \
        err "$kind already has a pending upgrade; inspect it with upgrade-status"
}

assert_pending_upgrade() {
    local kind="$1" id="$2" expected_version="$3" expected_hash="$4"
    local pending actual_version actual_hash
    pending="$(contract_read "$id" get_pending_upgrade)" || \
        err "could not read the pending $kind upgrade"
    [ "$pending" != "null" ] || err "no $kind upgrade is pending"
    actual_version="$(printf '%s' "$pending" | jq -er '.new_version')" || \
        err "pending $kind upgrade did not contain new_version"
    actual_hash="$(printf '%s' "$pending" | jq -er '.wasm_hash')" || \
        err "pending $kind upgrade did not contain wasm_hash"
    [ "$actual_version" = "$expected_version" ] || \
        err "pending $kind version mismatch: expected $expected_version, got $actual_version"
    [ "$actual_hash" = "$expected_hash" ] || \
        err "pending $kind wasm mismatch: expected $expected_hash, got $actual_hash"
}

assert_live_hash() {
    local kind="$1" id="$2" expected="$3" actual
    actual="$(deployed_wasm_hash "$id")" || err "could not read the deployed $kind wasm hash"
    [ "$actual" = "$expected" ] || \
        err "deployed $kind wasm mismatch: expected $expected, got $actual"
}

migrated_version() {
    contract_read "$1" get_migrated_to_version | jq -r '. // empty'
}

# Simulates migrate() to learn whether it would stamp. Prints ready,
# incomplete (events: migrate_events still has rows to convert, error 97) or
# applied (already stamped: events 69, profile 43).
migrate_state() {
    local kind="$1" id="$2" output status applied_code
    case "$kind" in
        events) applied_code=69 ;;
        profile) applied_code=43 ;;
    esac
    set +e
    output="$(contract_read "$id" migrate 2>&1)"
    status=$?
    set -e
    if [ "$status" -eq 0 ]; then
        echo ready
    elif [ "$kind" = events ] && [[ "$output" == *"Error(Contract, #97)"* ]]; then
        echo incomplete
    elif [[ "$output" == *"Error(Contract, #$applied_code)"* ]]; then
        echo applied
    else
        printf '%s\n' "$output" >&2
        return 1
    fi
}

prepare_invoke() {
    local kind="$1" id="$2" output_file="$3"
    shift 3
    local output_dir tmp
    output_dir="$(dirname "$output_file")"
    [ -d "$output_dir" ] || err "XDR output directory does not exist: $output_dir"
    assert_admin_source_matches "$kind" "$id"

    tmp="$(mktemp)"
    if ! stellar contract invoke \
        --network "$NETWORK" \
        --source-account "$ADMIN_SOURCE" \
        --id "$id" \
        --build-only \
        -- "$@" \
        | stellar tx simulate \
            --network "$NETWORK" \
            --source-account "$ADMIN_SOURCE" \
            > "$tmp"; then
        rm -f "$tmp"
        err "could not build and simulate the $kind $1 transaction"
    fi
    [ -s "$tmp" ] || {
        rm -f "$tmp"
        err "prepared $kind transaction was empty"
    }
    mv "$tmp" "$output_file"
    ok "Prepared simulated XDR for $kind $1: $output_file"
    echo "Decode it, sign it sequentially with the multisig quorum, then submit with:"
    echo "  stellar tx send \"<signed-xdr>\" --network $NETWORK"
}

upgrade_ids() {
    events_id="$(contract_id events)"
    profile_id="$(contract_id profile)"
    assert_governance_stable "$profile_id" "$events_id"
    if [ "$1" = events ]; then
        target_id="$events_id"
    else
        target_id="$profile_id"
    fi
}

cmd_deploy_profile() {
    require_env INITIAL_ADMIN_KEY
    require_cli
    ensure_deployments_dir
    if [ -n "$(deployment_get profile_contract)" ]; then
        err "profile contract already deployed: $(deployment_get profile_contract). Refusing to re-deploy."
    fi
    confirm_mainnet

    info "Building boundless-profile without the testnet feature..."
    "$REPO_ROOT/scripts/build-release.sh" --package boundless-profile
    local wasm="$REPO_ROOT/target/wasm32v1-none/release/boundless_profile.wasm"
    [ -f "$wasm" ] || err "missing wasm: $wasm"

    info "Deploying boundless-profile..."
    local profile_id live_hash
    profile_id=$(stellar contract deploy \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --wasm "$wasm" \
        -- \
        --admin "$(stellar keys address "$INITIAL_ADMIN_KEY")")

    ok "profile_contract=$profile_id"
    live_hash="$(deployed_wasm_hash "$profile_id")" || err "could not read the deployed profile wasm hash"
    deployment_set profile_contract "$profile_id"
    deployment_set profile_wasm_hash "$live_hash"
    deployment_set deployed_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    [ "$live_hash" = "$(hash_wasm "$wasm")" ] || \
        err "deployed profile wasm $live_hash differs from the local build; investigate before continuing"
}

cmd_deploy_events() {
    require_env INITIAL_ADMIN_KEY
    require_env FEE_ACCOUNT
    require_env INITIAL_GLOBAL_FEE_BPS
    [[ "$FEE_ACCOUNT" =~ ^G[A-Z2-7]{55}$ ]] || err "FEE_ACCOUNT must be a G... address"
    [[ "$INITIAL_GLOBAL_FEE_BPS" =~ ^[0-9]+$ ]] && [ "$INITIAL_GLOBAL_FEE_BPS" -le 1000 ] || \
        err "INITIAL_GLOBAL_FEE_BPS must be 0..1000 (the contract caps fees at 10%)"
    require_cli
    ensure_deployments_dir

    local profile_id
    profile_id="$(deployment_get profile_contract)"
    [ -n "$profile_id" ] || err "deploy-profile first; profile_contract not set"
    if [ -n "$(deployment_get events_contract)" ]; then
        err "events contract already deployed: $(deployment_get events_contract). Refusing to re-deploy; upgrades follow docs/upgrade-runbook.md."
    fi
    confirm_mainnet

    info "Building boundless-events without the testnet feature..."
    "$REPO_ROOT/scripts/build-release.sh" --package boundless-events
    local wasm="$REPO_ROOT/target/wasm32v1-none/release/boundless_events.wasm"
    [ -f "$wasm" ] || err "missing wasm: $wasm"

    info "Deploying boundless-events..."
    local events_id live_hash
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
    live_hash="$(deployed_wasm_hash "$events_id")" || err "could not read the deployed events wasm hash"
    deployment_set events_contract "$events_id"
    deployment_set events_wasm_hash "$live_hash"
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
    [ "$live_hash" = "$(hash_wasm "$wasm")" ] || \
        err "deployed events wasm $live_hash differs from the local build; investigate before continuing"
}

cmd_register_token() {
    local token="${1:-}" asset="${2:-}"
    [ -n "$token" ] && [ -n "$asset" ] || \
        err "usage: register-token <token-sac-address> <CODE:ISSUER|native>"
    [[ "$token" =~ ^C[A-Z2-7]{55}$ ]] || err "token must be a C... contract address"
    require_env INITIAL_ADMIN_KEY
    require_cli

    local events_id fee_account wrapped admin deployer
    events_id="$(deployment_get events_contract)"
    fee_account="$(deployment_get fee_account)"
    [ -n "$events_id" ] || err "events contract not deployed"
    [ -n "$fee_account" ] || err "fee_account missing from deployments/mainnet.json"

    deployer="$(stellar keys address "$INITIAL_ADMIN_KEY")"
    admin="$(ADMIN_SOURCE="$deployer" contract_read "$events_id" get_admin | read_json_string)" || \
        err "could not read the events admin"
    [ "$admin" = "$deployer" ] || \
        err "admin is now $admin; register tokens through the multisig (docs/contract-ops-runbook.md)"

    wrapped="$(stellar contract id asset --asset "$asset" --network "$NETWORK")"
    [ "$wrapped" = "$token" ] || err "$asset is wrapped by $wrapped, not $token"
    "$REPO_ROOT/scripts/admin/verify-fee-trustline.sh" mainnet "$fee_account" "$asset"
    confirm_mainnet

    info "Registering token $token..."
    stellar contract invoke \
        --network "$NETWORK" \
        --source "$INITIAL_ADMIN_KEY" \
        --id "$events_id" \
        -- register_supported_token \
        --token "$token"

    local supported
    supported="$(ADMIN_SOURCE="$deployer" contract_read "$events_id" is_supported_token --token "$token")"
    [ "$supported" = "true" ] || err "token registration not confirmed; got: $supported"
    ok "Token registered: $token"

    local tmp
    tmp="$(mktemp)"
    jq --arg t "$token" '.registered_tokens = ((.registered_tokens // []) + [$t] | unique)' \
        "$DEPLOYMENT_FILE" > "$tmp"
    mv "$tmp" "$DEPLOYMENT_FILE"
}

cmd_rotate_admin() {
    local new_admin="${1:-}"
    [ -n "$new_admin" ] || err "usage: rotate-admin <new-multisig-address>"
    require_env INITIAL_ADMIN_KEY
    require_cli

    local events_id profile_id
    events_id="$(deployment_get events_contract)"
    profile_id="$(deployment_get profile_contract)"
    [ -n "$events_id" ]  || err "events contract not deployed"
    [ -n "$profile_id" ] || err "profile contract not deployed"

    "$REPO_ROOT/scripts/admin/verify-multisig.sh" "$new_admin" mainnet
    confirm_mainnet

    info "Setting pending admin on events..."
    stellar contract invoke \
        --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$events_id" \
        -- set_admin --new_admin "$new_admin"

    info "Setting pending admin on profile..."
    stellar contract invoke \
        --network "$NETWORK" --source "$INITIAL_ADMIN_KEY" --id "$profile_id" \
        -- set_admin --new_admin "$new_admin"

    echo ""
    echo -e "${BOLD}$new_admin must now call accept_admin on both contracts.${NC}"
    echo "Build, simulate and sign each accept per docs/DEPLOYMENT.md, then run"
    echo "'verify' with ADMIN_SOURCE=$new_admin."
}

cmd_upload_wasm() {
    local kind="$1" wasm="${2:-}" version="${3:-}"
    [ -n "$wasm" ] && [ -n "$version" ] || \
        err "usage: upload-$kind-wasm <wasm-path> <version>"
    require_version_arg "$version"
    require_env UPLOAD_SOURCE
    require_env ADMIN_SOURCE
    require_cli
    assert_wasm_artifact "$kind" "$wasm" "$version"

    local id expected_hash uploaded_hash live
    id="$(contract_id "$kind")"
    live="$(contract_read "$id" version | read_json_string)" || \
        err "could not read the $kind version"
    [ "$live" != "$version" ] || err "$kind already reports $version"
    expected_hash="$(hash_wasm "$wasm")"
    confirm_mainnet

    uploaded_hash="$(stellar contract upload \
        --source-account "$UPLOAD_SOURCE" \
        --network "$NETWORK" \
        --optimize=false \
        --wasm "$wasm")"
    [ "$uploaded_hash" = "$expected_hash" ] || \
        err "uploaded $kind wasm hash mismatch: expected $expected_hash, got $uploaded_hash"

    append_upgrade_log "uploaded" "$kind" "$version" "$uploaded_hash"
    ok "Uploaded the exact $kind $version wasm: $uploaded_hash"
}

cmd_prepare_pause() {
    local kind="$1" output_file="${2:-}"
    [ -n "$output_file" ] || err "usage: prepare-pause-$kind <xdr-output>"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id target_id
    upgrade_ids "$kind"
    assert_unpaused "$kind" "$target_id"
    prepare_invoke "$kind" "$target_id" "$output_file" pause
}

cmd_prepare_propose_upgrade() {
    local kind="$1" wasm="${2:-}" version="${3:-}" output_file="${4:-}"
    [ -n "$wasm" ] && [ -n "$version" ] && [ -n "$output_file" ] || \
        err "usage: prepare-propose-upgrade-$kind <wasm-path> <version> <xdr-output>"
    require_version_arg "$version"
    require_env ADMIN_SOURCE
    require_cli
    assert_wasm_artifact "$kind" "$wasm" "$version"

    local events_id profile_id target_id wasm_hash live live_hash
    upgrade_ids "$kind"
    wasm_hash="$(hash_wasm "$wasm")"
    assert_no_pending_upgrade "$kind" "$target_id"
    live="$(contract_read "$target_id" version | read_json_string)" || \
        err "could not read the $kind version"
    [ "$live" != "$version" ] || \
        err "$kind already reports $version; a proposal must name a new version"
    live_hash="$(deployed_wasm_hash "$target_id")" || \
        err "could not read the deployed $kind wasm hash"
    [ "$live_hash" != "$wasm_hash" ] || err "this wasm is already live on $kind"

    prepare_invoke "$kind" "$target_id" "$output_file" propose_upgrade \
        --new_wasm_hash "$wasm_hash" \
        --new_version "$version"
    echo "Live $kind:      $live ($live_hash)"
    echo "Proposed $kind:  $version ($wasm_hash)"
}

cmd_verify_proposed_upgrade() {
    local kind="$1" wasm="${2:-}" version="${3:-}"
    [ -n "$wasm" ] && [ -n "$version" ] || \
        err "usage: verify-proposed-upgrade-$kind <wasm-path> <version>"
    require_version_arg "$version"
    require_env ADMIN_SOURCE
    require_cli
    assert_wasm_artifact "$kind" "$wasm" "$version"

    local events_id profile_id target_id wasm_hash
    upgrade_ids "$kind"
    wasm_hash="$(hash_wasm "$wasm")"
    assert_pending_upgrade "$kind" "$target_id" "$version" "$wasm_hash"

    contract_read "$target_id" get_pending_upgrade \
        | jq -r '"available_at_ledger: \(.available_at_ledger)\nexpires_at_ledger:   \(.expires_at_ledger)"'
    append_upgrade_log "proposal-verified" "$kind" "$version" "$wasm_hash"
    ok "The queued $kind upgrade matches $version and the exact wasm."
}

cmd_prepare_apply_upgrade() {
    local kind="$1" wasm="${2:-}" version="${3:-}" output_file="${4:-}"
    [ -n "$wasm" ] && [ -n "$version" ] && [ -n "$output_file" ] || \
        err "usage: prepare-apply-upgrade-$kind <wasm-path> <version> <xdr-output>"
    require_version_arg "$version"
    require_env ADMIN_SOURCE
    require_cli
    assert_wasm_artifact "$kind" "$wasm" "$version"

    local events_id profile_id target_id wasm_hash
    upgrade_ids "$kind"
    wasm_hash="$(hash_wasm "$wasm")"
    assert_pending_upgrade "$kind" "$target_id" "$version" "$wasm_hash"
    # Applying events can change the record layout; keep users out until
    # migrate has converted every row.
    if [ "$kind" = events ]; then
        assert_paused events "$events_id"
    fi

    prepare_invoke "$kind" "$target_id" "$output_file" apply_upgrade
}

cmd_prepare_migrate_events_page() {
    local max_events="${1:-}" output_file="${2:-}"
    [ -n "$max_events" ] && [ -n "$output_file" ] || \
        err "usage: prepare-migrate-events-page <max-events> <xdr-output>"
    [[ "$max_events" =~ ^[1-9][0-9]*$ ]] || err "max-events must be a positive integer"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id target_id live migrated state remaining
    upgrade_ids events
    assert_no_pending_upgrade events "$events_id"
    assert_paused events "$events_id"
    live="$(contract_read "$events_id" version | read_json_string)" || \
        err "could not read the events version"
    migrated="$(migrated_version "$events_id")" || err "could not read the events migration marker"
    [ "$migrated" != "$live" ] || err "events is already migrated to $live; there is nothing to page"
    state="$(migrate_state events "$events_id")" || err "could not simulate events migrate"
    [ "$state" = incomplete ] || \
        err "migrate_events has nothing left to convert; prepare-migrate-upgrade-events is next"
    remaining="$(contract_read "$events_id" migrate_events --max_events "$max_events" | tr -d '"')" || \
        err "could not simulate migrate_events"
    [[ "$remaining" =~ ^[0-9]+$ ]] || err "migrate_events simulation returned '$remaining'"

    prepare_invoke events "$events_id" "$output_file" migrate_events --max_events "$max_events"
    echo "Simulated: $remaining event(s) will remain after this page (the contract converts at most 8 per call)."
}

cmd_prepare_migrate_upgrade() {
    local kind="$1" version="${2:-}" output_file="${3:-}"
    [ -n "$version" ] && [ -n "$output_file" ] || \
        err "usage: prepare-migrate-upgrade-$kind <version> <xdr-output>"
    require_version_arg "$version"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id target_id migrated state
    upgrade_ids "$kind"
    assert_version "$kind" "$target_id" "$version"
    assert_no_pending_upgrade "$kind" "$target_id"
    if [ "$kind" = events ]; then
        assert_paused events "$events_id"
    fi
    migrated="$(migrated_version "$target_id")" || err "could not read the $kind migration marker"
    [ "$migrated" != "$version" ] || \
        err "$kind is already migrated to $version; migrate cannot be replayed"
    state="$(migrate_state "$kind" "$target_id")" || err "could not simulate $kind migrate"
    case "$state" in
        incomplete)
            err "migrate_events still has events to convert; prepare and submit prepare-migrate-events-page until it reports 0 remaining" ;;
        applied)
            err "$kind migrate reports it has already run" ;;
    esac

    prepare_invoke "$kind" "$target_id" "$output_file" migrate
}

cmd_prepare_cancel_upgrade() {
    local kind="$1" output_file="${2:-}"
    [ -n "$output_file" ] || err "usage: prepare-cancel-upgrade-$kind <xdr-output>"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id target_id pending_version
    upgrade_ids "$kind"
    pending_version="$(contract_read "$target_id" get_pending_upgrade | jq -r '.new_version // empty')" || \
        err "could not read the pending $kind upgrade"
    [ -n "$pending_version" ] || err "no $kind upgrade is pending"
    prepare_invoke "$kind" "$target_id" "$output_file" cancel_pending_upgrade
    echo "Cancelling queued $kind version $pending_version. The pause flag is unchanged."
}

cmd_prepare_unpause() {
    local kind="$1" version="${2:-}" output_file="${3:-}"
    [ -n "$version" ] && [ -n "$output_file" ] || \
        err "usage: prepare-unpause-$kind <version> <xdr-output>"
    require_version_arg "$version"
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id target_id migrated
    upgrade_ids "$kind"
    assert_version "$kind" "$target_id" "$version"
    assert_paused "$kind" "$target_id"
    assert_no_pending_upgrade "$kind" "$target_id"
    migrated="$(migrated_version "$target_id")" || err "could not read the $kind migration marker"
    if [ "$migrated" != "$version" ] && [ "${UNPAUSE_UNMIGRATED:-}" != 1 ]; then
        err "$kind $version is not migrated (marker: ${migrated:-none}). Finish the migration, or set UNPAUSE_UNMIGRATED=1 only when abandoning an upgrade that was never applied."
    fi
    prepare_invoke "$kind" "$target_id" "$output_file" unpause
}

cmd_verify_upgrade() {
    local kind="$1" wasm="${2:-}" version="${3:-}"
    [ -n "$wasm" ] && [ -n "$version" ] || \
        err "usage: verify-upgrade-$kind <wasm-path> <version>"
    require_version_arg "$version"
    require_env ADMIN_SOURCE
    require_cli
    assert_wasm_artifact "$kind" "$wasm" "$version"

    local events_id profile_id target_id wasm_hash migrated
    upgrade_ids "$kind"
    wasm_hash="$(hash_wasm "$wasm")"
    assert_version "$kind" "$target_id" "$version"
    assert_live_hash "$kind" "$target_id" "$wasm_hash"
    migrated="$(migrated_version "$target_id")" || err "could not read the $kind migration marker"
    [ "$migrated" = "$version" ] || \
        err "$kind migration marker is ${migrated:-none}, expected $version"
    assert_no_pending_upgrade "$kind" "$target_id"
    assert_unpaused "$kind" "$target_id"

    ensure_deployments_dir
    deployment_set "${kind}_contract" "$target_id"
    deployment_set "${kind}_wasm_hash" "$wasm_hash"
    deployment_set "${kind}_version" "$version"
    deployment_set "${kind}_migrated_to_version" "$version"
    deployment_set last_upgraded_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    append_upgrade_log "upgrade-verified" "$kind" "$version" "$wasm_hash"
    ok "$kind $version is live, migrated, has no pending upgrade and is unpaused."
}

show_read() {
    local label="$1" id="$2"
    shift 2
    printf '%s: %s\n' "$label" "$(contract_read "$id" "$@" 2>/dev/null || echo '(read failed)')"
}

cmd_upgrade_status() {
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id kind id state
    events_id="$(contract_id events)"
    profile_id="$(contract_id profile)"

    for kind in events profile; do
        if [ "$kind" = events ]; then id="$events_id"; else id="$profile_id"; fi
        echo "$kind: $id"
        show_read "  version" "$id" version
        printf '  wasm_hash: %s\n' "$(deployed_wasm_hash "$id" 2>/dev/null || echo '(read failed)')"
        show_read "  migrated_to_version" "$id" get_migrated_to_version
        state="$(migrate_state "$kind" "$id" 2>/dev/null)" || state="unknown (simulation failed)"
        printf '  migrate (simulated): %s\n' "$state"
        show_read "  pending_upgrade" "$id" get_pending_upgrade
        show_read "  is_paused" "$id" is_paused
        show_read "  admin" "$id" get_admin
    done
    show_read "events release_validator" "$events_id" get_release_validator
    show_read "events pending_release_validator" "$events_id" get_pending_release_validator
    show_read "profile events_contract" "$profile_id" get_events_contract
    show_read "profile pending_events_contract" "$profile_id" get_pending_events_contract
}

cmd_verify() {
    require_env ADMIN_SOURCE
    require_cli

    local events_id profile_id
    events_id="$(contract_id events)"
    profile_id="$(contract_id profile)"

    info "Mainnet state (compare with deployments/mainnet.json and docs/DEPLOYMENT.md)"
    echo "events: $events_id"
    show_read "  version" "$events_id" version
    show_read "  migrated_to_version" "$events_id" get_migrated_to_version
    show_read "  pending_upgrade" "$events_id" get_pending_upgrade
    show_read "  is_paused" "$events_id" is_paused
    show_read "  admin" "$events_id" get_admin
    show_read "  fee_account" "$events_id" get_fee_account
    show_read "  fee_bps" "$events_id" get_fee_bps
    show_read "  profile_contract" "$events_id" get_profile_contract
    show_read "  release_validator" "$events_id" get_release_validator
    show_read "  pending_release_validator" "$events_id" get_pending_release_validator
    show_read "  supported_token_count" "$events_id" supported_token_count
    echo "profile: $profile_id"
    show_read "  version" "$profile_id" version
    show_read "  migrated_to_version" "$profile_id" get_migrated_to_version
    show_read "  pending_upgrade" "$profile_id" get_pending_upgrade
    show_read "  is_paused" "$profile_id" is_paused
    show_read "  admin" "$profile_id" get_admin
    show_read "  events_contract" "$profile_id" get_events_contract
    show_read "  pending_events_contract" "$profile_id" get_pending_events_contract
}

case "$ACTION" in
    deploy-profile)                     cmd_deploy_profile ;;
    deploy-events)                      cmd_deploy_events ;;
    register-token)                     cmd_register_token "$@" ;;
    rotate-admin)                       cmd_rotate_admin "$@" ;;
    upload-profile-wasm)                cmd_upload_wasm profile "$@" ;;
    upload-events-wasm)                 cmd_upload_wasm events "$@" ;;
    prepare-pause-profile)              cmd_prepare_pause profile "$@" ;;
    prepare-pause-events)               cmd_prepare_pause events "$@" ;;
    prepare-propose-upgrade-profile)    cmd_prepare_propose_upgrade profile "$@" ;;
    prepare-propose-upgrade-events)     cmd_prepare_propose_upgrade events "$@" ;;
    verify-proposed-upgrade-profile)    cmd_verify_proposed_upgrade profile "$@" ;;
    verify-proposed-upgrade-events)     cmd_verify_proposed_upgrade events "$@" ;;
    prepare-apply-upgrade-profile)      cmd_prepare_apply_upgrade profile "$@" ;;
    prepare-apply-upgrade-events)       cmd_prepare_apply_upgrade events "$@" ;;
    prepare-migrate-events-page)        cmd_prepare_migrate_events_page "$@" ;;
    prepare-migrate-upgrade-profile)    cmd_prepare_migrate_upgrade profile "$@" ;;
    prepare-migrate-upgrade-events)     cmd_prepare_migrate_upgrade events "$@" ;;
    prepare-cancel-upgrade-profile)     cmd_prepare_cancel_upgrade profile "$@" ;;
    prepare-cancel-upgrade-events)      cmd_prepare_cancel_upgrade events "$@" ;;
    prepare-unpause-profile)            cmd_prepare_unpause profile "$@" ;;
    prepare-unpause-events)             cmd_prepare_unpause events "$@" ;;
    verify-upgrade-profile)             cmd_verify_upgrade profile "$@" ;;
    verify-upgrade-events)              cmd_verify_upgrade events "$@" ;;
    upgrade-status)                     cmd_upgrade_status ;;
    verify)                             cmd_verify ;;
    propose-upgrade-*|apply-upgrade-*|migrate-*|cancel-upgrade-*|pause-*|unpause-*|upgrade-*)
        err "$ACTION cannot submit as the 2-of-3 admin; use the matching prepare-* action, sign sequentially, and send"
        ;;
    "")
        err "no action specified; see the usage at the top of this script and docs/upgrade-runbook.md"
        ;;
    *)
        err "unknown action: $ACTION; see the usage at the top of this script"
        ;;
esac
