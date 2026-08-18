# Mainnet 1.7.0 upgrade — proposal package

Prepared 2026-08-18. Everything below is verified against the chain at that
time. **Re-run the pre-flight before signing**; the assumptions this upgrade
rests on are about live mainnet state, not about the code.

## Artifact

| | |
|---|---|
| Contract | `CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ` (events) |
| Current version | 1.6.0 |
| Target version | 1.7.0 |
| Wasm sha256 | `b87365c16102f7242a8fc3e770f615991841741c40673bb241cb254130be4c26` |
| Wasm size | 62,110 bytes (94% of the 64 KB ceiling) |
| Built from | `feat/escrow-open-pool` @ `870e9a3` |

Reproduce the hash before signing:

    stellar contract build --package boundless-events
    shasum -a 256 target/wasm32v1-none/release/boundless_events.wasm

These exact bytes ran on testnet (`CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP`)
and passed all 22 smoke scripts plus 312 contract tests.

## Pre-flight (re-run immediately before signing)

The 1.7.0 migration rewrites every `EventRecord`, because `winner_distribution`
and `prize_floors` differ in field name *and* value type, so a pre-1.7.0 row
cannot be decoded by the new struct at all. That rewrite is only safe because
no mainnet event can still reach `select_winners`.

    for id in 271539957145796609 271539957145796610; do
      stellar contract invoke --id CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ \
        --source <any> --network mainnet -- get_event --event_id $id
    done

Expected, and true as of preparation:

| Event | Status | Remaining escrow |
|---|---|---|
| `271539957145796609` | Completed | 0 |
| `271539957145796610` | Completed | 0 |

Also confirm nothing else exists (`#30 EventNotFound` on `…608`, `…611`, `…612`,
`…613`) and that the contract is unpaused.

**If any event reads Active, or a third event exists, stop.** Its record cannot
be rewritten safely while it can still be selected against.

## Signing

Admin is `GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2`, a 2-of-3
multisig (three weight-1 signers, master key weight 0). Two of the three must
sign. No key on the build machine can authorize this.

Unsigned `propose_upgrade` envelope (decode and verify before signing):

    AAAAAgAAAACqr+kenVNRznjlFU8yE+QTqegANttFtJQbOeXujoYHJwAAAGQDxLFqAAAAEAAAAAAAAAAAAAAAAQAAAAAAAAAYAAAAAAAAAAGLUhnQSRnPriBNNLORXFTX2uG/A42dHPozTgZeaFtYxAAAAA9wcm9wb3NlX3VwZ3JhZGUAAAAAAgAAAA0AAAAguHNlwWEC9yQqj8PncPYVmRhBdBxAZzuyQcslQTC+TCYAAAAOAAAABTEuNy4wAAAAAAAAAAAAAAAAAAAA

Decodes to: source `GCVK72I6…`, contract `CCFVEGOQ…`, `propose_upgrade`,
args `b87365c1…` and `"1.7.0"`. Verify with:

    stellar xdr decode --type TransactionEnvelope --output json

The sequence number is baked in. If the multisig account submits anything else
first, rebuild the envelope.

## Sequence

1. **Upload the wasm to mainnet.** Any funded account; needs no admin auth.
   `stellar contract upload --wasm target/wasm32v1-none/release/boundless_events.wasm --source <funded> --network mainnet`
   Must return `b87365c1…`. `apply_upgrade` fails if the wasm is not on the network.
2. **propose_upgrade** — the envelope above, 2 of 3 signatures.
3. **apply_upgrade** — *see the timelock warning below.*
4. **migrate_events(max)** — paged; repeat until `migration_remaining()` is 0.
   Two events, so one call is enough, but check the counter rather than assuming.
5. **migrate()** — one-shot version stamp. Refuses while any event is unconverted.
6. **Verify** `version()` reads `1.7.0` and both events now expose `prize_floors`.
7. **Deploy the backend immediately.** `create_event`, `select_winners`, `submit`,
   `withdraw_submission` and `get_submission` all changed shape, and `Submitted` /
   `SubmissionWithdrawn` now carry a slot. A lagging backend fails publishes
   loudly with no funds moved, which is the safe direction, but do not publish a
   bounty in the gap.
8. Record the result in `deployments/mainnet.json` and `mainnet-upgrades.jsonl`.

## Read before signing

- **The upgrade timelock is 0.** `propose_upgrade` and `apply_upgrade` can land
  in consecutive ledgers, so there is no window in which
  `cancel_pending_upgrade` can catch a mistake. The 1.1.0 upgrade ran with a
  ~1-day timelock. Restoring it is issue #122 and a single-value edit in
  `admin.rs`; doing so changes the wasm hash and invalidates this package.
- **The third-party audit is an open P0** in `BACKLOG.md` under "blocking
  mainnet", and this change touches the money path.
- **PR #120 is unmerged**, with 6 review comments outstanding beyond the owed
  fix that is already in.
- **`deploy_and_upgrade.sh` has an argument-order bug**: `$5` is the source
  account and silently defaults to `alice`. Prefer explicit `stellar contract
  invoke` calls over that script for mainnet.
- **`migrate()` is one-shot.** Once stamped it returns `#69
  MigrationAlreadyApplied` and cannot be re-run, so a migration defect found
  afterwards cannot be repaired by re-migrating. That is what happened on
  testnet.
