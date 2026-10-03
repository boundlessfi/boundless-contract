#!/usr/bin/env bash
# Exercises deploy_mainnet.sh against scripts/testdata/mock-stellar, so a guard
# that stops refusing an unsafe upgrade step fails CI. No network is used.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/boundless-mainnet-guards.XXXXXX")"
FAKE_BIN="$TEST_ROOT/bin"
SCRIPT="$TEST_ROOT/deploy_mainnet.sh"
XDR_DIR="$TEST_ROOT/xdr"
EVENTS_WASM="$TEST_ROOT/events-2.0.0.wasm"
PROFILE_WASM="$TEST_ROOT/profile-1.2.1.wasm"
MISLABELLED_WASM="$TEST_ROOT/events-mislabelled.wasm"
OLD_EVENTS_HASH="b87365c16102f7242a8fc3e770f615991841741c40673bb241cb254130be4c26"
OLD_PROFILE_HASH="6b4804920a4068dfdcaaa711fac6390850e0a92f4b5ed4e46259694b61cbae12"
WRONG_HASH="0000000000000000000000000000000000000000000000000000000000000000"

cleanup() {
    rm -rf "$TEST_ROOT"
}
trap cleanup EXIT

mkdir -p "$FAKE_BIN" "$XDR_DIR"
cp "$ROOT/scripts/testdata/mock-stellar" "$FAKE_BIN/stellar"
cp "$ROOT/deploy_mainnet.sh" "$SCRIPT"
chmod +x "$FAKE_BIN/stellar" "$SCRIPT"
printf 'kind=events\nversion=2.0.0\n' > "$EVENTS_WASM"
printf 'kind=profile\nversion=1.2.1\n' > "$PROFILE_WASM"
printf 'kind=events\nversion=2.0.1\n' > "$MISLABELLED_WASM"

export PATH="$FAKE_BIN:$PATH"
export STELLAR_NETWORK=mainnet-review
export ADMIN_SOURCE=GADMIN
export EVENTS_ID=EVENTS
export PROFILE_ID=PROFILE
unset STELLAR_RPC_URL STELLAR_NETWORK_PASSPHRASE UNPAUSE_UNMIGRATED

hash_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

events_hash="$(hash_file "$EVENTS_WASM")"
profile_hash="$(hash_file "$PROFILE_WASM")"
events_pending="{\"new_version\":\"2.0.0\",\"wasm_hash\":\"$events_hash\",\"available_at_ledger\":100,\"expires_at_ledger\":518500}"
profile_pending="{\"new_version\":\"1.2.1\",\"wasm_hash\":\"$profile_hash\",\"available_at_ledger\":100,\"expires_at_ledger\":518500}"
bad_events_pending="{\"new_version\":\"2.0.0\",\"wasm_hash\":\"$WRONG_HASH\"}"

# Mainnet as of the 2.0.0 upgrade: events 1.7.0 stamped at 1.6.0 with rows
# still in the old layout, profile 1.2.0 never stamped.
set_events() {
    export FAKE_EVENTS_VERSION="$1"
    export FAKE_EVENTS_PAUSED="$2"
    export FAKE_EVENTS_PENDING="$3"
    export FAKE_EVENTS_MIGRATED="$4"
    export FAKE_EVENTS_MIGRATION_REMAINING="${5:-0}"
    if [ "$1" = "2.0.0" ]; then
        export FAKE_EVENTS_LIVE_HASH="$events_hash"
    else
        export FAKE_EVENTS_LIVE_HASH="$OLD_EVENTS_HASH"
    fi
}

set_profile() {
    export FAKE_PROFILE_VERSION="$1"
    export FAKE_PROFILE_PAUSED="$2"
    export FAKE_PROFILE_PENDING="$3"
    export FAKE_PROFILE_MIGRATED="$4"
    if [ "$1" = "1.2.1" ]; then
        export FAKE_PROFILE_LIVE_HASH="$profile_hash"
    else
        export FAKE_PROFILE_LIVE_HASH="$OLD_PROFILE_HASH"
    fi
}

reset() {
    unset FAKE_CLI_VERSION FAKE_ADMIN UNPAUSE_UNMIGRATED
    export FAKE_PROFILE_EVENTS_CONTRACT=EVENTS
    export FAKE_PROFILE_EVENTS_PENDING=null
    set_events 1.7.0 false null '"1.6.0"' 8
    set_profile 1.2.0 false null null
}

expect_ok() {
    local label="$1" log
    shift
    log="$TEST_ROOT/last.log"
    if ! "$@" >"$log" 2>&1; then
        printf 'FAIL %s\n' "$label" >&2
        cat "$log" >&2
        exit 1
    fi
    printf 'PASS %s\n' "$label"
}

expect_fail() {
    local label="$1" log
    shift
    log="$TEST_ROOT/last.log"
    if "$@" >/dev/null 2>"$log"; then
        printf 'FAIL %s unexpectedly succeeded\n' "$label" >&2
        exit 1
    fi
    printf 'PASS %s failed closed: %s\n' "$label" "$(tail -1 "$log" | sed 's/\x1b\[[0-9;]*m//g')"
}

