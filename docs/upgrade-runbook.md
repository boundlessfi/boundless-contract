# Upgrade runbook

How to upgrade a live `boundless-events` or `boundless-profile` contract. On
mainnet every admin step is a 2-of-3 multisig transaction that
`./deploy_mainnet.sh` prepares and checks but never submits; on testnet
`./deploy_and_upgrade.sh` signs with the local admin identity. Section 8 is
the specific procedure for taking mainnet from events 1.7.0 and profile 1.2.0
to events 2.0.0 and profile 1.2.1.

Related: `docs/contract-ops-runbook.md` (signing and submitting a multisig
transaction), `docs/admin-custody-policy.md` (who signs what),
`docs/DEPLOYMENT.md` (fresh deploys).

## 1. How an upgrade works

| Call | Who | Effect |
|---|---|---|
| `propose_upgrade(new_wasm_hash, new_version)` | admin | Queues the hash and version label. Replaces any earlier proposal. |
| `apply_upgrade()` | admin | After `available_at_ledger` and before `expires_at_ledger`, swaps in the queued wasm and sets `version()`. Atomic: a failed apply leaves the old wasm and the proposal in place. |
| `cancel_pending_upgrade()` | admin | Drops the proposal. |
| `migrate_events(max_events)` | admin, events only | Converts up to `max_events` event records still in an older layout (the contract caps a call at 8) and returns how many remain. |
| `migrate()` | admin | Stamps `get_migrated_to_version()` with the live version. Runs once per version. On events it refuses with `MigrationIncomplete` (97) while `migrate_events` has rows left. |

There is no `upgrade()` entry point. Both contracts expose `version()`,
`get_pending_upgrade()` (hash, version, `proposed_at_ledger`,
`available_at_ledger`, `expires_at_ledger`) and `get_migrated_to_version()`.

| Window | Ledgers | Wall clock | Source |
|---|---|---|---|
| `UPGRADE_TIMELOCK_LEDGERS`, mainnet builds | 17,280 | about 1 day | `contracts/{events,profile}/src/admin.rs` |
| `UPGRADE_TIMELOCK_LEDGERS`, `--features testnet` | 0 | none | same |
| `PENDING_UPGRADE_TTL_LEDGERS` | 518,400 | about 30 days | same |

The timelock that applies is the one compiled into the wasm that is live when
you propose, not the one you are proposing. The deployed mainnet builds,
events 1.7.0 and profile 1.2.0, were compiled with 0 for the August rollout;
events 2.0.0 and profile 1.2.1 restore 17,280.

Pausing events blocks every event-lifecycle call: `create_event`, `add_funds`,
`select_winners`, `claim_prize`, `claim_milestone`, `forfeit_milestone`,
`start_cancel`, `process_cancel_batch`, `finalize_cancel`, `claim_refund`,
and the manager calls (`propose_manager`, `accept_manager`,
`cancel_pending_manager`, `reclaim_management`). Pausing profile blocks its
reputation and earnings writes; events treats those as best-effort, so payouts
still go through. Neither blocks reads or any admin call, including every step
in this runbook and `unpause`, so a contract can stay paused from apply through
migration.

## 2. Before you propose

1. Tag the release commit and build it the way mainnet will run it:

   ```sh
   ./scripts/build-release-linux.sh
   ./scripts/check-versions.sh
   stellar contract info meta --wasm target/release-linux/boundless_events.wasm
   ```

   `build-release-linux.sh` builds in the same Linux x86-64 environment as CI,
   so the hashes it prints must equal the ones in CI's "Build contracts
   reproducibly" step for the tagged commit. A native build on macOS or arm64
   gives a different hash for the same code, because cargo folds the host
   platform into symbol names and that changes the function order, so only
   this build is proposed. The contractmeta `version` must equal the version you will
   propose, and a second person reproduces both hashes with the same script
   before anyone signs. Keep these exact files; every later step checks them.
2. `cargo test --workspace` and `./scripts/test-storage-compat.sh` pass. The
   storage suite replays the real mainnet state through the upgrade
   (`docs/storage-compatibility.md`).
3. The same source has been upgraded on testnet with `./deploy_and_upgrade.sh`
   (Section 5), including every `migrate_events` page and `migrate`.
4. The backend that talks to the new surface is ready to ship before or with
   the upgrade. A backend that lags fails loudly with no funds moved; do not
   publish events in the gap.
5. `EXPECTED_SIGNERS=<G1>,<G2>,<G3> ./scripts/admin/verify-multisig.sh "$ADMIN_SOURCE" mainnet`
   passes: the admin account still has exactly the signer roster on file.
