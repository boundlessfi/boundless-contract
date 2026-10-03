#!/usr/bin/env bash
#
# verify-fee-trustline.sh: confirm the fee account can receive a token.
#
# The contract moves the platform fee to the fee account inside create_event
# and add_funds. Without an authorized trustline for the token, every such
# call reverts. Run before whitelisting a token and before rotating the fee
# account.
#
# Usage:
#   ./scripts/admin/verify-fee-trustline.sh <testnet|mainnet> <FEE_ACCOUNT> <ASSET>
#
# ASSET is CODE:ISSUER, or "native" for XLM.
#
# FEE_TRUSTLINE_ACCOUNT_FILE reads the Horizon account JSON from a file instead
# of the network (used by scripts/test-admin-scripts.sh).

set -euo pipefail

NETWORK=${1:-""}
FEE_ACCOUNT=${2:-""}
ASSET=${3:-""}

if [[ -z "$NETWORK" || -z "$FEE_ACCOUNT" || -z "$ASSET" ]]; then
    echo "Usage: $0 <testnet|mainnet> <FEE_ACCOUNT> <CODE:ISSUER|native>" >&2
    exit 1
fi
case "$NETWORK" in
    testnet) HORIZON="https://horizon-testnet.stellar.org" ;;
    mainnet) HORIZON="https://horizon.stellar.org" ;;
    *) echo "Unknown network '$NETWORK'." >&2; exit 1 ;;
esac
if [[ ! "$FEE_ACCOUNT" =~ ^G[A-Z2-7]{55}$ ]]; then
    echo "error: fee account must be a G... address (got: $FEE_ACCOUNT)" >&2
    exit 1
fi

if [[ "$ASSET" == "native" ]]; then
    echo "PASS: every account holds XLM."
    exit 0
fi
if [[ ! "$ASSET" =~ ^([A-Za-z0-9]{1,12}):(G[A-Z2-7]{55})$ ]]; then
    echo "error: asset must be CODE:ISSUER or native (got: $ASSET)" >&2
    exit 1
fi
CODE="${BASH_REMATCH[1]}"
ISSUER="${BASH_REMATCH[2]}"

if [[ -n "${FEE_TRUSTLINE_ACCOUNT_FILE:-}" ]]; then
    ACCOUNT_JSON=$(cat "$FEE_TRUSTLINE_ACCOUNT_FILE")
else
    ACCOUNT_JSON=$(curl -sf "$HORIZON/accounts/$FEE_ACCOUNT" || true)
fi
if [[ -z "$ACCOUNT_JSON" ]]; then
    echo "FAIL: fee account $FEE_ACCOUNT does not exist on $NETWORK." >&2
    exit 1
fi

LINE=$(echo "$ACCOUNT_JSON" | jq -c --arg c "$CODE" --arg i "$ISSUER" \
    '[.balances[] | select(.asset_code == $c and .asset_issuer == $i)] | first // empty')
if [[ -z "$LINE" ]]; then
    echo "FAIL: $FEE_ACCOUNT has no trustline for $CODE:$ISSUER." >&2
    exit 1
fi
if [[ "$(echo "$LINE" | jq -r '.is_authorized')" != "true" ]]; then
    echo "FAIL: the $CODE trustline on $FEE_ACCOUNT is not authorized by the issuer." >&2
    exit 1
fi

echo "PASS: $FEE_ACCOUNT holds an authorized $CODE trustline."
