# Mainnet deployment runbook

**For:** the contracts engineer + the admin multi-sig signers.
**Last updated:** 2026-07-24 (SDK 23 to SDK 27 storage compatibility result).

This is the cold-deploy procedure for Boundless contracts on Stellar mainnet. Follow it in order. Do not skip the verification steps. Every transaction below is irreversible.

The contract surface changed in the 2026-06 audit fix sweep. Anything wired to the prior surface (`cancel_event`, `upgrade(wasm_hash)`, the 7-field `Profile` struct) will break. Off-chain coordination is covered in `boundless-nestjs` `BACKLOG.md` and `boundless-platform` codegen.

---

## 0. Pre-flight checklist

Before touching mainnet, all of the following must be true.

- [ ] **Third-party security audit:** report received, all critical + high findings resolved, public report link available.
- [ ] **In-house Stellar-skill audit:** every H / M / L finding in `docs/audit-2026-06-stellar-skill.md` is either ✓ closed or has an inline rationale. Verify via `BACKLOG.md` `Done` log.
- [ ] **Testnet smokes all green:** Bounty Curated Showdown, Hackathon flagship, Grant 3-position cohort, Crowdfunding campaign, XLM end-to-end smoke, replay-lock + sweep smokes.
- [ ] **Admin multi-sig provisioned:** signer addresses generated, threshold configured, recovery plan documented (see `admin-custody-policy.md`).
- [ ] **Fee account address generated:** Stellar G-address with USDC trustline pre-funded with the minimum reserve balance.
- [ ] **Treasury composition decision locked:** per `business-model.md` 8.4 (100% USDC at launch).
- [ ] **Per-pillar fee % locked:** per `business-model.md` 8.1 (Bounty 2.5%, Hackathon 1.5%, Grant 1.5%, Crowdfunding 2.5%).
- [ ] **Legal sign-off:** per `boundless-nestjs/docs/runbooks/legal-review.md` for the pillars going live first.
- [ ] **Production env config staged:** Postgres, Redis, secret vault, observability all production-grade.
- [ ] **Status page live** at `status.boundless.com`.
- [ ] **On-call rotation active.**
- [ ] **Founder + lead engineer both confirmed available** for the deploy window.

If any item is unchecked, stop. Mainnet is not negotiable.

---

## 1. Deployment timeline

| Phase | Duration | Owner |
|---|---|---|
| Pre-flight verification (Section 0) | 1 hour | Lead engineer + founder |
| Deploy profile contract | 15 min | Lead engineer |
| Deploy events contract | 30 min | Lead engineer |
| Wire profile → events | 15 min | Lead engineer |
| Register USDC SAC | 5 min | Lead engineer |
| Smoke test on mainnet (tiny amount) | 30 min | Lead engineer |
| Multi-sig rotation (initial admin → multi-sig) | 30 min | Founder + signers |
| Public announcement | 15 min | Founder + marketing |

Total: ~3 hours of focused work, plus pre-flight.

---

## 2. Deploy procedure

### 2.1 Set environment

```bash
# Use our managed production provider. The URL below is Stellar's listed public
# fallback, not the preferred production endpoint.
export STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
export STELLAR_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015"
stellar network add boundless-mainnet \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$STELLAR_NETWORK_PASSPHRASE"

# From here onward use only the named network. Never mix --network with the
# explicit RPC/passphrase pair in one command; see contract-ops-runbook Rule 5.
export STELLAR_NETWORK=boundless-mainnet
unset STELLAR_RPC_URL STELLAR_NETWORK_PASSPHRASE

# Treasury G-address (fee_account). Pre-funded with min reserve + USDC trustline.
export FEE_ACCOUNT=GADMINMS...  # placeholder; replace with real fee account

# Initial admin: a single key used ONLY to deploy and immediately rotate to multi-sig.
# This key is destroyed after Section 2.6.
export INITIAL_ADMIN_KEY=SDADM...

# Multi-sig signer addresses; collected before deploy (see admin-custody-policy.md).
export MULTISIG_SIGNER_1=GSIGN1...
export MULTISIG_SIGNER_2=GSIGN2...
export MULTISIG_SIGNER_3=GSIGN3...
export MULTISIG_THRESHOLD=2  # 2-of-3 to start; review at scale

# Boundless platform constants
export INITIAL_GLOBAL_FEE_BPS=250  # 2.5% Bounty default; overridden per event
# (Contract 1.0.0 also required BOOTSTRAP_PROFILE_CREDITS here; the 1.1.0
#  credit-removal upgrade moved credits to an off-chain ledger in
#  boundless-nestjs, and the profile constructor now takes only --admin.)

# USDC SAC address on Stellar mainnet (verify against Circle's documentation
# at the time of deploy; this is the canonical Stellar Asset Contract for USDC).
export USDC_SAC=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
```

