# Mainnet deployment runbook

**For:** the contracts engineer + the admin multi-sig signers.
**Last updated:** 2026-07-17 (revised for the 1.1.0 → 1.2.0 cancellation-liveness upgrade guard).

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
stellar contract build

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
# For this tree: profile returns "1.1.0" and events returns "1.2.0".
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

Pause intentionally does **not** block admin recovery and governance entrypoints such as `propose_upgrade`, `apply_upgrade`, `cancel_pending_upgrade`, `migrate`, configuration changes, token-list changes, admin rotation, or `unpause`. This is why the 1.1.0 → 1.2.0 upgrade can remain paused through its timelock.

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

The 2026-06 audit replaced the immediate-effect `upgrade(wasm_hash)` admin call with a three-step timelock flow. Use `deploy_mainnet.sh` for mainnet; it verifies the queued version and exact WASM hash and provides the special 1.1.0 → 1.2.0 zero-event guard. Direct contract calls below are only for diagnosis.

### 7.1 Constants

| Window | Value (ledgers @ ~5s) | Wall clock |
|---|---|---|
| `UPGRADE_TIMELOCK_LEDGERS` | 17_280 | ~1 day |
| `PENDING_UPGRADE_TTL_LEDGERS` | 518_400 | ~30 days |

These are in `contracts/events/src/admin.rs` and `contracts/profile/src/admin.rs`. Verify against the live wasm before timing a window.

### 7.2 Propose

For the 1.1.0 → 1.2.0 upgrade, the zero-event assumption must remain true for the entire timelock. Set the live contract ID explicitly if `deployments/mainnet.json` is not present. `ADMIN_SOURCE` is the admin multi-sig G-address, not a secret or single-signer alias. `UPLOAD_SOURCE` is a separate funded, signable CLI identity; uploading WASM does not require contract-admin authority.

```bash
export EVENTS_ID=CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ
export ADMIN_SOURCE=$MULTISIG_ADMIN_ADDRESS
export UPLOAD_SOURCE=boundless-upgrade-uploader

stellar contract build
EVENTS_WASM=target/wasm32v1-none/release/boundless_events.wasm

# Upload the exact already-built artifact before starting downtime.
./deploy_mainnet.sh upload-events-wasm "$EVENTS_WASM"

# Guard the current 1.1.0 zero-event state and prepare the pause transaction.
./deploy_mainnet.sh prepare-pause-events /tmp/events-1.2.0-pause.xdr
```

Sign `/tmp/events-1.2.0-pause.xdr` sequentially with the required quorum and submit it with `stellar tx send`. Then:

```bash
./deploy_mainnet.sh upgrade-status
# is_paused must be true.

# Recheck version + pause + zero-event state, then prepare the proposal.
./deploy_mainnet.sh prepare-propose-upgrade-events \
  "$EVENTS_WASM" "1.2.0" /tmp/events-1.2.0-propose.xdr
```

Sign and submit `/tmp/events-1.2.0-propose.xdr`, then verify what actually landed:

```bash
./deploy_mainnet.sh verify-proposed-upgrade-events "$EVENTS_WASM" "1.2.0"
./deploy_mainnet.sh upgrade-status
```

The guarded preparation and verification:

- requires the live version to be `1.1.0` and no other upgrade to be pending;
- requires the pause transaction to have landed and `is_paused` to read back as `true`;
- derives the first possible event ID from `id_base` and requires `get_event` to fail specifically with `EventNotFound`;
- uploads without re-optimizing and requires the returned hash to equal the local file's SHA-256;
- builds and simulates the admin transaction without attempting to bypass the 2-of-3 signing flow;
- reads back both the queued version and exact WASM hash after the signed proposal lands.

The contract remains paused after proposal. Do not unpause during the timelock: `is_paused == true` is what prevents a new event from invalidating the zero-event snapshot before apply. If the event check succeeds or returns any ambiguous RPC error, the script aborts fail-closed. Do not apply 1.2.0; implement a paginated legacy-total migration instead.

Publish the `proposed_at` ledger sequence and `available_at` ledger to the status page so the community has a window to inspect the new wasm before it lands.

### 7.3 Apply

Wait for the current ledger sequence to reach `available_at_ledger`. Keep the exact proposed WASM artifact. On day-of:

```bash
./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh prepare-apply-upgrade-events \
  "$EVENTS_WASM" "1.2.0" /tmp/events-1.2.0-apply.xdr
```

Before preparing apply, the script rechecks the local WASM hash against the queued hash, current version, `is_paused == true`, and the zero-event invariant. Sign and submit `/tmp/events-1.2.0-apply.xdr`, then read status. It must show version `1.2.0`, no pending proposal, and still paused.

Prepare, sign, and submit migration:

```bash
./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh prepare-migrate-upgrade-events \
  "1.2.0" /tmp/events-1.2.0-migrate.xdr
```

After migration lands, prepare, sign, and submit unpause:

```bash
./deploy_mainnet.sh upgrade-status
# get_migrated_to_version must be "1.2.0".
./deploy_mainnet.sh prepare-unpause-events \
  "1.2.0" /tmp/events-1.2.0-unpause.xdr
```

Only after the signed unpause lands:

```bash
./deploy_mainnet.sh verify-upgrade-events "$EVENTS_WASM" "1.2.0"
```

The final verification requires version `1.2.0`, migration marker `1.2.0`, no pending proposal, and `is_paused == false` before updating the local deployment record.

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

If a propose was wrong but not yet applied:

```bash
./deploy_mainnet.sh prepare-cancel-upgrade-events \
  /tmp/events-1.2.0-cancel.xdr
```

Sign and submit the cancel XDR. Cancellation deliberately leaves the pause state unchanged. Either propose the corrected artifact while still paused, or inspect `upgrade-status` and, only when abandoning the upgrade, prepare/sign/send an unpause for the still-live version:

```bash
./deploy_mainnet.sh prepare-unpause-events \
  "1.1.0" /tmp/events-abandon-upgrade-unpause.xdr
```

`apply_upgrade` itself is atomic: a failed apply leaves the old WASM and pending proposal in place. Regenerate the prepared apply XDR because simulated transactions have time bounds. Migration is a separate transaction. If apply lands but migration or final verification fails, 1.2.0 remains paused and the pending proposal is already cleared. Recover with:

```bash
./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh prepare-migrate-upgrade-events \
  "1.2.0" /tmp/events-1.2.0-migrate-recovery.xdr
# Sign and submit the migration XDR.
./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh prepare-unpause-events \
  "1.2.0" /tmp/events-1.2.0-unpause-recovery.xdr
# Sign and submit the unpause XDR.
./deploy_mainnet.sh verify-upgrade-events "$EVENTS_WASM" "1.2.0"
```

`prepare-unpause-events` refuses to prepare a 1.2.0 unpause until its migration marker is exactly `1.2.0`.

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

For the 1.1.0 → 1.2.0 mainnet upgrade, `deploy_mainnet.sh` freezes the zero-event assumption: it pauses before proposal, verifies `is_paused == true` and `EventNotFound`, keeps the contract paused through the timelock, and repeats both checks immediately before apply. Any existing event or ambiguous RPC result aborts the upgrade. New events initialize `NonOwnerContributionTotal` at zero and `add_funds` maintains it in O(1). An older event with no contributors initializes the missing total lazily. If an older event already has contributors, `add_funds` and `start_cancel` fail with `CancellationTotalMissing` instead of using an incomplete total.
