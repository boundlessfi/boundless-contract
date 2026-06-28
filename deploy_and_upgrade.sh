#!/bin/bash

# Boundless contract deploy + upgrade helper.
#
# Usage:
#   ./deploy_and_upgrade.sh deploy <events|profile> [network] [source_account]
#       Build + deploy a fresh contract (one of: events, profile).
#       The constructor must be supplied via env vars; see the runbook.
#
#   ./deploy_and_upgrade.sh propose-upgrade <events|profile> <new_version> \
#       [network] [source_account]
#       Build, upload the new wasm, then call propose_upgrade(wasm_hash,
#       new_version). The timelock (~1 day) starts now.
#
#   ./deploy_and_upgrade.sh apply-upgrade <events|profile> [network] [source_account]
#       After the timelock has elapsed (and before expires_at_ledger), call
#       apply_upgrade(). The contract swaps to the proposed wasm and bumps
#       its on-chain version label.
#
#   ./deploy_and_upgrade.sh cancel-pending-upgrade <events|profile> \
#       [network] [source_account]
#       Admin prunes a stale proposal so a fresh one can be queued.
#
#   ./deploy_and_upgrade.sh migrate <events|profile> [network] [source_account]
#       Admin runs the post-upgrade one-shot migration for the current
#       Version. Guarded by MigratedToVersion so a second call rejects with
#       MigrationAlreadyApplied.
#
#   ./deploy_and_upgrade.sh status <events|profile> [network]
#       Print on-chain version + pending upgrade + migrated-to version.
#
# Spec: docs/mainnet-deploy-runbook.md
#       docs/audit-2026-06-stellar-skill.md (H6 timelocked upgrade)

set -euo pipefail

ACTION=${1:-"status"}
CONTRACT_KIND=${2:-"events"}
NETWORK=${4:-"testnet"}
SOURCE_ACCOUNT=${5:-"alice"}

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

if ! command -v stellar &> /dev/null; then
    echo -e "${RED}Error: stellar CLI is not installed.${NC}"
    echo "Install from https://soroban.stellar.org/docs/getting-started/setup"
    exit 1
fi

case "$CONTRACT_KIND" in
    events)
        WASM_PATH="target/wasm32v1-none/release/boundless_events.wasm"
        CONTRACT_ID_FILE=".stellar/events_contract_id_${NETWORK}.txt"
        ;;
    profile)
        WASM_PATH="target/wasm32v1-none/release/boundless_profile.wasm"
        CONTRACT_ID_FILE=".stellar/profile_contract_id_${NETWORK}.txt"
        ;;
    *)
        echo -e "${RED}Unknown contract kind '$CONTRACT_KIND'. Use 'events' or 'profile'.${NC}"
        exit 1
        ;;
esac

build_contract() {
    echo "Building $CONTRACT_KIND..."
    # Testnet builds enable the `testnet` feature → zero upgrade timelock for
    # fast iteration. Both events and profile expose this feature. Mainnet (and
    # any non-testnet build) MUST omit it; the default keeps the full
    # audit-mandated timelock, so this fails safe.
    if [ "$NETWORK" = "testnet" ]; then
        stellar contract build --package "boundless-$CONTRACT_KIND" --features testnet
    else
        stellar contract build
    fi
}

read_contract_id() {
    # Errors go to stderr so callers using $(read_contract_id) don't capture
    # the diagnostic into the variable.
    if [ ! -f "$CONTRACT_ID_FILE" ]; then
        echo -e "${RED}Contract id file $CONTRACT_ID_FILE not found.${NC}" >&2
        echo "Run 'deploy' first or create the file manually with the C-address." >&2
        exit 1
    fi
    cat "$CONTRACT_ID_FILE"
}