### 2.2 Deploy profile contract

```bash
cd boundless-contract
stellar contract build --locked

# Deploy boundless-profile first; events depends on its address.
PROFILE_WASM=target/wasm32v1-none/release/boundless_profile.wasm

PROFILE_ID=$(stellar contract deploy \
  --network "$STELLAR_NETWORK" \
  --source $INITIAL_ADMIN_KEY \
  --wasm $PROFILE_WASM \
  -- \
  --admin $(stellar keys address $INITIAL_ADMIN_KEY))

echo "PROFILE_ID=$PROFILE_ID" | tee -a deployments/mainnet.env
```

Verify:
```bash
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source $INITIAL_ADMIN_KEY \
  --id $PROFILE_ID \
  -- get_admin
```
Should return the address of `$INITIAL_ADMIN_KEY`.

### 2.3 Deploy events contract

```bash
EVENTS_WASM=target/wasm32v1-none/release/boundless_events.wasm

EVENTS_ID=$(stellar contract deploy \
  --network "$STELLAR_NETWORK" \
  --source $INITIAL_ADMIN_KEY \
  --wasm $EVENTS_WASM \
  -- \
  --admin $(stellar keys address $INITIAL_ADMIN_KEY) \
  --fee_account $FEE_ACCOUNT \
  --fee_bps $INITIAL_GLOBAL_FEE_BPS \
  --profile_contract $PROFILE_ID)

echo "EVENTS_ID=$EVENTS_ID" | tee -a deployments/mainnet.env
```

Verify reads:
```bash
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- get_admin
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- get_fee_bps
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- get_fee_account
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- get_profile_contract
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- is_paused
```

All must return the values you set.

### 2.4 Wire profile → events

```bash
# Profile contract needs to know which events contract is allowed to write to it.
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source $INITIAL_ADMIN_KEY \
  --id $PROFILE_ID \
  -- set_events_contract \
  --new_addr $EVENTS_ID
```

`set_events_contract` is **first-set-only**: after this call succeeds, the contract refuses a second `set_events_contract` and rotation must go through the timelocked two-step (`propose_events_contract` + ~1-day wait + `accept_events_contract`). See Section 7.4 for the rotation procedure if the events contract address ever needs to change.

Verify the version label was written by the constructor:

```bash
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $PROFILE_ID -- version
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- version
# For this tree: profile returns "1.2.0" and events returns "1.5.0".
# For a tagged release, compare each result with that package's INITIAL_VERSION.
```

### 2.5 Register USDC SAC

```bash
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source $INITIAL_ADMIN_KEY \
  --id $EVENTS_ID \
  -- register_supported_token \
  --token $USDC_SAC

# Confirm
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID \
  -- is_supported_token --token $USDC_SAC
# Should return true.
```

If you are launching with XLM support too (per B14):
```bash
export XLM_SAC=<XLM Stellar Asset Contract address>  # native XLM SAC
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID \
  -- register_supported_token --token $XLM_SAC
```

### 2.6 Tiny mainnet smoke (single founder funds it)

Before rotating admin, run one tiny end-to-end flow with the founder's own USDC. This confirms the deploy is wired correctly without touching customer money.

