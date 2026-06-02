#!/usr/bin/env bash
# register_token.sh — whitelist a token on boundless-events.
#
# Usage:
#   ./scripts/deploy/register_token.sh <network> <token-contract-address>
#
# Prerequisites (admin's responsibility):
#   The fee_account must hold an active trustline for the token. The contract
#   does NOT pre-flight this; a missing trustline will surface at the first
#   deposit attempt as a transaction failure.
#
# To verify the trustline first (run before this script):
#   stellar account get --account "$FEE_ACCOUNT" --network "$NETWORK"
#   # confirm the asset appears in balances[]
#
# Spec: boundless-platform-contract-prd.md Section 8.

set -euo pipefail

NETWORK="${1:-}"
TOKEN="${2:-}"

if [[ -z "$NETWORK" || -z "$TOKEN" ]]; then
  echo "usage: $0 <network> <token-contract-address>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
if [[ -f "$REPO_ROOT/.env.deploy" ]]; then
  # shellcheck disable=SC1091
  set -a; source "$REPO_ROOT/.env.deploy"; set +a
fi

DEPLOY_RECORD="$REPO_ROOT/deployments/$NETWORK.json"
if [[ ! -f "$DEPLOY_RECORD" ]]; then
  echo "error: no deployment record at $DEPLOY_RECORD; run deploy.sh first" >&2
  exit 1
fi

# Pull the events contract id and fee account from the deployment record.
EVENTS_ID=$(node -e "console.log(require('$DEPLOY_RECORD').events_contract)")
FEE_ACCOUNT=$(node -e "console.log(require('$DEPLOY_RECORD').fee_account)")

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
echo "    fee account:    $FEE_ACCOUNT (operator: verify trustline exists)"
echo "    token contract: $TOKEN"
echo

read -r -p "fee account holds an active trustline for this token? [y/N] " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
  echo "aborted. add the trustline first; the contract does not enforce it." >&2
  exit 1
fi

stellar contract invoke \
  --id "$EVENTS_ID" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  register_supported_token \
  --token "$TOKEN"

echo "==> done. token $TOKEN is now whitelisted."

# Append the token to the deployment record's supported_tokens array.
TMP=$(mktemp)
node -e "
const fs = require('fs');
const path = '$DEPLOY_RECORD';
const rec = require(path);
rec.supported_tokens = Array.from(new Set([...(rec.supported_tokens || []), '$TOKEN']));
fs.writeFileSync(path, JSON.stringify(rec, null, 2) + '\n');
"
