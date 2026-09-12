#!/bin/bash
#
# verify-fee-trustline.sh: verify that the fee account holds active, authorized
# trustlines for all supported/candidate tokens before `register_supported_token`
# or `set_fee_account` is executed.
#
# Threat Model Reference: DoS.15 (Process control for unverified fee trustlines).
# Without a valid trustline, fee collection in `deposit_with_fee_at` /
# `release_with_fee_at` will revert, causing `create_event`, `add_funds`, and
# crowdfunding `claim_milestone` to fail.
#
# Usage:
#   ./scripts/admin/verify-fee-trustline.sh <FEE_ACCOUNT_G_ADDRESS> [testnet|mainnet] [ASSET_CODE:ISSUER ...]
#
# Example:
#   ./scripts/admin/verify-fee-trustline.sh GA... testnet USDC:GB...

set -euo pipefail

if ! command -v curl &>/dev/null; then
    echo "Error: curl is required." >&2
    exit 1
fi
if ! command -v jq &>/dev/null; then
    echo "Error: jq is required (e.g., brew install jq or apt install jq)." >&2
    exit 1
fi

FEE_ACCOUNT=${1:-""}
NETWORK=${2:-"testnet"}
shift 2 || true
ASSETS=("$@")

if [[ -z "$FEE_ACCOUNT" ]]; then
    echo "Usage: $0 <FEE_ACCOUNT_G_ADDRESS> [testnet|mainnet] [ASSET_CODE:ISSUER ...]" >&2
    exit 1
fi

case "$NETWORK" in
    testnet) HORIZON="https://horizon-testnet.stellar.org" ;;
    mainnet) HORIZON="https://horizon.stellar.org" ;;
    *)       echo "Unknown network '$NETWORK'." >&2; exit 1 ;;
esac

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; NC='\033[0m'

echo -e "${YELLOW}Verifying fee account ${FEE_ACCOUNT} on ${NETWORK}...${NC}"

ACCOUNT_JSON=$(curl -sf "$HORIZON/accounts/$FEE_ACCOUNT" || true)
if [[ -z "$ACCOUNT_JSON" ]]; then
    echo -e "${RED}Error: Fee account ${FEE_ACCOUNT} not found on ${NETWORK}.${NC}" >&2
    exit 1
fi

PASS=0
FAIL=0

echo -e "  ${GREEN}✓${NC} Account exists on ${NETWORK}"
PASS=$((PASS + 1))

# Inspect active balances on the fee account
BALANCES=$(echo "$ACCOUNT_JSON" | jq -r '.balances')

if [[ ${#ASSETS[@]} -eq 0 ]]; then
    echo -e "\n${YELLOW}Discovered trustlines/balances on fee account:${NC}"
    echo "$BALANCES" | jq -r '.[] | "  - " + (if .asset_type == "native" then "XLM (native)" else (.asset_code + ":" + .asset_issuer) end) + " (balance: " + .balance + ", authorized: " + (.is_authorized // true | tostring) + ")"'
else
    echo -e "\n${YELLOW}Asserting required trustlines:${NC}"
    for ASSET in "${ASSETS[@]}"; do
        if [[ "$ASSET" == "XLM" || "$ASSET" == "native" ]]; then
            HAS_NATIVE=$(echo "$BALANCES" | jq -r '[.[] | select(.asset_type == "native")] | length')
            if [[ "$HAS_NATIVE" -gt 0 ]]; then
                echo -e "  ${GREEN}✓${NC} XLM (native balance available)"
                PASS=$((PASS + 1))
            else
                echo -e "  ${RED}✗${NC} Missing native XLM balance on fee account"
                FAIL=$((FAIL + 1))
            fi
            continue
        fi

        CODE="${ASSET%%:*}"
        ISSUER="${ASSET##*:}"

        # Match either exact code+issuer or asset code
        MATCH=$(echo "$BALANCES" | jq -r --arg code "$CODE" --arg issuer "$ISSUER" \
            '[.[] | select(.asset_code == $code and .asset_issuer == $issuer)] | .[0]')

        if [[ -n "$MATCH" && "$MATCH" != "null" ]]; then
            IS_AUTH=$(echo "$MATCH" | jq -r '.is_authorized // false')
            if [[ "$IS_AUTH" == "true" ]]; then
                echo -e "  ${GREEN}✓${NC} Trustline active & authorized: ${ASSET}"
                PASS=$((PASS + 1))
            else
                echo -e "  ${RED}✗${NC} Trustline exists but is NOT authorized: ${ASSET}"
                FAIL=$((FAIL + 1))
            fi
        else
            echo -e "  ${RED}✗${NC} Missing trustline for: ${ASSET}"
            FAIL=$((FAIL + 1))
        fi
    done
fi

echo
if [[ $FAIL -gt 0 ]]; then
    echo -e "${RED}FAIL: $FAIL check(s) failed; $PASS passed.${NC}" >&2
    echo "Do NOT register this token or rotate fee account until trustlines are established and authorized." >&2
    exit 1
fi

echo -e "${GREEN}PASS: Fee account trustline verification successful.${NC}"