```bash
cd ../boundless-nestjs
STELLAR_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015" \
EVENTS_CONTRACT_ID=$EVENTS_ID \
PROFILE_CONTRACT_ID=$PROFILE_ID \
npx ts-node -r tsconfig-paths/register scripts/smoke/mainnet-tiny-bounty.ts
```

The script publishes a $1 Bounty, has one applicant submit, and pays out. Verify:
- The settlement transaction landed within 90 seconds.
- The fee account received the expected fee (1 USDC × 2.5% = 2,500 stroops = 0.025 USDC).
- The applicant's wallet shows 1 USDC.

If any step fails, **pause the contract** before continuing (see Section 4) and investigate.

### 2.7 Rotate admin from single-key to multi-sig

**Gate:** complete `docs/multisig-preflight.md` (every checkbox) and run `./scripts/admin/verify-multisig.sh <MULTISIG_G_ADDRESS> mainnet` until every check passes BEFORE invoking `set_admin`. If verify-multisig fails, do not proceed.


Now that the deploy works, rotate the admin key. This is the operation that locks production down.

```bash
# Step A: build the multi-sig account.
# Multi-sig accounts on Stellar are regular G-addresses with custom signers + thresholds.
# Set up the multi-sig account separately (out of band; see admin-custody-policy.md).

export MULTISIG_ADMIN_ADDRESS=GMULTISIG...

# Step B: set the pending admin (two-step rotation).
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source $INITIAL_ADMIN_KEY \
  --id $EVENTS_ID \
  -- set_admin \
  --new_admin $MULTISIG_ADMIN_ADDRESS

# Step C: build + simulate the multi-sig accept transaction.
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source-account $MULTISIG_ADMIN_ADDRESS \
  --id $EVENTS_ID \
  --build-only \
  -- accept_admin \
  | stellar tx simulate \
      --network "$STELLAR_NETWORK" \
      --source-account $MULTISIG_ADMIN_ADDRESS \
  > /tmp/events-accept-admin.xdr
```

Sign `/tmp/events-accept-admin.xdr` sequentially with the required quorum, then submit the final signed XDR with `stellar tx send "<signed-xdr>" --network "$STELLAR_NETWORK"`. Follow `docs/contract-ops-runbook.md` Section 5 exactly.

Verify:
```bash
stellar contract invoke --network "$STELLAR_NETWORK" --source $INITIAL_ADMIN_KEY --id $EVENTS_ID -- get_admin
# Should now return $MULTISIG_ADMIN_ADDRESS.
```

Do the same rotation on `$PROFILE_ID`.

### 2.8 Destroy the initial admin key

The initial admin key was a single-key, used only to deploy. Once Section 2.7 succeeds:

1. Securely delete the local copy on the deploy machine (`shred -u` or equivalent).
2. Confirm via `stellar keys ls` that the key is no longer registered.
3. The on-chain account that held the key continues to exist but has no admin authority on the contract anymore. Its funds can be swept to the treasury later.

### 2.9 Record the deployment

```bash
cat > boundless-contract/deployments/mainnet.json <<EOF
{
  "network": "mainnet",
  "passphrase": "Public Global Stellar Network ; September 2015",
  "deployed_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "events_contract": "$EVENTS_ID",
  "profile_contract": "$PROFILE_ID",
  "admin": "$MULTISIG_ADMIN_ADDRESS",
  "fee_account": "$FEE_ACCOUNT",
  "initial_fee_bps": $INITIAL_GLOBAL_FEE_BPS,
  "registered_tokens": ["$USDC_SAC"],
  "events_wasm_hash": "$(sha256sum $EVENTS_WASM | cut -d' ' -f1)",
  "profile_wasm_hash": "$(sha256sum $PROFILE_WASM | cut -d' ' -f1)"
}
EOF

git add boundless-contract/deployments/mainnet.json
git commit -m "feat(contract): mainnet deploy"
git push
```

Commit + tag the wasm hashes. We can verify any future upgrade against this baseline.

---

## 3. Post-deploy verification

After Section 2 completes, the platform is structurally ready. Before public announcement:

