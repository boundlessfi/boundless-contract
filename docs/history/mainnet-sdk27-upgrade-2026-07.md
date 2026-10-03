# Mainnet SDK 27 upgrade procedure and 2026-06 audit delta

> Historical record. Sections 7.2 to 7.5 and 8 of the former
> `docs/mainnet-deploy-runbook.md`, last revised 2026-07-24. They were the
> pinned procedure for moving mainnet from SDK 23 to SDK 27 builds (events
> 1.1.0 to 1.5.0 and profile 1.1.0 to 1.2.0, behind a zero-event guard), plus
> a summary of the contract surface changes from the June 2026 audit. Mainnet
> never ran these exact builds: the operator's upgrade log records events going
> from 1.1.0 to 1.6.0 between 2026-07-28 and 2026-07-30, and profile reached
> 1.2.0 in the August upgrade (`mainnet-upgrade-runbook-2026-08.md`).
> `docs/upgrade-runbook.md` replaces this procedure, and `deploy_mainnet.sh` no
> longer pins versions or hashes. Commands and paths are as written then;
> `scripts/test-sdk27-compat.sh` and `docs/sdk27-compatibility-fixtures.md`
> have since become `scripts/test-storage-compat.sh` and
> `docs/storage-compatibility.md` and cover a later upgrade.

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
./scripts/test-sdk27-compat.sh
```

CI must have rebuilt the contracts and matched the pinned fixtures. Abort if a
fixture hash differs, a compatibility test fails, or the zero-event check no
longer returns `EventNotFound`. The SDK 27 contracts retain a read fallback for
legacy temporary `OpSeen(BytesN<32>)` rows; new markers use the domain-scoped
key.

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

./scripts/test-sdk27-compat.sh

EVENTS_WASM=contracts/compatibility/fixtures/events-1.5.0-sdk27.wasm
PROFILE_WASM=contracts/compatibility/fixtures/profile-1.2.0-sdk27.wasm

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

Do not unpause either contract during the timelock. If `get_event` finds an
event or returns anything other than `EventNotFound`, abort without applying
either upgrade and investigate. Publish both `proposed_at_ledger` and
`available_at_ledger` values. Apply only after the later
`available_at_ledger` value has been reached.

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

- `UpgradeTimelockNotElapsed`: wait longer; leave the contract paused.
- `UpgradeProposalExpired`: cancel the pending proposal, then propose again.
- `UpgradeNotProposed`: no proposal is in storage; inspect `upgrade-status`.

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

## 8. 2026-06 Stellar-skill audit: delta from the previous runbook

This section summarizes everything that changed in the contract surface between this revision and the prior runbook. Use it when reviewing customer / partner integrations.

### 8.1 Removed

- `upgrade(new_wasm_hash)`: replaced by `propose_upgrade` + `apply_upgrade`. See Section 7.
- `cancel_event(event_id, op_id)`: replaced by `start_cancel` + `process_cancel_batch` + `finalize_cancel`. See Section 8.4.
- `Profile.wins_count`, `submissions_count`, `applications_count`, `milestones_completed`: the off-chain indexer derives counters from the emitted events instead.
- `__link_keep`: linker keep-alive trick, no longer needed.

### 8.2 Added

- `version() -> String`: on-chain semver. Returns that package's `INITIAL_VERSION`; bumped by `apply_upgrade`.
- `propose_upgrade(wasm_hash, new_version)`, `apply_upgrade()`, `cancel_pending_upgrade()`, `migrate()`: timelocked upgrade flow.
- `start_cancel(event_id, op_id)`, `process_cancel_batch(event_id, max_refunds, op_id) -> u32`, `finalize_cancel(event_id, op_id)`: paged cancellation.
- `propose_events_contract(addr)`, `accept_events_contract()`, `cancel_pending_events_contract()`: two-step rotation for the profile contract's events binding. First-set still uses `set_events_contract`.
- Paged read accessors on the events contract: `get_winner_count`, `get_winner_at`, `get_contributor_count`, `get_contributor_at`. The aggregated reads (`get_winners`, `get_contributors`) cap at the per-event maximum and stay available for backwards compat. The applicant accessors went with the participation records in 2.0.0.
- New `EventStatus::Cancelling` variant. Any switch on EventStatus needs to handle it.
- New events: `PendingUpgradeProposed`, `UpgradeApplied`, `Migrated`, `PendingUpgradeCancelled`, `PendingEventsContractSet`, `EventsRotationCancelled`. `EventCreated` adds a `title` field.

### 8.3 Behavioral changes

- **`select_winners` math (M1)** now pays against `remaining_escrow` at the moment of the call, snapshotted before any release. Partner top-ups via `add_funds` flow to winners instead of staying trapped until cancel.
- **Crowdfunding `claim_milestone` requires admin co-sign (M5).** The builder's auth is necessary but no longer sufficient. Off-chain admin tooling must co-sign the tx.
- **`MAX_FEE_BPS` is 1_000 (10%) (L4)**, down from 5_000 (50%). Per-event overrides still respect the cap.
- **Storage type changes (H1)**: admin/config now lives in `instance()` storage. Persistent reads of event-scoped data bump TTL on every read (H2). No off-chain wiring change; deploy fresh contracts and the new layout takes effect.
- **Per-event lists are paged (H3, H4)**: `Vec<Address>` lists and the `Map<Address, Submission>` collapse into per-element keys with an index counter. Soft caps at `MAX_APPLICANTS_PER_EVENT = 5_000` and `MAX_CONTRIBUTORS_PER_EVENT = 5_000`. Submissions key per-`(event, applicant)`. The aggregated reads cap at these limits.

### 8.4 Paged cancel: sequence diagram

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
