#!/usr/bin/env bash
# verify.sh — read-only post-deployment checks.
#
# Usage:
#   ./scripts/deploy/verify.sh <network>
#
# Reads each contract's admin / fee account / profile binding and prints them
# alongside the deployment record so an operator can confirm they match.

set -euo pipefail

NETWORK="${1:-}"
if [[ -z "$NETWORK" ]]; then
  echo "usage: $0 <network>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEPLOY_RECORD="$REPO_ROOT/deployments/$NETWORK.json"

# Load .env.deploy for ADMIN_IDENTITY (the CLI requires --source-account even
# on read-only invokes; the call is unsigned simulate-only).
if [[ -f "$REPO_ROOT/.env.deploy" ]]; then
  # shellcheck disable=SC1091
  set -a; source "$REPO_ROOT/.env.deploy"; set +a
fi
SOURCE="${ADMIN_IDENTITY:-default}"

if [[ ! -f "$DEPLOY_RECORD" ]]; then
  echo "error: no deployment record at $DEPLOY_RECORD" >&2
  exit 1
fi

EVENTS_ID=$(node -e "console.log(require('$DEPLOY_RECORD').events_contract)")
PROFILE_ID=$(node -e "console.log(require('$DEPLOY_RECORD').profile_contract)")

echo "==> deployment record"
cat "$DEPLOY_RECORD"
echo

echo "==> on-chain events contract state"
echo "    admin:            $(stellar contract invoke --id "$EVENTS_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_admin)"
echo "    fee_account:      $(stellar contract invoke --id "$EVENTS_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_fee_account)"
echo "    fee_bps:          $(stellar contract invoke --id "$EVENTS_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_fee_bps)"
echo "    profile_contract: $(stellar contract invoke --id "$EVENTS_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_profile_contract)"
echo "    is_paused:        $(stellar contract invoke --id "$EVENTS_ID" --source-account "$SOURCE" --network "$NETWORK" -- is_paused)"
echo

echo "==> on-chain profile contract state"
echo "    admin:            $(stellar contract invoke --id "$PROFILE_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_admin)"
echo "    events_contract:  $(stellar contract invoke --id "$PROFILE_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_events_contract)"
echo "    default_credits:  $(stellar contract invoke --id "$PROFILE_ID" --source-account "$SOURCE" --network "$NETWORK" -- get_default_bootstrap_credits)"
echo "    is_paused:        $(stellar contract invoke --id "$PROFILE_ID" --source-account "$SOURCE" --network "$NETWORK" -- is_paused)"