- [ ] **End-to-end smoke per pillar on mainnet** with tiny ($1-5) amounts.
- [ ] **Settlement-time telemetry** showing < 60s p95.
- [ ] **Monitoring dashboards** showing live data.
- [ ] **Status page** updated to "operational."
- [ ] **DLQ empty.**
- [ ] **Fee account balance** matches expected fees collected during smokes.
- [ ] **All four pillar publish flows** tested via the production API.
- [ ] **Audit report linked** publicly via the docs.

---

## 4. Pause-the-contract escape hatch

If anything looks wrong between Section 2.6 and the public announcement, pause immediately.

```bash
# Pause: any TWO multi-sig signers (lower-quorum emergency authority).
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source-account $MULTISIG_ADMIN_ADDRESS \
  --id $EVENTS_ID \
  --build-only \
  -- pause \
  | stellar tx simulate \
      --network "$STELLAR_NETWORK" \
      --source-account $MULTISIG_ADMIN_ADDRESS \
  > /tmp/events-emergency-pause.xdr
```

Sign `/tmp/events-emergency-pause.xdr` sequentially with two signers, submit with `stellar tx send`, then read `is_paused` back as `true`.

Pause stops event-lifecycle mutations guarded by `require_not_paused`, including `create_event`, `add_funds`, `propose_manager`, `accept_manager`, `cancel_pending_manager`, `start_cancel`, `process_cancel_batch`, `finalize_cancel`, `select_winners`, `claim_milestone`, `apply_to_bounty`, `withdraw_application`, `submit`, and `withdraw_submission`. Reads continue.

Pause intentionally does **not** block admin recovery and governance entrypoints such as `propose_upgrade`, `apply_upgrade`, `cancel_pending_upgrade`, `migrate`, configuration changes, token-list changes, admin rotation, or `unpause`. This is why the coordinated SDK 27 upgrade can remain paused through its timelock.

Unpausing requires full quorum (see `admin-custody-policy.md` Section 4).

---

## 5. After mainnet: ongoing operational cadence

| Cadence | Action | Owner |
|---|---|---|
| Daily | Check monitoring dashboards | On-call |
| Weekly | Settlement-time report | Founder |
| Monthly | Backup verification drill | On-call |
| Quarterly | Multi-sig rotation drill | Founder + signers |
| Quarterly | Audit refresh (whichever contract version is live) | Lead engineer |
| Semi-annually | Treasury composition review | Founder |
| Annually | Sumsub vendor review | Founder |

---

## 6. Open follow-ups

- Automate the smoke battery in Section 2.6 + 3 as a single `npm run smoke:mainnet` target.
- Move the deploy machine to a hardware-isolated workstation; the deploy session uses the only sensitive key.
- Publish the verified `mainnet.json` to the docs site as a tamper-evident reference.
- Extend `deployments/mainnet-upgrades.jsonl` logging with transaction hashes. `deploy_mainnet.sh` already records timestamp, action, version, and wasm hash.

---

## 7. Upgrades (timelocked; H6)

The 2026-06 audit replaced the immediate-effect `upgrade(wasm_hash)` admin call with a three-step timelock flow. Use `deploy_mainnet.sh` for mainnet; it verifies the queued version and exact WASM hash for both contracts and preserves the zero-event guard through the SDK 27 timelock. Direct contract calls below are only for diagnosis.

### 7.1 Constants

| Window | Value (ledgers @ ~5s) | Wall clock |
|---|---|---|
| `UPGRADE_TIMELOCK_LEDGERS` | 17_280 | ~1 day |
| `PENDING_UPGRADE_TTL_LEDGERS` | 518_400 | ~30 days |

These are in `contracts/events/src/admin.rs` and `contracts/profile/src/admin.rs`. Verify against the live wasm before timing a window.

### SDK 27 compatibility gate

The suite upgrades the exact mainnet `1.1.0` WASMs and ledger `63617727`
snapshot, plus synthetic SDK 23 rows for every persisted layout, through H6
to the pinned events `1.5.0` and profile `1.2.0` WASMs. It then exercises
configuration, governance, event reads, winners, submissions, contributions,
cancellation, prize claims, earnings, indexes, cursors, and legacy replay
markers. Fixture hashes, provenance, and regeneration are recorded in
`docs/sdk27-compatibility-fixtures.md`.

