#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/boundless-mainnet-guards.XXXXXX")"
FAKE_BIN="$TEST_ROOT/bin"
SCRIPT="$TEST_ROOT/deploy_mainnet.sh"
EVENTS_WASM="$ROOT/contracts/compatibility/fixtures/events-1.5.0-sdk27.wasm"
PROFILE_WASM="$ROOT/contracts/compatibility/fixtures/profile-1.2.0-sdk27.wasm"
XDR_DIR="$TEST_ROOT/xdr"
OLD_EVENTS_HASH="2a8789bf5dd392b91ddfe6591072a0cb9fae70045abf379fd1dbbc2db4af1de3"
OLD_PROFILE_HASH="b9e3500cfb559781e68359655f37775c78afffc5a8436daca9d1d1f8e3569b02"
WRONG_HASH="0000000000000000000000000000000000000000000000000000000000000000"

cleanup() {
    rm -rf "$TEST_ROOT"
}
trap cleanup EXIT

mkdir -p "$FAKE_BIN" "$XDR_DIR"
cp "$ROOT/scripts/testdata/mock-stellar" "$FAKE_BIN/stellar"
cp "$ROOT/deploy_mainnet.sh" "$SCRIPT"
chmod +x "$FAKE_BIN/stellar" "$SCRIPT"

export PATH="$FAKE_BIN:$PATH"
export STELLAR_NETWORK=mainnet-review
export ADMIN_SOURCE=GADMIN
export EVENTS_ID=EVENTS
export PROFILE_ID=PROFILE

hash_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

events_wasm_hash="$(hash_file "$EVENTS_WASM")"
profile_wasm_hash="$(hash_file "$PROFILE_WASM")"
events_pending="{\"new_version\":\"1.5.0\",\"wasm_hash\":\"$events_wasm_hash\"}"
profile_pending="{\"new_version\":\"1.2.0\",\"wasm_hash\":\"$profile_wasm_hash\"}"
bad_events_pending="{\"new_version\":\"1.5.0\",\"wasm_hash\":\"$WRONG_HASH\"}"
bad_profile_pending="{\"new_version\":\"1.2.0\",\"wasm_hash\":\"$WRONG_HASH\"}"

set_state() {
    export FAKE_EVENTS_VERSION="$1"
    export FAKE_EVENTS_PAUSED="$2"
    export FAKE_EVENTS_PENDING="$3"
    export FAKE_EVENTS_MIGRATED="$4"
    export FAKE_PROFILE_VERSION="$5"
    export FAKE_PROFILE_PAUSED="$6"
    export FAKE_PROFILE_PENDING="$7"
    export FAKE_PROFILE_MIGRATED="$8"
    export FAKE_PROFILE_EVENTS_CONTRACT=EVENTS
    export FAKE_PROFILE_EVENTS_PENDING=null
    if [ "$FAKE_EVENTS_VERSION" = "1.5.0" ]; then
        export FAKE_EVENTS_LIVE_HASH="$events_wasm_hash"
    else
        export FAKE_EVENTS_LIVE_HASH="$OLD_EVENTS_HASH"
    fi
    if [ "$FAKE_PROFILE_VERSION" = "1.2.0" ]; then
        export FAKE_PROFILE_LIVE_HASH="$profile_wasm_hash"
    else
        export FAKE_PROFILE_LIVE_HASH="$OLD_PROFILE_HASH"
    fi
}

expect_ok() {
    local label="$1"
    shift
    "$@" >/dev/null
    printf 'PASS %s\n' "$label"
}

expect_fail() {
    local label="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        printf 'FAIL %s unexpectedly succeeded\n' "$label" >&2
        exit 1
    fi
    printf 'PASS %s failed closed\n' "$label"
}

set_state 1.1.0 false null null 1.1.0 false null null
export FAKE_PROFILE_EVENTS_PENDING='{"target":"ROTATED","proposed_at":100}'
expect_fail pause-events-with-pending-profile-rotation \
    "$SCRIPT" prepare-pause-events "$XDR_DIR/pending-rotation-pause-events.xdr"

set_state 1.1.0 false null null 1.1.0 false null null
export FAKE_PROFILE_EVENTS_CONTRACT=OTHER_EVENTS
expect_fail pause-events-with-profile-binding-drift \
    "$SCRIPT" prepare-pause-events "$XDR_DIR/binding-drift-pause-events.xdr"

set_state 1.1.0 false null null 1.1.0 false null null
expect_ok pause-events "$SCRIPT" prepare-pause-events "$XDR_DIR/pause-events.xdr"
expect_fail pause-profile-before-events \
    "$SCRIPT" prepare-pause-profile "$XDR_DIR/early-pause-profile.xdr"