6. Freeze governance for the window: nobody calls `set_admin`, `accept_admin`,
   `propose_events_contract`, `accept_events_contract` or
   `cancel_pending_events_contract` while upgrade envelopes are in flight.
   `deploy_mainnet.sh` refuses every step unless both admins equal
   `ADMIN_SOURCE`, the profile still points at `EVENTS_ID`, and no binding
   rotation is pending.

Environment for `deploy_mainnet.sh`:

```sh
export STELLAR_NETWORK=boundless-mainnet       # see docs/DEPLOYMENT.md 4.2
export ADMIN_SOURCE=<admin multisig G-address>
export EVENTS_ID=<events contract>             # default: deployments/mainnet.json
export PROFILE_ID=<profile contract>
export UPLOAD_SOURCE=<any funded CLI identity> # uploads need no admin authority
```

## 3. Simulate before signing

Every envelope `deploy_mainnet.sh` writes has been through
`stellar tx simulate`. A `--build-only` transaction on its own has a 100-stroop
fee and no Soroban data (no footprint, no resource fee) and is rejected on
submit; never sign one.

Each envelope is built against the multisig account's current sequence
number, so two envelopes prepared back to back carry the same sequence and
only one can land (`tx_bad_seq`). Prepare the next envelope only after the
previous transaction has confirmed, and prepare it close to signing so the
simulation reflects current state and the time bounds have not passed
(`txTooLate`). Before signing, every signer decodes the envelope and checks the
source, contract id, function and arguments:

```sh
stellar tx decode --output json-formatted < events-propose.xdr
```