Before any SDK 27 mainnet proposal:

```bash
stellar contract build --locked
./scripts/test-sdk27-compat.sh
```

Abort if a fixture or build hash differs, a compatibility test fails, or the
zero-event check no longer returns `EventNotFound`. The SDK 27 contracts retain
a read fallback for legacy temporary `OpSeen(BytesN<32>)` rows; new markers use
the domain-scoped key.

Sections 7.2 through 7.5 are the executable SDK 27 procedure for events `1.1.0` → `1.5.0` and profile `1.1.0` → `1.2.0`.
The mainnet script intentionally rejects other upgrade target versions. A
future release must update its reviewed version/hash pins and compatibility
fixtures before preparing a proposal.

### 7.2 Freeze and propose SDK 27

This is a coordinated two-contract upgrade. Keep both contracts paused from before either proposal until both migrations and post-upgrade reads have succeeded. Set the live contract IDs explicitly if `deployments/mainnet.json` is unavailable. `ADMIN_SOURCE` is the admin multi-sig G-address, not a secret or single-signer alias. `UPLOAD_SOURCE` is a separate funded, signable CLI identity; uploading WASM does not require contract-admin authority.

Freeze governance changes for the whole window: do not call `set_admin`,
`accept_admin`, `propose_events_contract`, `accept_events_contract`, or
`cancel_pending_events_contract` on either contract while upgrade XDRs are
being prepared, signed, or submitted. Before every signed submission, rerun
the corresponding preparation or verification command against current ledger
state. The script requires both current admins to equal `ADMIN_SOURCE`, the
profile's active events binding to equal `EVENTS_ID`, and
`get_pending_events_contract` to be `null`.

```bash
export EVENTS_ID=CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ
export PROFILE_ID=CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC
export ADMIN_SOURCE=$MULTISIG_ADMIN_ADDRESS
export UPLOAD_SOURCE=boundless-upgrade-uploader

stellar contract build --locked
./scripts/test-sdk27-compat.sh

EVENTS_WASM=target/wasm32v1-none/release/boundless_events.wasm
PROFILE_WASM=target/wasm32v1-none/release/boundless_profile.wasm

# Upload the exact compatibility-tested artifacts before downtime.
./deploy_mainnet.sh upload-profile-wasm "$PROFILE_WASM"
./deploy_mainnet.sh upload-events-wasm "$EVENTS_WASM"

# Freeze events first. This command only prepares XDR.
./deploy_mainnet.sh prepare-pause-events /tmp/events-1.5.0-pause.xdr
```

Sign and submit the events pause XDR. Only after it lands, prepare, sign, and submit the profile pause:

```bash
./deploy_mainnet.sh prepare-pause-profile /tmp/profile-1.2.0-pause.xdr
```

Never prepare the next admin XDR before the prior one lands; each uses the multi-sig source account's current sequence. Then:

```bash
./deploy_mainnet.sh upgrade-status
# Both is_paused values must be true, both versions must be 1.1.0,
# both pending-upgrade values must be null, profile.get_events_contract must
# equal EVENTS_ID, and profile.get_pending_events_contract must be null.
```

Prepare the profile proposal:

```bash
./deploy_mainnet.sh prepare-propose-upgrade-profile \
  "$PROFILE_WASM" "1.2.0" /tmp/profile-1.2.0-propose.xdr
```

Sign and submit it, then verify what landed:

```bash
./deploy_mainnet.sh verify-proposed-upgrade-profile "$PROFILE_WASM" "1.2.0"
```

Only then prepare, sign, and submit the events proposal:

```bash
./deploy_mainnet.sh prepare-propose-upgrade-events \
  "$EVENTS_WASM" "1.5.0" /tmp/events-1.5.0-propose.xdr
```

Verify the events proposal and inspect both contracts:

```bash
./deploy_mainnet.sh verify-proposed-upgrade-events "$EVENTS_WASM" "1.5.0"
./deploy_mainnet.sh upgrade-status
```

