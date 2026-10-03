#!/bin/bash
#
# verify-multisig.sh: pre-flight gate before transferring admin authority.
#
# Given a candidate multi-sig G-address, this script queries the network and
# asserts the live signer configuration matches docs/admin-custody-policy.md:
#
#   - exactly 3 signers + master key disabled
#   - every signer is an ed25519 account key (no pre-auth tx or hash-x
#     signers, which would let a single transaction or preimage stand in
#     for a person)
#   - each signer has weight 1
#   - low threshold = 0 (no read-only signers allowed)
#   - med + high thresholds = 2 (2-of-3 quorum)
#
# Run on testnet first (the practice multisig). After mainnet provisioning,
# run it with `mainnet` against the production multi-sig BEFORE the
# `set_admin` call (deploy_mainnet.sh rotate-admin runs it for you).
#
# Usage:
#   ./scripts/admin/verify-multisig.sh <MULTISIG_G_ADDRESS> [testnet|mainnet]
#
# EXPECTED_SIGNERS (comma-separated G-addresses) additionally requires the
# signer set to be exactly that roster. MULTISIG_ACCOUNT_FILE reads the
# Horizon account JSON from a file instead of the network (used by
# scripts/test-admin-scripts.sh).

set -euo pipefail

if ! command -v stellar &>/dev/null; then
    echo "Error: stellar CLI not installed." >&2
    exit 1
fi
if ! command -v jq &>/dev/null; then
    echo "Error: jq not installed (brew install jq)." >&2
    exit 1
fi

MULTISIG=${1:-""}
NETWORK=${2:-"testnet"}

if [[ -z "$MULTISIG" ]]; then
    echo "Usage: $0 <MULTISIG_G_ADDRESS> [testnet|mainnet]" >&2
    exit 1
fi
if [[ ! "$MULTISIG" =~ ^G[A-Z2-7]{55}$ ]]; then
    echo "Error: '$MULTISIG' is not a G... account address." >&2
    exit 1
fi

case "$NETWORK" in
    testnet) HORIZON="https://horizon-testnet.stellar.org" ;;
    mainnet) HORIZON="https://horizon.stellar.org" ;;
    *)       echo "Unknown network '$NETWORK'." >&2; exit 1 ;;
esac

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; NC='\033[0m'

echo -e "${YELLOW}Verifying multi-sig ${MULTISIG} on ${NETWORK}...${NC}"

if [[ -n "${MULTISIG_ACCOUNT_FILE:-}" ]]; then
    ACCOUNT_JSON=$(cat "$MULTISIG_ACCOUNT_FILE")
else
    ACCOUNT_JSON=$(curl -sf "$HORIZON/accounts/$MULTISIG" || true)
fi
if [[ -z "$ACCOUNT_JSON" ]]; then
    echo -e "${RED}Account not found on $NETWORK.${NC}" >&2
    exit 1
fi

PASS=0
FAIL=0

assert() {
    local label=$1 actual=$2 expected=$3
    if [[ "$actual" == "$expected" ]]; then
        echo -e "  ${GREEN}✓${NC} $label: $actual"
        PASS=$((PASS + 1))
    else
        echo -e "  ${RED}✗${NC} $label: got '$actual', expected '$expected'"
        FAIL=$((FAIL + 1))
    fi
}

# Master key must be neutralized so the account can only act via the
# multi-sig signers. The master key appears in `signers` with key == the
# account id; weight 0 means it's been disabled via set_options.
MASTER_WEIGHT=$(echo "$ACCOUNT_JSON" | \
    jq -r --arg id "$MULTISIG" '.signers[] | select(.key == $id) | .weight')
assert "master key weight (0 = disabled)" "$MASTER_WEIGHT" "0"

# Thresholds: low=0 (no read-only quorum allowed), med=2, high=2.
LOW=$(echo "$ACCOUNT_JSON" | jq -r '.thresholds.low_threshold')
MED=$(echo "$ACCOUNT_JSON" | jq -r '.thresholds.med_threshold')
HIGH=$(echo "$ACCOUNT_JSON" | jq -r '.thresholds.high_threshold')
assert "low_threshold"  "$LOW"  "0"
assert "med_threshold"  "$MED"  "2"
assert "high_threshold" "$HIGH" "2"

# Signers: expect exactly 3 with weight 1 each (other than the disabled master).
SIGNERS_COUNT=$(echo "$ACCOUNT_JSON" | jq '[.signers[] | select(.type != "ed25519_public_key" or .weight > 0)] | length')
assert "non-zero-weight signer count" "$SIGNERS_COUNT" "3"

# Each non-master signer weight should be 1 (uniform, no privileged signer).
WEIGHTS=$(echo "$ACCOUNT_JSON" | jq -r '[.signers[] | select(.weight > 0) | .weight] | join(",")')
assert "signer weights" "$WEIGHTS" "1,1,1"

# A pre-auth transaction or hash-x signer counts toward the quorum without a
# person behind it.
NON_KEY=$(echo "$ACCOUNT_JSON" | jq '[.signers[] | select(.weight > 0 and .type != "ed25519_public_key")] | length')
assert "non-key signers (pre-auth tx, hash-x, signed payload)" "$NON_KEY" "0"

BAD_KEYS=$(echo "$ACCOUNT_JSON" | \
    jq --arg id "$MULTISIG" '[.signers[] | select(.weight > 0) | .key | select((test("^G[A-Z2-7]{55}$") | not) or . == $id)] | length')
assert "signers that are not distinct G-addresses" "$BAD_KEYS" "0"

if [[ -n "${EXPECTED_SIGNERS:-}" ]]; then
    ACTUAL=$(echo "$ACCOUNT_JSON" | jq -r '[.signers[] | select(.weight > 0) | .key] | sort | join(",")')
    WANTED=$(echo "$EXPECTED_SIGNERS" | tr ',' '\n' | sed '/^$/d' | sort | paste -sd, -)
    assert "signer roster" "$ACTUAL" "$WANTED"
fi

echo
echo -e "${YELLOW}Signer addresses on file:${NC}"
echo "$ACCOUNT_JSON" | jq -r '.signers[] | select(.weight > 0) | "  " + .key + " (weight " + (.weight|tostring) + ", type " + .type + ")"'

echo
if [[ $FAIL -gt 0 ]]; then
    echo -e "${RED}FAIL: $FAIL check(s) failed; $PASS passed.${NC}" >&2
    echo "Do NOT rotate admin authority to this address. Re-provision per docs/admin-custody-policy.md." >&2
    exit 1
fi

echo -e "${GREEN}PASS: all $PASS checks passed.${NC}"
echo
echo "Next steps:"
echo "  1. Compare the printed signer addresses against the founder's roster."
echo "  2. Confirm each signer can sign a test transaction (drill)."
echo "  3. Only then rotate admin per docs/DEPLOYMENT.md (Rotate admin to the multisig)."