set_state 1.1.0 true null null 1.1.0 false null null
expect_ok pause-profile "$SCRIPT" prepare-pause-profile "$XDR_DIR/pause-profile.xdr"

set_state 1.1.0 true null null 1.1.0 true null null
expect_ok propose-profile \
    "$SCRIPT" prepare-propose-upgrade-profile "$PROFILE_WASM" 1.2.0 "$XDR_DIR/propose-profile.xdr"
expect_fail propose-profile-with-events-artifact \
    "$SCRIPT" prepare-propose-upgrade-profile "$EVENTS_WASM" 1.2.0 "$XDR_DIR/wrong-profile-artifact.xdr"
expect_fail propose-profile-with-unpinned-version \
    "$SCRIPT" prepare-propose-upgrade-profile "$PROFILE_WASM" 9.9.9 "$XDR_DIR/wrong-profile-version.xdr"
expect_fail propose-events-before-profile-staged \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 1.5.0 "$XDR_DIR/early-propose-events.xdr"

set_state 1.1.0 true null null 1.1.0 true "$profile_pending" null
expect_ok verify-profile-proposal \
    "$SCRIPT" verify-proposed-upgrade-profile "$PROFILE_WASM" 1.2.0
expect_ok propose-events \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 1.5.0 "$XDR_DIR/propose-events.xdr"
expect_fail propose-events-with-profile-artifact \
    "$SCRIPT" prepare-propose-upgrade-events "$PROFILE_WASM" 1.5.0 "$XDR_DIR/wrong-events-artifact.xdr"
expect_fail propose-events-with-unpinned-version \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 9.9.9 "$XDR_DIR/wrong-events-version.xdr"
set_state 1.1.0 true null null 1.1.0 true "$bad_profile_pending" null
expect_fail propose-events-with-wrong-pending-profile-wasm \
    "$SCRIPT" prepare-propose-upgrade-events "$EVENTS_WASM" 1.5.0 "$XDR_DIR/wrong-profile-pending-propose-events.xdr"

set_state 1.1.0 true "$events_pending" null 1.1.0 true "$profile_pending" null
expect_ok verify-events-proposal \
    "$SCRIPT" verify-proposed-upgrade-events "$EVENTS_WASM" 1.5.0
expect_ok apply-profile \
    "$SCRIPT" prepare-apply-upgrade-profile "$PROFILE_WASM" 1.2.0 "$XDR_DIR/apply-profile.xdr"
expect_fail apply-events-before-profile-applied \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 1.5.0 "$XDR_DIR/early-apply-events.xdr"
set_state 1.1.0 true "$bad_events_pending" null 1.1.0 true "$profile_pending" null
expect_fail apply-profile-with-wrong-pending-events-wasm \
    "$SCRIPT" prepare-apply-upgrade-profile "$PROFILE_WASM" 1.2.0 "$XDR_DIR/wrong-events-pending-apply-profile.xdr"

set_state 1.1.0 true "$events_pending" null 1.2.0 true null null
expect_ok apply-events \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 1.5.0 "$XDR_DIR/apply-events.xdr"
export FAKE_PROFILE_LIVE_HASH="$WRONG_HASH"
expect_fail apply-events-with-wrong-live-profile-wasm \
    "$SCRIPT" prepare-apply-upgrade-events "$EVENTS_WASM" 1.5.0 "$XDR_DIR/wrong-profile-live-apply-events.xdr"

set_state 1.5.0 true null null 1.2.0 true null null
expect_ok migrate-profile \
    "$SCRIPT" prepare-migrate-upgrade-profile 1.2.0 "$XDR_DIR/migrate-profile.xdr"
export FAKE_EVENTS_LIVE_HASH="$WRONG_HASH"
expect_fail migrate-profile-with-wrong-live-events-wasm \
    "$SCRIPT" prepare-migrate-upgrade-profile 1.2.0 "$XDR_DIR/wrong-events-live-migrate-profile.xdr"
set_state 1.5.0 true null null 1.2.0 true null null
export FAKE_PROFILE_LIVE_HASH="$WRONG_HASH"
expect_fail migrate-profile-with-wrong-live-profile-wasm \
    "$SCRIPT" prepare-migrate-upgrade-profile 1.2.0 "$XDR_DIR/wrong-profile-live-migrate-profile.xdr"
set_state 1.5.0 true null null 1.2.0 true null null
expect_fail migrate-events-before-profile-migrated \
    "$SCRIPT" prepare-migrate-upgrade-events 1.5.0 "$XDR_DIR/early-migrate-events.xdr"

set_state 1.5.0 true null null 1.2.0 true null '"1.2.0"'
expect_ok migrate-events \
    "$SCRIPT" prepare-migrate-upgrade-events 1.5.0 "$XDR_DIR/migrate-events.xdr"