The guarded preparation and verification:

- requires both live versions to be `1.1.0`, both contracts to be paused, and no conflicting upgrade to be pending;
- rechecks both active admins, the profile-to-events binding, and the absence of a pending profile events-contract rotation at every prepared or verified upgrade step;
- derives the first possible event ID from `id_base` and requires `get_event` to fail specifically with `EventNotFound` before the events pause, proposal, and apply;
- requires Stellar CLI `27.0.0` and pins the local events and profile files to the exact compatibility-tested SDK 27 hashes;
- uploads without re-optimizing and requires each returned hash to equal the local file's SHA-256;
- builds and simulates every admin transaction without bypassing the 2-of-3 signing flow;
- reads back each queued version and exact WASM hash after the signed proposals land;
- queries each live contract's executable hash after apply and rechecks it before migration, unpause, and final verification.

Do not unpause either contract during the timelock. If the event check succeeds or returns an ambiguous RPC error, abort without applying either upgrade and investigate the live state. Publish both `proposed_at_ledger` and `available_at_ledger` values. Apply only after the later of the two `available_at_ledger` values has been reached.

### 7.3 Apply, migrate, verify, and unpause

Keep the exact proposed artifacts. Apply profile first while both contracts remain paused:

```bash
./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh prepare-apply-upgrade-profile \
  "$PROFILE_WASM" "1.2.0" /tmp/profile-1.2.0-apply.xdr
```

Sign and submit the profile apply XDR. Confirm profile is `1.2.0`, has no pending proposal, and remains paused. Then prepare, sign, and submit events apply:

```bash
./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh prepare-apply-upgrade-events \
  "$EVENTS_WASM" "1.5.0" /tmp/events-1.5.0-apply.xdr
```

Before preparing either apply transaction, the script rechecks the pinned
local artifact, queued hash, current versions, pause state, both admins, the
profile-to-events binding, and the absence of a pending events-contract
rotation. Before events apply it also rechecks the zero-event invariant and
the profile's live executable hash. After each signed apply lands, the next
guarded command queries the contract's live executable hash and requires the
exact pinned SDK 27 value. Status must show events `1.5.0`, profile `1.2.0`,
no pending proposals, the original profile-to-events binding, no pending
rotation, and both contracts still paused.

Migrate profile first, then events. Prepare, sign, submit, and verify each transaction before moving to the next:

```bash
./deploy_mainnet.sh prepare-migrate-upgrade-profile \
  "1.2.0" /tmp/profile-1.2.0-migrate.xdr
```

Sign and submit the profile migration. After it lands:

```bash
./deploy_mainnet.sh upgrade-status

./deploy_mainnet.sh prepare-migrate-upgrade-events \
  "1.5.0" /tmp/events-1.5.0-migrate.xdr
```

Sign and submit the events migration. After it lands:

```bash
./deploy_mainnet.sh upgrade-status
# Migrated-to values must be profile 1.2.0 and events 1.5.0.
```

Unpause profile first so events cannot accept work while its profile dependency is frozen. After the profile unpause lands, unpause events:

```bash
./deploy_mainnet.sh prepare-unpause-profile \
  "1.2.0" /tmp/profile-1.2.0-unpause.xdr
```

Sign and submit the profile unpause. After it lands:

```bash
./deploy_mainnet.sh prepare-unpause-events \
  "1.5.0" /tmp/events-1.5.0-unpause.xdr
```

Sign and submit the events unpause.

Only after both signed unpause transactions land:

```bash
./deploy_mainnet.sh verify-upgrade-profile "$PROFILE_WASM" "1.2.0"
./deploy_mainnet.sh verify-upgrade-events "$EVENTS_WASM" "1.5.0"
./deploy_mainnet.sh upgrade-status
```

Final verification requires the exact local and live executable hashes, expected versions and migration markers, no pending proposals or events-contract rotation, the original profile-to-events binding, both expected admins, and both pause flags to be `false` before updating the deployment records.

Common contract errors:

