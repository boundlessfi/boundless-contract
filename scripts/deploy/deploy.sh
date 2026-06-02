#!/usr/bin/env bash
# deploy.sh — deploy boundless-events + boundless-profile to a Stellar network.
#
# Usage:
#   ./scripts/deploy/deploy.sh <network>
#
# where <network> is one of: testnet, futurenet, mainnet.
#
# Required env (loaded from .env.deploy or shell):
#   ADMIN_IDENTITY            stellar CLI identity name with admin authority
#   FEE_ACCOUNT               Stellar G-address that receives fees
#   FEE_BPS                   platform fee in basis points (e.g. 250 = 2.5%)
#   BOOTSTRAP_CREDITS         starting credit balance for new profiles (e.g. 10)
#
# Spec: boundless-platform-contract-prd.md Section 12.2.

set -euo pipefail

NETWORK="${1:-}"
if [[ -z "$NETWORK" ]]; then
  echo "usage: $0 <testnet|futurenet|mainnet>" >&2
  exit 1
fi
if [[ "$NETWORK" != "testnet" && "$NETWORK" != "futurenet" && "$NETWORK" != "mainnet" ]]; then
  echo "error: network must be one of testnet, futurenet, mainnet (got: $NETWORK)" >&2
  exit 1
fi

# Load .env.deploy if present (and not already exported).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
if [[ -f "$REPO_ROOT/.env.deploy" ]]; then
  # shellcheck disable=SC1091
  set -a; source "$REPO_ROOT/.env.deploy"; set +a
fi

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
require BOOTSTRAP_CREDITS

# Soroban host on Stellar testnet accepts WebAssembly emitted with the
# reference-types extension; stellar-cli's local simulator only validates that
# extension from 26.0.0 onward. Earlier CLIs reject Rust 1.90.0 wasm32v1-none
# output at simulation time with a "reference-types not enabled" error.
MIN_CLI_MAJOR=26
CLI_VERSION_LINE="$(stellar --version 2>/dev/null | head -1)"
CLI_MAJOR="$(printf '%s' "$CLI_VERSION_LINE" | grep -oE '[0-9]+' | head -1)"
if [[ -z "${CLI_MAJOR:-}" ]]; then
  echo "error: could not parse stellar CLI version from: $CLI_VERSION_LINE" >&2
  exit 1
fi
if (( CLI_MAJOR < MIN_CLI_MAJOR )); then
  cat >&2 <<EOF
error: stellar CLI $CLI_VERSION_LINE is too old.
       Need >= $MIN_CLI_MAJOR.0.0 so the local simulator can validate
       reference-types wasm emitted by Rust 1.90.0.

Upgrade:
  brew upgrade stellar/tap/stellar-cli
  # or
  cargo install --locked stellar-cli@26.1.0
EOF
  exit 1
fi

# Sanity check fee_bps.
if (( FEE_BPS < 0 || FEE_BPS > 5000 )); then
  echo "error: FEE_BPS must be in [0, 5000]; got $FEE_BPS" >&2
  exit 1
fi

ADMIN_ADDR=$(stellar keys address "$ADMIN_IDENTITY")

echo "==> deploying boundless contracts"
echo "    network:           $NETWORK"
echo "    admin identity:    $ADMIN_IDENTITY ($ADMIN_ADDR)"
echo "    fee account:       $FEE_ACCOUNT"
echo "    fee bps:           $FEE_BPS"
echo "    bootstrap credits: $BOOTSTRAP_CREDITS"
echo

# 1. Build both contracts.
echo "==> building contracts"
( cd "$REPO_ROOT/contracts/events"  && stellar contract build )
( cd "$REPO_ROOT/contracts/profile" && stellar contract build )

EVENTS_WASM="$REPO_ROOT/target/wasm32v1-none/release/boundless_events.wasm"
PROFILE_WASM="$REPO_ROOT/target/wasm32v1-none/release/boundless_profile.wasm"

for f in "$EVENTS_WASM" "$PROFILE_WASM"; do
  [[ -f "$f" ]] || { echo "error: wasm not found at $f" >&2; exit 1; }
done

# 2. Deploy boundless-profile first (events constructor needs its address).
echo
echo "==> deploying boundless-profile"
PROFILE_ID=$(stellar contract deploy \
  --wasm "$PROFILE_WASM" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  --admin "$ADMIN_ADDR" \
  --default_bootstrap_credits "$BOOTSTRAP_CREDITS")
echo "    profile contract id: $PROFILE_ID"

# 3. Deploy boundless-events pointing at the profile contract.
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

# 4. Wire profile to recognize the events contract.
echo
echo "==> wiring profile.set_events_contract"
stellar contract invoke \
  --id "$PROFILE_ID" \
  --source "$ADMIN_IDENTITY" \
  --network "$NETWORK" \
  -- \
  set_events_contract \
  --new_addr "$EVENTS_ID"

# 5. Persist deployment record.
DEPLOY_RECORD="$REPO_ROOT/deployments/$NETWORK.json"
mkdir -p "$(dirname "$DEPLOY_RECORD")"
cat > "$DEPLOY_RECORD" <<EOF
{
  "network": "$NETWORK",
  "deployed_at": "$(date -u +%FT%TZ)",
  "admin_address": "$ADMIN_ADDR",
  "fee_account": "$FEE_ACCOUNT",
  "fee_bps": $FEE_BPS,
  "bootstrap_credits": $BOOTSTRAP_CREDITS,
  "events_contract": "$EVENTS_ID",
  "profile_contract": "$PROFILE_ID"
}
EOF

echo
echo "==> done. summary written to $DEPLOY_RECORD"
echo
echo "set these in the nestjs deployment env:"
echo "  BOUNDLESS_EVENTS_CONTRACT_ADDRESS=$EVENTS_ID"
echo "  BOUNDLESS_PROFILE_CONTRACT_ADDRESS=$PROFILE_ID"
echo
echo "next: ./scripts/deploy/register_token.sh $NETWORK <token-address>"