deploy_contract() {
    echo -e "${YELLOW}Deploying boundless-$CONTRACT_KIND to $NETWORK...${NC}"
    build_contract

    # Constructor args are runbook-specific. We refuse to invent defaults so
    # an accidental rerun cannot reinitialize the contract with wrong values.
    echo -e "${YELLOW}NOTE: This script does NOT pass constructor args.${NC}"
    echo "After the deploy succeeds, invoke the constructor manually per the runbook:"
    echo "  docs/mainnet-deploy-runbook.md (events: admin, fee_account, fee_bps, profile_contract)"
    echo "  docs/mainnet-deploy-runbook.md (profile: admin, default_bootstrap_credits)"
    echo ""

    CONTRACT_ID=$(stellar contract deploy \
        --wasm "$WASM_PATH" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK")

    mkdir -p .stellar
    echo "$CONTRACT_ID" > "$CONTRACT_ID_FILE"
    echo -e "${GREEN}Deployed.${NC}"
    echo "Contract id: $CONTRACT_ID"
    echo "Saved to:    $CONTRACT_ID_FILE"
}

propose_upgrade() {
    # Usage check: needs new_version arg in slot 3.
    local NEW_VERSION=${3:-""}
    if [ -z "$NEW_VERSION" ]; then
        echo -e "${RED}propose-upgrade requires <new_version> in arg 3.${NC}"
        echo "Example: ./deploy_and_upgrade.sh propose-upgrade events 0.3.0 testnet alice"
        exit 1
    fi

    echo -e "${YELLOW}Proposing upgrade for boundless-$CONTRACT_KIND on $NETWORK to $NEW_VERSION...${NC}"
    build_contract

    local CONTRACT_ID
    CONTRACT_ID=$(read_contract_id)

    echo "Uploading new wasm..."
    local WASM_HASH
    WASM_HASH=$(stellar contract upload \
        --source-account "$SOURCE_ACCOUNT" \
        --wasm "$WASM_PATH" \
        --network "$NETWORK")
    echo "New wasm hash: $WASM_HASH"

    echo "Calling propose_upgrade..."
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        -- \
        propose_upgrade \
        --new_wasm_hash "$WASM_HASH" \
        --new_version "$NEW_VERSION"

    echo -e "${GREEN}Proposal recorded.${NC}"
    echo "The timelock is ~1 day at 5s/ledger (17_280 ledgers)."
    echo "Run 'apply-upgrade' once the timelock has elapsed."
    echo "Run 'status' to inspect available_at / expires_at ledger sequence."
}

apply_upgrade() {
    echo -e "${YELLOW}Applying queued upgrade for boundless-$CONTRACT_KIND on $NETWORK...${NC}"
    local CONTRACT_ID
    CONTRACT_ID=$(read_contract_id)

    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        -- \
        apply_upgrade

    echo -e "${GREEN}Upgrade applied.${NC}"
    echo "Run 'status' to confirm the new version label."
    echo "Run 'migrate' if the upgrade requires a data migration."
}

cancel_pending_upgrade() {
    echo -e "${YELLOW}Cancelling pending upgrade for boundless-$CONTRACT_KIND on $NETWORK...${NC}"
    local CONTRACT_ID
    CONTRACT_ID=$(read_contract_id)

    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        -- \
        cancel_pending_upgrade
    echo -e "${GREEN}Pending upgrade cleared.${NC}"
}

migrate() {
    echo -e "${YELLOW}Running post-upgrade migrate for boundless-$CONTRACT_KIND on $NETWORK...${NC}"
    local CONTRACT_ID
    CONTRACT_ID=$(read_contract_id)

    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        -- \
        migrate
    echo -e "${GREEN}Migrate completed.${NC}"
}

show_status() {
    local CONTRACT_ID
    CONTRACT_ID=$(read_contract_id)
    echo -e "${YELLOW}Status for boundless-$CONTRACT_KIND on $NETWORK${NC}"
    echo "Contract id: $CONTRACT_ID"
    echo ""
    echo "On-chain version:"
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        --send no \
        -- \
        version || true
    echo ""
    echo "Pending upgrade (if any):"
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        --send no \
        -- \
        get_pending_upgrade || true
    echo ""
    echo "Last migrated-to version:"
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source "$SOURCE_ACCOUNT" \
        --network "$NETWORK" \
        --send no \
        -- \
        get_migrated_to_version || true
}

case "$ACTION" in
    deploy)              deploy_contract ;;
    propose-upgrade)     propose_upgrade "$@" ;;
    apply-upgrade)       apply_upgrade ;;
    cancel-pending-upgrade) cancel_pending_upgrade ;;
    migrate)             migrate ;;
    status)              show_status ;;
    upgrade)
        echo -e "${RED}'upgrade' is gone. The 2026-06 audit replaced it with a timelock flow:${NC}"
        echo "  1. propose-upgrade <contract-kind> <new_version>"
        echo "  2. wait ~1 day (the on-chain UPGRADE_TIMELOCK_LEDGERS)"
        echo "  3. apply-upgrade <contract-kind>"
        echo "  4. migrate <contract-kind>  (if the upgrade ships storage changes)"
        echo ""
        echo "See docs/audit-2026-06-stellar-skill.md (H6) for the rationale."
        exit 1
        ;;
    *)
        echo -e "${RED}Unknown action '$ACTION'.${NC}"
        echo "Use: deploy | propose-upgrade | apply-upgrade | cancel-pending-upgrade | migrate | status"
        exit 1
        ;;
esac

echo -e "${GREEN}Done.${NC}"