set_state 1.5.0 true null '"1.5.0"' 1.2.0 true null '"1.2.0"'
expect_ok unpause-profile \
    "$SCRIPT" prepare-unpause-profile 1.2.0 "$XDR_DIR/unpause-profile.xdr"
expect_fail unpause-profile-with-unpinned-version \
    "$SCRIPT" prepare-unpause-profile 9.9.9 "$XDR_DIR/wrong-profile-unpause-version.xdr"
export FAKE_PROFILE_LIVE_HASH="$WRONG_HASH"
expect_fail unpause-profile-with-wrong-live-profile-wasm \
    "$SCRIPT" prepare-unpause-profile 1.2.0 "$XDR_DIR/wrong-profile-live-unpause-profile.xdr"
set_state 1.5.0 true null '"1.5.0"' 1.2.0 true null '"1.2.0"'
export FAKE_EVENTS_LIVE_HASH="$WRONG_HASH"
expect_fail unpause-profile-with-wrong-live-events-wasm \
    "$SCRIPT" prepare-unpause-profile 1.2.0 "$XDR_DIR/wrong-events-live-unpause-profile.xdr"
set_state 1.5.0 true null '"1.5.0"' 1.2.0 true null '"1.2.0"'
expect_fail unpause-events-before-profile-unpaused \
    "$SCRIPT" prepare-unpause-events 1.5.0 "$XDR_DIR/early-unpause-events.xdr"

set_state 1.5.0 true null '"1.5.0"' 1.2.0 false null '"1.2.0"'
expect_ok unpause-events \
    "$SCRIPT" prepare-unpause-events 1.5.0 "$XDR_DIR/unpause-events.xdr"
expect_fail unpause-events-with-unpinned-version \
    "$SCRIPT" prepare-unpause-events 9.9.9 "$XDR_DIR/wrong-events-unpause-version.xdr"
export FAKE_EVENTS_LIVE_HASH="$WRONG_HASH"
expect_fail unpause-events-with-wrong-live-events-wasm \
    "$SCRIPT" prepare-unpause-events 1.5.0 "$XDR_DIR/wrong-events-live-unpause-events.xdr"
set_state 1.5.0 true null '"1.5.0"' 1.2.0 false null '"1.2.0"'
export FAKE_PROFILE_LIVE_HASH="$WRONG_HASH"
expect_fail unpause-events-with-wrong-live-profile-wasm \
    "$SCRIPT" prepare-unpause-events 1.5.0 "$XDR_DIR/wrong-profile-live-unpause-events.xdr"

set_state 1.5.0 false null '"1.5.0"' 1.2.0 false null '"1.2.0"'
expect_ok upgrade-status "$SCRIPT" upgrade-status
expect_ok verify-profile "$SCRIPT" verify-upgrade-profile "$PROFILE_WASM" 1.2.0
expect_ok verify-events "$SCRIPT" verify-upgrade-events "$EVENTS_WASM" 1.5.0
export FAKE_EVENTS_LIVE_HASH="$WRONG_HASH"
expect_fail verify-events-with-wrong-live-wasm \
    "$SCRIPT" verify-upgrade-events "$EVENTS_WASM" 1.5.0
set_state 1.5.0 false null '"1.5.0"' 1.2.0 false null '"1.2.0"'
export FAKE_PROFILE_LIVE_HASH="$WRONG_HASH"
expect_fail verify-profile-with-wrong-live-wasm \
    "$SCRIPT" verify-upgrade-profile "$PROFILE_WASM" 1.2.0

set_state 1.1.0 true "$events_pending" null 1.1.0 true "$profile_pending" null
expect_ok cancel-profile \
    "$SCRIPT" prepare-cancel-upgrade-profile "$XDR_DIR/cancel-profile.xdr"
expect_fail abandon-profile-before-events-cancel \
    "$SCRIPT" prepare-unpause-profile 1.1.0 "$XDR_DIR/early-abandon-profile.xdr"

set_state 1.1.0 true "$events_pending" null 1.1.0 true null null
expect_ok cancel-events \
    "$SCRIPT" prepare-cancel-upgrade-events "$XDR_DIR/cancel-events.xdr"

set_state 1.1.0 true null null 1.1.0 true null null
expect_ok abandon-unpause-profile \
    "$SCRIPT" prepare-unpause-profile 1.1.0 "$XDR_DIR/abandon-profile.xdr"
expect_fail abandon-events-before-profile-unpaused \
    "$SCRIPT" prepare-unpause-events 1.1.0 "$XDR_DIR/early-abandon-events.xdr"

set_state 1.1.0 true null null 1.1.0 false null null
expect_ok abandon-unpause-events \
    "$SCRIPT" prepare-unpause-events 1.1.0 "$XDR_DIR/abandon-events.xdr"
