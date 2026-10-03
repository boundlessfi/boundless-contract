#!/usr/bin/env bash
# deploy.sh: deploy boundless-profile and boundless-events to testnet or
# futurenet, wire them together, and write deployments/<network>.json.
#
# Usage:
#   ./scripts/deploy/deploy.sh <testnet|futurenet>
#
# Required env (loaded from .env.deploy or the shell):
#   ADMIN_IDENTITY            stellar CLI identity that deploys and holds admin
#   FEE_ACCOUNT               Stellar G-address that receives fees
#   FEE_BPS                   platform fee in basis points (e.g. 250 = 2.5%)
#
# Mainnet deploys use ./deploy_mainnet.sh instead (docs/DEPLOYMENT.md).

set -euo pipefail

NETWORK="${1:-}"
if [[ -z "$NETWORK" ]]; then
  echo "usage: $0 <testnet|futurenet>" >&2
  exit 1
fi
if [[ "$NETWORK" == "mainnet" ]]; then
  echo "error: mainnet deploys go through ./deploy_mainnet.sh deploy-profile and deploy-events (docs/DEPLOYMENT.md)" >&2
  exit 1
fi
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/deploy-env.sh
source "$SCRIPT_DIR/../lib/deploy-env.sh"
load_env_deploy "$REPO_ROOT/.env.deploy"
require_network "$NETWORK"

require() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "error: $name must be set (export it or write it to .env.deploy)" >&2
    exit 1
  fi
}
require ADMIN_IDENTITY
require FEE_ACCOUNT
require FEE_BPS
require_g_address FEE_ACCOUNT "$FEE_ACCOUNT"
if [[ ! "$FEE_BPS" =~ ^[0-9]+$ ]]; then
  echo "error: FEE_BPS must be a whole number of basis points (got: $FEE_BPS)" >&2
  exit 1
fi

# CI builds with stellar CLI 28.1.0 against soroban-sdk 28; older CLIs are
# untested with these contracts.
MIN_CLI_MAJOR=28
CLI_VERSION_LINE="$(stellar --version 2>/dev/null | head -1)"
CLI_MAJOR="$(printf '%s' "$CLI_VERSION_LINE" | grep -oE '[0-9]+' | head -1)"
if [[ -z "${CLI_MAJOR:-}" ]]; then
  echo "error: could not parse stellar CLI version from: $CLI_VERSION_LINE" >&2
  exit 1
fi
if (( CLI_MAJOR < MIN_CLI_MAJOR )); then
  echo "error: stellar CLI $CLI_VERSION_LINE is too old; CI uses 28.1.0 (cargo install --locked stellar-cli@28.1.0)" >&2
  exit 1
fi

# The constructor accepts any value, but every create_event reverts above the
# contract's MAX_FEE_BPS of 1000 (10%).
if (( FEE_BPS < 0 || FEE_BPS > 1000 )); then
  echo "error: FEE_BPS must be in [0, 1000] (the contract caps fees at 10%); got $FEE_BPS" >&2
  exit 1
fi

ADMIN_ADDR=$(stellar keys address "$ADMIN_IDENTITY")

echo "==> deploying boundless contracts"
echo "    network:           $NETWORK"
echo "    admin identity:    $ADMIN_IDENTITY ($ADMIN_ADDR)"
echo "    fee account:       $FEE_ACCOUNT"
echo "    fee bps:           $FEE_BPS"
echo

# The testnet feature zeroes the upgrade timelock so test upgrades can apply at
# once; mainnet builds (deploy_mainnet.sh) leave it off.
echo "==> building contracts with --features testnet"
"$REPO_ROOT/scripts/build-release.sh" --package boundless-events --features testnet
"$REPO_ROOT/scripts/build-release.sh" --package boundless-profile --features testnet

EVENTS_WASM="$REPO_ROOT/target/wasm32v1-none/release/boundless_events.wasm"
PROFILE_WASM="$REPO_ROOT/target/wasm32v1-none/release/boundless_profile.wasm"

for f in "$EVENTS_WASM" "$PROFILE_WASM"; do
  [[ -f "$f" ]] || { echo "error: wasm not found at $f" >&2; exit 1; }
done

# Profile first: the events constructor takes its address.
echo
echo "==> deploying boundless-profile"
PROFILE_ID=$(stellar contract deploy \
  --wasm "$PROFILE_WASM" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  --admin "$ADMIN_ADDR")
echo "    profile contract id: $PROFILE_ID"

echo
echo "==> deploying boundless-events"
EVENTS_ID=$(stellar contract deploy \
  --wasm "$EVENTS_WASM" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  --admin "$ADMIN_ADDR" \
  --fee_account "$FEE_ACCOUNT" \
  --fee_bps "$FEE_BPS" \
  --profile_contract "$PROFILE_ID")
echo "    events contract id:  $EVENTS_ID"

echo
echo "==> wiring profile.set_events_contract"
stellar contract invoke \
  --id "$PROFILE_ID" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  set_events_contract \
  --new_addr "$EVENTS_ID"

DEPLOY_RECORD="$REPO_ROOT/deployments/$NETWORK.json"
mkdir -p "$(dirname "$DEPLOY_RECORD")"
cat > "$DEPLOY_RECORD" <<EOF
{
  "network": "$NETWORK",
  "deployed_at": "$(date -u +%FT%TZ)",
  "admin_address": "$ADMIN_ADDR",
  "fee_account": "$FEE_ACCOUNT",
  "fee_bps": $FEE_BPS,
  "events_contract": "$EVENTS_ID",
  "profile_contract": "$PROFILE_ID",
  "deployer_identity": "$ADMIN_IDENTITY"
}
EOF

echo
echo "==> done. local record written to $DEPLOY_RECORD"
echo
echo "set these in the boundless-nestjs deployment env:"
echo "  BOUNDLESS_EVENTS_CONTRACT_ADDRESS=$EVENTS_ID"
echo "  BOUNDLESS_PROFILE_CONTRACT_ADDRESS=$PROFILE_ID"
echo
echo "next: ./scripts/deploy/register_token.sh $NETWORK <token-address> <CODE:ISSUER|native>"