- `UpgradeTimelockNotElapsed` — wait longer; leave the contract paused.
- `UpgradeProposalExpired` — cancel the pending proposal, then propose again.
- `UpgradeNotProposed` — no proposal is in storage; inspect `upgrade-status`.

### 7.4 Rotating the events-contract address on `boundless-profile`

`set_events_contract` is first-set-only (Section 2.4). If the events contract needs to be redeployed at a new address (e.g., recovery from a hard fork), the profile contract uses a separate timelocked rotation:

```bash
# Propose the new events contract address.
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source-account $MULTISIG_ADMIN_ADDRESS \
  --id $PROFILE_ID \
  --build-only \
  -- propose_events_contract \
  --new_addr $NEW_EVENTS_ID \
  | stellar tx simulate --network "$STELLAR_NETWORK" --source-account $MULTISIG_ADMIN_ADDRESS \
  > /tmp/profile-propose-events-contract.xdr

# Wait ~1 day (EVENTS_CONTRACT_TIMELOCK_LEDGERS = 17_280).
stellar contract invoke \
  --network "$STELLAR_NETWORK" \
  --source-account $MULTISIG_ADMIN_ADDRESS \
  --id $PROFILE_ID \
  --build-only \
  -- accept_events_contract \
  | stellar tx simulate --network "$STELLAR_NETWORK" --source-account $MULTISIG_ADMIN_ADDRESS \
  > /tmp/profile-accept-events-contract.xdr
```

Sign and submit each prepared XDR separately, verifying the proposal landed before preparing the accept transaction.

Expiry is 7 days (`PENDING_EVENTS_CONTRACT_TTL_LEDGERS = 120_960`).

### 7.5 Recovery

Before either apply lands, cancel each affected proposal in profile-then-events
order, signing and submitting each XDR before preparing the next:

```bash
./deploy_mainnet.sh prepare-cancel-upgrade-profile \
  /tmp/profile-1.2.0-cancel.xdr
./deploy_mainnet.sh prepare-cancel-upgrade-events \
  /tmp/events-1.5.0-cancel.xdr
```

Only prepare a cancel for a contract with a pending proposal. Cancellation
leaves both contracts paused. Either restart the proposal sequence or, when
abandoning the upgrade, unpause profile and then events, again submitting each
XDR before preparing the next:

```bash
./deploy_mainnet.sh prepare-unpause-profile \
  "1.1.0" /tmp/profile-abandon-upgrade-unpause.xdr
./deploy_mainnet.sh prepare-unpause-events \
  "1.1.0" /tmp/events-abandon-upgrade-unpause.xdr
```

`apply_upgrade` is atomic per contract: a failed apply leaves that contract's
old WASM and proposal in place. Once either apply lands, do not downgrade or
unpause. Run `upgrade-status`, complete the other apply, and resume Section 7.3
from the first missing migration or verification step. The unpause commands
require the exact live WASM and migration markers.

---

## 8. 2026-06 Stellar-skill audit — delta from the previous runbook

This section summarizes everything that changed in the contract surface between this revision and the prior runbook. Use it when reviewing customer / partner integrations.

### 8.1 Removed

- `upgrade(new_wasm_hash)` — replaced by `propose_upgrade` + `apply_upgrade`. See Section 7.
- `cancel_event(event_id, op_id)` — replaced by `start_cancel` + `process_cancel_batch` + `finalize_cancel`. See Section 8.4.
- `Profile.wins_count`, `submissions_count`, `applications_count`, `milestones_completed` — off-chain indexer derives counters from the emitted events instead.
- `__link_keep` — linker keep-alive trick, no longer needed.

### 8.2 Added

