#!/usr/bin/env bash
# register_token.sh: whitelist a token on boundless-events while the deployer
# identity still holds admin (before the multisig rotation).
#
# Usage:
#   ./scripts/deploy/register_token.sh <network> <token-contract-address> <CODE:ISSUER|native>
#
# The asset must be the one the token contract wraps, and the fee account must
# hold an authorized trustline for it: the contract checks neither, and a
# missing trustline makes every create_event and add_funds for the token
# revert. Both are verified before anything is signed.

set -euo pipefail

NETWORK="${1:-}"
TOKEN="${2:-}"
ASSET="${3:-}"

if [[ -z "$NETWORK" || -z "$TOKEN" || -z "$ASSET" ]]; then
  echo "usage: $0 <network> <token-contract-address> <CODE:ISSUER|native>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/deploy-env.sh
source "$SCRIPT_DIR/../lib/deploy-env.sh"
load_env_deploy "$REPO_ROOT/.env.deploy"
require_network "$NETWORK"
require_c_address token "$TOKEN"

DEPLOY_RECORD="$REPO_ROOT/deployments/$NETWORK.json"
if [[ ! -f "$DEPLOY_RECORD" ]]; then
  echo "error: no deployment record at $DEPLOY_RECORD; run deploy.sh first" >&2
  exit 1
fi

# Pull the events contract id and fee account from the deployment record.
EVENTS_ID=$(record_field "$DEPLOY_RECORD" events_contract)
FEE_ACCOUNT=$(record_field "$DEPLOY_RECORD" fee_account)
require_c_address events_contract "$EVENTS_ID"
require_g_address fee_account "$FEE_ACCOUNT"

require() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "error: $name must be set" >&2
    exit 1
  fi
}
require ADMIN_IDENTITY

echo "==> registering token on boundless-events"
echo "    network:        $NETWORK"
echo "    events:         $EVENTS_ID"
echo "    fee account:    $FEE_ACCOUNT"
echo "    token contract: $TOKEN"
echo

WRAPPED=$(stellar contract id asset --asset "$ASSET" --network "$NETWORK")
if [[ "$WRAPPED" != "$TOKEN" ]]; then
  echo "error: $ASSET is wrapped by $WRAPPED, not $TOKEN" >&2
  exit 1
fi
"$SCRIPT_DIR/../admin/verify-fee-trustline.sh" "$NETWORK" "$FEE_ACCOUNT" "$ASSET"

stellar contract invoke \
  --id "$EVENTS_ID" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  register_supported_token \
  --token "$TOKEN"

echo "==> done. token $TOKEN is now whitelisted."

TMP=$(mktemp)
jq --arg t "$TOKEN" \
  '.supported_tokens = ((.supported_tokens // []) | if index($t) then . else . + [$t] end)' \
  "$DEPLOY_RECORD" > "$TMP"
mv "$TMP" "$DEPLOY_RECORD"