x() {
    printf '%s/%s.xdr' "$XDR_DIR" "$1"
}

reset
expect_ok upgrade-status "$SCRIPT" upgrade-status
expect_ok verify "$SCRIPT" verify
export FAKE_CLI_VERSION=27.0.0
expect_fail old-cli "$SCRIPT" upgrade-status
reset
export STELLAR_RPC_URL=https://rpc.example.invalid
expect_fail explicit-rpc-with-named-network "$SCRIPT" upgrade-status
unset STELLAR_RPC_URL
expect_fail direct-submit-action "$SCRIPT" propose-upgrade-events "$EVENTS_WASM" 2.0.0
expect_fail unknown-action "$SCRIPT" launch-everything

reset
export FAKE_PROFILE_EVENTS_PENDING='{"target":"ROTATED","proposed_at":100}'
expect_fail pause-with-pending-profile-rotation "$SCRIPT" prepare-pause-events "$(x a)"
reset
export FAKE_PROFILE_EVENTS_CONTRACT=OTHER_EVENTS
expect_fail pause-with-profile-binding-drift "$SCRIPT" prepare-pause-events "$(x b)"
reset
export FAKE_ADMIN=GOTHER
expect_fail pause-with-admin-mismatch "$SCRIPT" prepare-pause-events "$(x c)"

reset
expect_ok pause-events "$SCRIPT" prepare-pause-events "$(x pause-events)"
[ -s "$(x pause-events)" ] || { echo "FAIL pause-events wrote no XDR" >&2; exit 1; }
expect_fail pause-events-missing-output-dir "$SCRIPT" prepare-pause-events "$TEST_ROOT/missing/out.xdr"
set_events 1.7.0 true null '"1.6.0"' 8
expect_fail pause-events-twice "$SCRIPT" prepare-pause-events "$(x d)"

expect_ok propose-events \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x propose-events)"
expect_fail propose-events-with-profile-artifact \
    "$SCRIPT" prepare-propose-upgrade-events "$PROFILE_WASM" 1.2.1 "$(x e)"
expect_fail propose-events-version-not-in-contractmeta \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 2.0.1 "$(x f)"
expect_fail propose-events-mislabelled-artifact \
    "$SCRIPT" prepare-propose-upgrade-events "$MISLABELLED_WASM" 2.0.0 "$(x g)"
expect_fail propose-events-malformed-version \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 2.0 "$(x h)"
expect_fail propose-events-missing-wasm \
    "$SCRIPT" prepare-propose-upgrade-events "$TEST_ROOT/none.wasm" 2.0.0 "$(x i)"
set_events 1.7.0 true "$events_pending" '"1.6.0"' 8
expect_fail propose-events-while-pending \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x j)"
set_events 2.0.0 false null '"2.0.0"' 0
expect_fail propose-events-same-version \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x k)"

set_events 1.7.0 true "$events_pending" '"1.6.0"' 8
expect_ok verify-events-proposal \
    "$SCRIPT" verify-proposed-upgrade-events "$EVENTS_WASM" 2.0.0
expect_fail verify-events-proposal-wrong-version \
    "$SCRIPT" verify-proposed-upgrade-events "$MISLABELLED_WASM" 2.0.1
set_events 1.7.0 true "$bad_events_pending" '"1.6.0"' 8
expect_fail verify-events-proposal-wrong-wasm \
    "$SCRIPT" verify-proposed-upgrade-events "$EVENTS_WASM" 2.0.0

set_events 1.7.0 true "$events_pending" '"1.6.0"' 8
expect_ok apply-events \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x apply-events)"
set_events 1.7.0 false "$events_pending" '"1.6.0"' 8
expect_fail apply-events-unpaused \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x l)"
set_events 1.7.0 true "$bad_events_pending" '"1.6.0"' 8
expect_fail apply-events-wrong-pending-wasm \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x m)"
set_events 1.7.0 true null '"1.6.0"' 8
expect_fail apply-events-nothing-pending \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 2.0.0 "$(x n)"

set_events 2.0.0 true null '"1.6.0"' 2
expect_ok migrate-events-page \
    "$SCRIPT" prepare-migrate-events-page 8 "$(x migrate-events-page)"
expect_fail migrate-events-page-zero "$SCRIPT" prepare-migrate-events-page 0 "$(x o)"
expect_fail migrate-events-page-not-a-number "$SCRIPT" prepare-migrate-events-page eight "$(x p)"
expect_fail migrate-before-pages-finish \
    "$SCRIPT" prepare-migrate-upgrade-events 2.0.0 "$(x q)"
set_events 2.0.0 false null '"1.6.0"' 2
expect_fail migrate-events-page-unpaused "$SCRIPT" prepare-migrate-events-page 8 "$(x r)"
set_events 2.0.0 true "$events_pending" '"1.6.0"' 2
expect_fail migrate-events-page-with-pending-upgrade \
    "$SCRIPT" prepare-migrate-events-page 8 "$(x s)"
