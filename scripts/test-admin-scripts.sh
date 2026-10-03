#!/usr/bin/env bash
# Runs the admin pre-flight checks against recorded Horizon fixtures, so a
# check that silently passes a bad configuration fails CI.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA="$ROOT/scripts/testdata/horizon"
# shellcheck source=testdata/horizon/ids.env
source "$DATA/ids.env"
FAILED=0

expect() {
    local want=$1 label=$2
    shift 2
    if "$@" >/dev/null 2>&1; then got=pass; else got=fail; fi
    if [[ "$got" == "$want" ]]; then
        echo "PASS $label ($got)"
    else
        echo "FAIL $label: expected $want, got $got"
        FAILED=1
    fi
}

multisig() {
    MULTISIG_ACCOUNT_FILE="$DATA/$1" "$ROOT/scripts/admin/verify-multisig.sh" "$MULTISIG" testnet
}
roster() {
    EXPECTED_SIGNERS=$2 MULTISIG_ACCOUNT_FILE="$DATA/$1" \
        "$ROOT/scripts/admin/verify-multisig.sh" "$MULTISIG" testnet
}
fee() {
    FEE_TRUSTLINE_ACCOUNT_FILE="$DATA/$1" \
        "$ROOT/scripts/admin/verify-fee-trustline.sh" testnet "$FEE" "USDC:$ISSUER"
}

expect pass "2-of-3 of account keys" multisig multisig-ok.json
expect fail "pre-auth tx signer in the quorum" multisig multisig-preauth.json
expect fail "hash-x signer in the quorum" multisig multisig-hashx.json
expect pass "signer roster matches" roster multisig-ok.json "$SIGNERS"
expect fail "signer roster differs" roster multisig-ok.json "${SIGNERS%,*},$OTHER"
expect fail "malformed multisig address" \
    "$ROOT/scripts/admin/verify-multisig.sh" 'GBAD$(id)' testnet

expect pass "fee account trusts the token" fee fee-ok.json
expect fail "fee account has no trustline" fee fee-missing.json
expect fail "fee trustline not authorized" fee fee-unauthorized.json
expect pass "native needs no trustline" \
    "$ROOT/scripts/admin/verify-fee-trustline.sh" testnet "$FEE" native

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
printf 'ADMIN_IDENTITY=alice\nFEE_BPS="250"\nINJECT=$(touch %s/pwned)\n' "$TMP" > "$TMP/env"
(
    # shellcheck source=lib/deploy-env.sh
    source "$ROOT/scripts/lib/deploy-env.sh"
    load_env_deploy "$TMP/env"
    [[ "$ADMIN_IDENTITY" == alice && "$FEE_BPS" == 250 && ! -e "$TMP/pwned" ]]
) && echo "PASS .env.deploy is parsed, not executed" || {
    echo "FAIL .env.deploy is parsed, not executed"
    FAILED=1
}

exit "$FAILED"
