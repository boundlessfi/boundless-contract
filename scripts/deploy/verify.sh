#!/usr/bin/env bash
# verify.sh: read-only post-deployment checks.
#
# Usage:
#   ./scripts/deploy/verify.sh <testnet|futurenet|mainnet>
#
# Prints the local deployment record next to each contract's version, admin,
# fee settings, bindings, pause flag and release validator so an operator can
# confirm they match.

set -euo pipefail

NETWORK="${1:-}"
if [[ -z "$NETWORK" ]]; then
  echo "usage: $0 <testnet|futurenet|mainnet>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_RECORD="$REPO_ROOT/deployments/$NETWORK.json"

# The CLI wants a source account even for a simulated read; nothing is signed.
# shellcheck source=../lib/deploy-env.sh
source "$SCRIPT_DIR/../lib/deploy-env.sh"
require_network "$NETWORK"
load_env_deploy "$REPO_ROOT/.env.deploy"
SOURCE="${ADMIN_IDENTITY:-default}"

if [[ ! -f "$DEPLOY_RECORD" ]]; then
  echo "error: no deployment record at $DEPLOY_RECORD" >&2
  exit 1
fi

EVENTS_ID=$(record_field "$DEPLOY_RECORD" events_contract)
PROFILE_ID=$(record_field "$DEPLOY_RECORD" profile_contract)
require_c_address events_contract "$EVENTS_ID"
require_c_address profile_contract "$PROFILE_ID"

echo "==> deployment record"
cat "$DEPLOY_RECORD"
echo

read_call() {
  local id="$1"
  shift
  stellar contract invoke --id "$id" --source-account "$SOURCE" --network "$NETWORK" --send no -- "$@"
}

echo "==> on-chain events contract state"
echo "    version:           $(read_call "$EVENTS_ID" version)"
echo "    migrated_to:       $(read_call "$EVENTS_ID" get_migrated_to_version)"
echo "    admin:             $(read_call "$EVENTS_ID" get_admin)"
echo "    fee_account:       $(read_call "$EVENTS_ID" get_fee_account)"
echo "    fee_bps:           $(read_call "$EVENTS_ID" get_fee_bps)"
echo "    profile_contract:  $(read_call "$EVENTS_ID" get_profile_contract)"
echo "    release_validator: $(read_call "$EVENTS_ID" get_release_validator)"
echo "    is_paused:         $(read_call "$EVENTS_ID" is_paused)"
echo

echo "==> on-chain profile contract state"
echo "    version:           $(read_call "$PROFILE_ID" version)"
echo "    migrated_to:       $(read_call "$PROFILE_ID" get_migrated_to_version)"
echo "    admin:             $(read_call "$PROFILE_ID" get_admin)"
echo "    events_contract:   $(read_call "$PROFILE_ID" get_events_contract)"
echo "    is_paused:         $(read_call "$PROFILE_ID" is_paused)"