set_events 2.0.0 true null '"1.6.0"' 0
expect_fail migrate-events-page-when-done "$SCRIPT" prepare-migrate-events-page 8 "$(x t)"

expect_ok migrate-events \
    "$SCRIPT" prepare-migrate-upgrade-events 2.0.0 "$(x migrate-events)"
expect_fail migrate-events-wrong-version \
    "$SCRIPT" prepare-migrate-upgrade-events 1.9.9 "$(x u)"
set_events 2.0.0 false null '"1.6.0"' 0
expect_fail migrate-events-unpaused \
    "$SCRIPT" prepare-migrate-upgrade-events 2.0.0 "$(x v)"
set_events 2.0.0 true null '"2.0.0"' 0
expect_fail migrate-events-replay \
    "$SCRIPT" prepare-migrate-upgrade-events 2.0.0 "$(x w)"
expect_fail migrate-events-page-after-stamp "$SCRIPT" prepare-migrate-events-page 8 "$(x y)"

expect_ok unpause-events "$SCRIPT" prepare-unpause-events 2.0.0 "$(x unpause-events)"
expect_fail unpause-events-wrong-version "$SCRIPT" prepare-unpause-events 1.7.0 "$(x z)"
set_events 2.0.0 true null '"1.6.0"' 0
expect_fail unpause-events-unmigrated "$SCRIPT" prepare-unpause-events 2.0.0 "$(x aa)"
set_events 1.7.0 true null '"1.6.0"' 8
export UNPAUSE_UNMIGRATED=1
expect_ok abandon-unpause-events "$SCRIPT" prepare-unpause-events 1.7.0 "$(x abandon-events)"
unset UNPAUSE_UNMIGRATED
set_events 1.7.0 true "$events_pending" '"1.6.0"' 8
export UNPAUSE_UNMIGRATED=1
expect_fail unpause-events-with-pending-upgrade "$SCRIPT" prepare-unpause-events 1.7.0 "$(x ab)"
unset UNPAUSE_UNMIGRATED
set_events 2.0.0 false null '"2.0.0"' 0
expect_fail unpause-events-not-paused "$SCRIPT" prepare-unpause-events 2.0.0 "$(x ac)"

expect_ok verify-events "$SCRIPT" verify-upgrade-events "$EVENTS_WASM" 2.0.0
export FAKE_EVENTS_LIVE_HASH="$WRONG_HASH"
expect_fail verify-events-wrong-live-wasm "$SCRIPT" verify-upgrade-events "$EVENTS_WASM" 2.0.0
set_events 2.0.0 true null '"2.0.0"' 0
expect_fail verify-events-still-paused "$SCRIPT" verify-upgrade-events "$EVENTS_WASM" 2.0.0
set_events 2.0.0 false null '"1.6.0"' 0
expect_fail verify-events-unmigrated "$SCRIPT" verify-upgrade-events "$EVENTS_WASM" 2.0.0

reset
set_events 2.0.0 false null '"2.0.0"' 0
expect_ok propose-profile \
    "$SCRIPT" prepare-propose-upgrade-profile "$PROFILE_WASM" 1.2.1 "$(x propose-profile)"
expect_fail propose-profile-with-events-artifact \
    "$SCRIPT" prepare-propose-upgrade-profile "$EVENTS_WASM" 2.0.0 "$(x ad)"
set_profile 1.2.0 false "$profile_pending" null
expect_ok verify-profile-proposal \
    "$SCRIPT" verify-proposed-upgrade-profile "$PROFILE_WASM" 1.2.1
expect_ok apply-profile \
    "$SCRIPT" prepare-apply-upgrade-profile "$PROFILE_WASM" 1.2.1 "$(x apply-profile)"
expect_ok cancel-profile "$SCRIPT" prepare-cancel-upgrade-profile "$(x cancel-profile)"
set_profile 1.2.0 false null null
expect_fail cancel-profile-nothing-pending "$SCRIPT" prepare-cancel-upgrade-profile "$(x ae)"
set_profile 1.2.1 false null null
expect_ok migrate-profile "$SCRIPT" prepare-migrate-upgrade-profile 1.2.1 "$(x migrate-profile)"
expect_fail migrate-profile-wrong-version \
    "$SCRIPT" prepare-migrate-upgrade-profile 1.2.0 "$(x af)"
set_profile 1.2.1 false null '"1.2.1"'
expect_fail migrate-profile-replay "$SCRIPT" prepare-migrate-upgrade-profile 1.2.1 "$(x ag)"
expect_ok verify-profile "$SCRIPT" verify-upgrade-profile "$PROFILE_WASM" 1.2.1
export FAKE_PROFILE_LIVE_HASH="$WRONG_HASH"
expect_fail verify-profile-wrong-live-wasm "$SCRIPT" verify-upgrade-profile "$PROFILE_WASM" 1.2.1

set_events 2.0.0 true "$events_pending" '"1.6.0"' 0
expect_ok cancel-events "$SCRIPT" prepare-cancel-upgrade-events "$(x cancel-events)"