Signers sign sequentially (each signs the previous signer's output) and the
final XDR is submitted from the CLI with `stellar tx send`
(`docs/contract-ops-runbook.md`, Section 5).

## 4. Mainnet procedure

Run `./deploy_mainnet.sh upgrade-status` before and after every step. It
prints, per contract, the version, live wasm hash, migration marker, a
simulated `migrate` (ready, incomplete or applied), the pending upgrade, the
pause flag and the admin, plus the release validator and the profile binding.

Every `prepare-*` command writes a simulated XDR to the path you give it,
after its checks pass. Sign and submit it before running the next one.

1. Upload each new wasm. No admin authority is needed, so there is no
   ceremony; the command refuses unless the network returns the local file's
   hash.

   ```sh
   ./deploy_mainnet.sh upload-events-wasm <wasm> <version>
   ```

2. Propose, then confirm what landed.

   ```sh
   ./deploy_mainnet.sh prepare-propose-upgrade-events <wasm> <version> propose.xdr
   ./deploy_mainnet.sh verify-proposed-upgrade-events <wasm> <version>
   ```

   The prepare step refuses unless the wasm exports the contract's entry
   points, its contractmeta version equals `<version>`, the live contract
   reports a different version and hash, and nothing is already queued.
   `verify-proposed` prints `available_at_ledger` and `expires_at_ledger`;
   publish both.
3. Wait for `available_at_ledger`. Users can see the proposal on chain for the
   whole window.
4. Events only: pause.

   ```sh
   ./deploy_mainnet.sh prepare-pause-events pause.xdr
   ```

5. Apply. For events the command refuses unless the contract is paused.

   ```sh
   ./deploy_mainnet.sh prepare-apply-upgrade-events <wasm> <version> apply.xdr
   ```

   `upgrade-status` must now show the new version and the new wasm hash.
6. Events only: convert old records.

   ```sh
   ./deploy_mainnet.sh prepare-migrate-events-page 8 page.xdr
   ```

   The command prints how many events the page will leave (simulated). Submit
   it, and repeat until a page reports 0. It refuses once nothing is left to
   convert.
7. Stamp the migration.

   ```sh
   ./deploy_mainnet.sh prepare-migrate-upgrade-events <version> migrate.xdr
   ```

   It refuses while `migrate_events` still has rows (error 97) and if the
   marker already equals `<version>`.
8. Events only: unpause. Refused unless the version is live, nothing is
   queued, and the marker equals the version.

   ```sh
   ./deploy_mainnet.sh prepare-unpause-events <version> unpause.xdr
   ```

9. Confirm the end state and update the local record.

   ```sh
   ./deploy_mainnet.sh verify-upgrade-events <wasm> <version>
   ```

   It requires the exact live hash, the version, the marker, no pending
   upgrade and an unpaused contract, then writes the version and hash to
   `deployments/mainnet.json` and appends to
   `deployments/mainnet-upgrades.jsonl`. Add the transaction hashes to that
   log by hand.

Profile uses the same commands with `-profile` in place of `-events`, skips the
pause and the paging, and has no `migrate_events`.

## 5. Testnet procedure

Testnet builds have no timelock, so each step can follow the last at once.
The contract ids come from `deployments/testnet.json` and the signer defaults
to its `deployer_identity` (argument 5 overrides it).

```sh
./deploy_and_upgrade.sh propose-upgrade events 2.1.0 testnet
./deploy_and_upgrade.sh apply-upgrade events - testnet
./deploy_and_upgrade.sh migrate-events events - testnet   # loops until 0
./deploy_and_upgrade.sh migrate events - testnet
./deploy_and_upgrade.sh status events - testnet
```

`propose-upgrade` builds with `--features testnet`, refuses if the build's
contractmeta version differs from the one you pass, uploads, and proposes.

## 6. Cancelling and rolling back

| Point reached | What you can still do |
|---|---|
| Proposed, not applied | `prepare-cancel-upgrade-<contract>`. The proposal is public for the whole timelock, which is the point of having one. Cancelling leaves the pause flag alone. |
| Applied, no `migrate_events` page yet | Propose and apply the previous wasm. Records have not been touched, but the rollback waits out the timelock of the build that is now live, so keep events paused meanwhile. |
| One or more pages run | Fix forward. Converted records stay converted. |
| Migrated | `migrate` cannot run again for this version (events 69, profile 43). A defect needs a new version. |

Abandoning an upgrade before it is applied: cancel, then unpause. If the live
contract's migration marker already trailed its version before you started,
as it does on mainnet events today, `prepare-unpause-*` refuses; set
`UNPAUSE_UNMIGRATED=1` for that one command, and only for an upgrade that was
never applied.

If anything looks wrong mid-sequence, pause first (2-of-3, never blocked) and
decide afterwards.

Errors you may see while simulating:

| Error | Events | Profile | Meaning |
|---|---|---|---|
| `UpgradeNotProposed` | 65 | 40 | Nothing queued; check `upgrade-status`. |
| `UpgradeTimelockNotElapsed` | 67 | 41 | Too early; wait for `available_at_ledger`. |
| `UpgradeProposalExpired` | 68 | 42 | Cancel and propose again. |
| `MigrationAlreadyApplied` | 69 | 43 | `migrate` already ran for this version. |
| `MigrationIncomplete` | 97 | | `migrate_events` still has rows. |
| `Paused` | 70 | 30 | A lifecycle call hit a paused contract. |

## 7. Rotating the profile's events-contract binding

`set_events_contract` works once, at deploy. If events ever moves to a new
address, profile rotates through a timelock: `propose_events_contract(new)`,
then `accept_events_contract()` after 17,280 ledgers and within 120,960
ledgers (about 7 days) of the proposal; `cancel_pending_events_contract()`
drops it. Each is an admin call on profile, built and simulated the same way:

```sh
stellar contract invoke --network "$STELLAR_NETWORK" --source-account "$ADMIN_SOURCE" \
  --id "$PROFILE_ID" --build-only -- propose_events_contract --new_addr <NEW_EVENTS_ID> \
  | stellar tx simulate --network "$STELLAR_NETWORK" --source-account "$ADMIN_SOURCE" \
  > profile-propose-events-contract.xdr
```

Confirm the proposal with `get_pending_events_contract` before preparing the
accept. `deploy_mainnet.sh` refuses upgrade steps while a rotation is pending,
so finish or cancel it first.

## 8. Mainnet 1.7.0 to 2.0.0

### 8.1 Where mainnet stands

| | events | profile |
|---|---|---|
| Contract | `CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ` | `CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC` |
| Live version | 1.7.0 | 1.2.0 |
| Migration marker | 1.6.0 | none |
| Target | 2.0.0 | 1.2.1 |
| Upgrade timelock of the live build | 0 | 0 |

The admin of both is the 2-of-3 multisig
`GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2` (three weight-1
signers, thresholds 0/2/2, master weight 0). The fee account is
`GADSIP2HPINWTMEUNMVYA5NJRL372OV52HDEJTRTLDZXMHADIPQNYH55` at 250 bps, and the
registered USDC SAC is `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75`.

Events holds 8 events. The 1.7.0 `migrate_events` never ran (there is no
`MigrationCursor`), so events 1 and 2 (`…609` and `…610`) are still in the
pre-1.7.0 record layout and fail to decode on read. Both are Completed with
zero escrow. The other six are in the current layout.

Because the live builds have a zero timelock, each 2.0.0 and 1.2.1 proposal
can be applied as soon as it lands, in the same session. Every upgrade after
this one waits 17,280 ledgers.

Re-read all of this with `upgrade-status` and `get_event` before signing.

### 8.2 What has to ship in the backend first

- Read winners and contributors in pages of at most 90 rows.
- Crank cancellation refunds in batches of at most 15 per
  `process_cancel_batch`.
- Split hackathon selections above 20 awards per `select_winners` call. A
  grant still selects in one call, with at most 40 recipients.
- Call `forfeit_milestone` when a grant milestone is rejected.
- Use `claim_refund` and `get_unclaimed_refund` for refunds the contract had to
  hold back.
- Stop calling the participation entry points (`apply`, `apply_to_bounty`,
  `remove_applicant`, `submit`) and the applicant and submission reads; 2.0.0
  removes them and participation lives in the backend.

### 8.3 Steps

Rehearsed on testnet on 2026-10-02: events `CBEODVJG…` went to 2.0.0 (wasm
`70791d71…`), 185 events paged through `migrate_events` in 24 calls, then
`migrate` stamped 2.0.0. If testnet profile still reports 1.2.0, upgrade it to
1.2.1 there first.

Build both wasms per Section 2 and set the environment:

```sh
export STELLAR_NETWORK=boundless-mainnet
export ADMIN_SOURCE=GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2
export EVENTS_ID=CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ
export PROFILE_ID=CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC
export UPLOAD_SOURCE=<funded identity>
EVENTS_WASM=target/release-linux/boundless_events.wasm
PROFILE_WASM=target/release-linux/boundless_profile.wasm

./deploy_mainnet.sh upgrade-status
./deploy_mainnet.sh upload-profile-wasm "$PROFILE_WASM" 1.2.1
./deploy_mainnet.sh upload-events-wasm "$EVENTS_WASM" 2.0.0
```

Profile first. Either order is safe: events 1.7.0 never calls the
`slash_reputation` entry point that 1.2.1 removes, and events 2.0.0 only calls
functions that 1.2.0 already has. Profile stays unpaused. Each line below is
one signing ceremony; submit it and wait for it to land before the next.

```sh
./deploy_mainnet.sh prepare-propose-upgrade-profile "$PROFILE_WASM" 1.2.1 profile-propose.xdr
./deploy_mainnet.sh verify-proposed-upgrade-profile "$PROFILE_WASM" 1.2.1
./deploy_mainnet.sh prepare-apply-upgrade-profile "$PROFILE_WASM" 1.2.1 profile-apply.xdr
./deploy_mainnet.sh prepare-migrate-upgrade-profile 1.2.1 profile-migrate.xdr
./deploy_mainnet.sh verify-upgrade-profile "$PROFILE_WASM" 1.2.1
```

Then events:

```sh
./deploy_mainnet.sh prepare-pause-events events-pause.xdr
./deploy_mainnet.sh prepare-propose-upgrade-events "$EVENTS_WASM" 2.0.0 events-propose.xdr
./deploy_mainnet.sh verify-proposed-upgrade-events "$EVENTS_WASM" 2.0.0
./deploy_mainnet.sh prepare-apply-upgrade-events "$EVENTS_WASM" 2.0.0 events-apply.xdr
./deploy_mainnet.sh prepare-migrate-events-page 8 events-page.xdr
./deploy_mainnet.sh prepare-migrate-upgrade-events 2.0.0 events-migrate.xdr
./deploy_mainnet.sh prepare-unpause-events 2.0.0 events-unpause.xdr
./deploy_mainnet.sh verify-upgrade-events "$EVENTS_WASM" 2.0.0
```

One page of 8 converts all eight events, so the page command should report 0
remaining. If more events exist by then, repeat the page until it reports 0;
`migrate` refuses with error 97 until it does. Until 2.0.0 is applied,
`upgrade-status` cannot read the release validator (1.7.0 has no such
function).

After the unpause:

- `get_event` on `271539957145796609` and `271539957145796610` decodes and
  shows `prize_floors`.
- `get_release_validator` is `null`, so the admin multisig co-signs
  crowdfunding releases until a validator is appointed. Appointing one is a
  separate decision: `propose_release_validator` from the multisig, then
  `accept_release_validator` signed by the validator key itself within
  120,960 ledgers (`docs/admin-custody-policy.md`, Section 4.1).
- Deploy the backend from the matching commit if it was not already live, and
  record every transaction hash in `deployments/mainnet-upgrades.jsonl`.

Rollback: before the events apply, cancel and unpause with
`UNPAUSE_UNMIGRATED=1` (the marker was already 1.6.0). After the apply, a
rollback to 1.7.0 waits the 17,280-ledger timelock that 2.0.0 carries, with
events paused throughout; after the first page, fix forward.