- `version() -> String` — on-chain semver. Returns that package's `INITIAL_VERSION`; bumped by `apply_upgrade`.
- `propose_upgrade(wasm_hash, new_version)`, `apply_upgrade()`, `cancel_pending_upgrade()`, `migrate()` — timelocked upgrade flow.
- `start_cancel(event_id, op_id)`, `process_cancel_batch(event_id, max_refunds, op_id) -> u32`, `finalize_cancel(event_id, op_id)` — paged cancellation.
- `propose_events_contract(addr)`, `accept_events_contract()`, `cancel_pending_events_contract()` — two-step rotation for the profile contract's events binding. First-set still uses `set_events_contract`.
- Paged read accessors on the events contract: `get_applicant_count`, `get_applicant_at`, `get_winner_count`, `get_winner_at`, `get_contributor_count`, `get_contributor_at`. The aggregated reads (`get_applicants`, `get_winners`, `get_contributors`) cap at the per-event maximum and stay available for backwards compat.
- New `EventStatus::Cancelling` variant. Any switch on EventStatus needs to handle it.
- New events: `PendingUpgradeProposed`, `UpgradeApplied`, `Migrated`, `PendingUpgradeCancelled`, `PendingEventsContractSet`, `EventsRotationCancelled`. `EventCreated` adds a `title` field.

### 8.3 Behavioral changes

- **`select_winners` math (M1)** now pays against `remaining_escrow` at the moment of the call, snapshotted before any release. Partner top-ups via `add_funds` flow to winners instead of staying trapped until cancel.
- **Crowdfunding `claim_milestone` requires admin co-sign (M5).** The builder's auth is necessary but no longer sufficient. Off-chain admin tooling must co-sign the tx.
- **`MAX_FEE_BPS` is 1_000 (10%) (L4)**, down from 5_000 (50%). Per-event overrides still respect the cap.
- **Storage type changes (H1)**: admin/config now lives in `instance()` storage. Persistent reads of event-scoped data bump TTL on every read (H2). No off-chain wiring change; deploy fresh contracts and the new layout takes effect.
- **Per-event lists are paged (H3, H4)**: `Vec<Address>` lists and the `Map<Address, Submission>` collapse into per-element keys with an index counter. Soft caps at `MAX_APPLICANTS_PER_EVENT = 5_000` and `MAX_CONTRIBUTORS_PER_EVENT = 5_000`. Submissions key per-`(event, applicant)`. The aggregated reads cap at these limits.

### 8.4 Paged cancel — sequence diagram

```
start_cancel(event_id, op_id)
  ├─ 0 contributors → status flips to Cancelled inline, owner refunded.
  └─ N contributors → status flips to Cancelling,
     CancellationState stored from one NonOwnerContributionTotal read.

process_cancel_batch(event_id, max_refunds, op_id) -> remaining
  ├─ permissionless once status is Cancelling.
  ├─ refunds up to min(max_refunds, MAX_REFUNDS_PER_BATCH = 25) contributors.
  ├─ advances cursor inside CancellationState.
  └─ returns the count of contributors still queued. Loop until 0.

finalize_cancel(event_id, op_id)
  ├─ permissionless once status is Cancelling.
  ├─ requires cursor == count_at_start (errors with CancellationNotFinished otherwise).
  ├─ pays owner residual on the FullPartnerThenResidual branch.
  └─ status flips to Cancelled, CancellationState cleared.
```

Cap math: at `MAX_REFUNDS_PER_BATCH = 25`, refunding 5_000 contributors takes 200 batch transactions (~17 minutes wall clock at testnet ledger cadence). Most cancels are well under that limit.

The paged-cancel automation worker lives in `boundless-nestjs` (see its own BACKLOG for the worker's status). The orchestrator's `beginStartCancel` kicks off step 1. Any account can sponsor steps 2 and 3, so an unavailable event manager cannot strand a cancellation already in progress.

For the events `1.1.0` → `1.5.0` and profile `1.1.0` → `1.2.0` mainnet upgrade, `deploy_mainnet.sh` freezes the zero-event assumption: it pauses before proposal, verifies `is_paused == true` and `EventNotFound`, keeps the contract paused through the timelock, and repeats both checks immediately before apply. Any existing event or ambiguous RPC result aborts the upgrade. New events initialize `NonOwnerContributionTotal` at zero and `add_funds` maintains it in O(1). An older event with no contributors initializes the missing total lazily. If an older event already has contributors, `add_funds` and `start_cancel` fail with `CancellationTotalMissing` instead of using an incomplete total.
