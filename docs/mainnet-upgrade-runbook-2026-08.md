# Mainnet upgrade runbook — events 1.7.0 + profile 1.2.0

Prepared 2026-08-18 from `feat/escrow-open-pool` @ `870e9a3`.

Every chain reading below was taken at preparation time. **Re-read them before
signing.** What makes this upgrade safe is the state of mainnet, not the state
of the code.

---

## 1. Artifacts

| | events | profile |
|---|---|---|
| Contract | `CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ` | `CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC` |
| On chain now | 1.6.0 | 1.1.0 |
| Target | 1.7.0 | 1.2.0 |
| Wasm sha256 | `b87365c16102f7242a8fc3e770f615991841741c40673bb241cb254130be4c26` | `6b4804920a4068dfdcaaa711fac6390850e0a92f4b5ed4e46259694b61cbae12` |
| Size | 62,110 bytes (94% of the 64 KB ceiling) | 16,148 bytes |
| Data migration | **Yes — rewrites every EventRecord** | No; `migrate()` is a version stamp with an empty dispatch |

Reproduce both hashes before signing:

```
stellar contract build --package boundless-events
stellar contract build --package boundless-profile
shasum -a 256 target/wasm32v1-none/release/boundless_events.wasm
shasum -a 256 target/wasm32v1-none/release/boundless_profile.wasm
```

Profile's public interface is byte-identical between 1.1.0 and 1.2.0 (verified
by diffing `pub fn` signatures). Its delta is internal: `checked_add` over
`saturating_add` with typed errors (#101), non-malleable child op_id derivation
and namespaced `OpSeen` (#95), SDK 27 storage gating (#102), and the zeroed
timelock. Because the interface is unchanged, events 1.7.0 runs against either
profile version, so the ordering below is convention, not a hard dependency.

---

## 2. Pre-flight — re-run immediately before signing

The events migration rewrites every `EventRecord`, because `winner_distribution`
and `prize_floors` differ in field name *and* value type: a pre-1.7.0 row cannot
be decoded by the new struct at all. That is only safe because no mainnet event
can still reach `select_winners`.

```
for id in 271539957145796609 271539957145796610; do
  stellar contract invoke --id CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ \
    --source <any> --network mainnet -- get_event --event_id $id
done
```

Expected, and true at preparation:

| Event | Status | Remaining escrow |
|---|---|---|
| `271539957145796609` | Completed | 0 |
| `271539957145796610` | Completed | 0 |

Then confirm:

- `…608`, `…611`, `…612`, `…613` all return `#30 EventNotFound` (check the error
  code, not just the absence of output — a loose probe reports false positives).
- `is_paused` is `false` on both contracts.
- `get_pending_upgrade` is `null` on both. It was at preparation; a stale
  proposal must be cleared with `cancel_pending_upgrade` first.

**Stop if any event reads Active, or a third event exists.** Its record cannot
be rewritten safely while it can still be selected against.

---

## 3. Two things that will bite you

**The sequence number.** Both prepared envelopes were built against the admin
account's current sequence, so they carry the *same* `seq_num`
(`271536946373722128`). Only one can ever be submitted; the second fails with
`tx_bad_seq`. **Build each envelope only after the previous transaction has
confirmed.** The same applies to every `apply_upgrade`, `migrate_events` and
`migrate` call below — each is a separate admin transaction consuming a
sequence number.

**The timelock is zero.** `UPGRADE_TIMELOCK_LEDGERS = 0` on both contracts, so
`propose_upgrade` and `apply_upgrade` can land in consecutive ledgers and there
is no window in which `cancel_pending_upgrade` can catch a mistake. The 1.1.0
upgrade ran with roughly a day. Restoring it is issue #122 and a single-value
edit in `admin.rs` — but it changes both wasm hashes and invalidates every
envelope here.

---

## 4. Signing

Admin for both contracts is `GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2`:
a 2-of-3 multisig (three weight-1 signers, master key weight 0 and therefore
unable to authorize anything on its own). No key on the build machine can sign
this. Follow `docs/contract-ops-runbook.md` Section 5 for the ceremony; sign
sequentially, then submit with `stellar tx send "<signed-xdr>" --network mainnet`.

Decode every envelope before signing:

```
stellar xdr decode --type TransactionEnvelope --output json <<< "<xdr>"
```

---

## 5. Sequence

Profile first, matching the deploy convention that events depends on profile.
Each numbered step is a separate 2-of-3 signing ceremony unless marked read-only.

### 5.1 Upload both wasms

Any funded mainnet account; **no admin auth required**, so this needs no
ceremony. `apply_upgrade` fails if the wasm is not already on the network.

```
stellar contract upload --wasm target/wasm32v1-none/release/boundless_profile.wasm \
  --source <funded> --network mainnet     # must print 6b480492…
stellar contract upload --wasm target/wasm32v1-none/release/boundless_events.wasm \
  --source <funded> --network mainnet     # must print b87365c1…
```

### 5.2 Profile: 1.1.0 → 1.2.0

1. **propose_upgrade** — prepared envelope:

```
AAAAAgAAAACqr+kenVNRznjlFU8yE+QTqegANttFtJQbOeXujoYHJwAAAGQDxLFqAAAAEAAAAAAAAAAAAAAAAQAAAAAAAAAYAAAAAAAAAAH2o/HE+cZznpgt904v7VoQtlwY8dnSkPqh/UC5V+HG0gAAAA9wcm9wb3NlX3VwZ3JhZGUAAAAAAgAAAA0AAAAga0gEkgpAaN/cqqcR+sY5CFDgqS9LXtTkYllpS2HLrhIAAAAOAAAABTEuMi4wAAAAAAAAAAAAAAAAAAAA
```

Decodes to: source `GCVK72I6…`, contract `CD3KH4OE…`, `propose_upgrade`,
args `6b480492…` and `"1.2.0"`.

2. **apply_upgrade** — build fresh (sequence has moved):

```
stellar contract invoke --id CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC \
  --source boundless-mainnet-msig --network mainnet --build-only -- apply_upgrade
```

3. **migrate** — version stamp only; the dispatch is empty, so nothing is
   rewritten. Build fresh, same pattern.

4. **Verify (read-only):** `version()` reads `"1.2.0"`, `is_paused` still
   `false`, `get_admin` unchanged, and `get_events_contract` still points at
   `CCFVEGOQ…`.

### 5.3 Events: 1.6.0 → 1.7.0

1. **propose_upgrade** — prepared envelope (rebuild if profile's steps consumed
   this sequence, which they will):

```
AAAAAgAAAACqr+kenVNRznjlFU8yE+QTqegANttFtJQbOeXujoYHJwAAAGQDxLFqAAAAEAAAAAAAAAAAAAAAAQAAAAAAAAAYAAAAAAAAAAGLUhnQSRnPriBNNLORXFTX2uG/A42dHPozTgZeaFtYxAAAAA9wcm9wb3NlX3VwZ3JhZGUAAAAAAgAAAA0AAAAguHNlwWEC9yQqj8PncPYVmRhBdBxAZzuyQcslQTC+TCYAAAAOAAAABTEuNy4wAAAAAAAAAAAAAAAAAAAA
```

Decodes to: source `GCVK72I6…`, contract `CCFVEGOQ…`, `propose_upgrade`,
args `b87365c1…` and `"1.7.0"`.

Rebuild with:

```
stellar contract invoke --id CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ \
  --source boundless-mainnet-msig --network mainnet --build-only \
  -- propose_upgrade --new_wasm_hash b87365c16102f7242a8fc3e770f615991841741c40673bb241cb254130be4c26 --new_version "1.7.0"
```

2. **apply_upgrade** — build fresh.

3. **migrate_events(max_events)** — paged; **this is the point of no return**
   (see §6). Two events, so one call with `--max_events 8` suffices, but drive
   it off the counter rather than the assumption:

```
stellar contract invoke --id CCFVEGOQ… --source <any> --network mainnet -- migration_remaining   # read-only
```

Repeat `migrate_events` until `migration_remaining` reads `0`.

4. **migrate** — one-shot version stamp. It refuses while any event is still
   unconverted, so step 3 must be complete.

5. **Verify (read-only):** `version()` reads `"1.7.0"`, and both events now
   expose `prize_floors` instead of `winner_distribution`:

```
stellar contract invoke --id CCFVEGOQ… --source <any> --network mainnet -- get_event --event_id 271539957145796609
```

Expect `prize_floors: {"1": "100000000"}` on `…609` and
`{"1": "600000000", "2": "400000000"}` on `…610` — the absolute amounts the old
percentages would have paid against each budget.

### 5.4 Backend cutover — immediately after §5.3 step 4

`create_event`, `select_winners`, `submit`, `withdraw_submission` and
`get_submission` all changed shape, and `Submitted` / `SubmissionWithdrawn` now
carry a slot. Deploy `boundless-nestjs` from the matching commit as soon as
`migrate()` confirms.

A lagging backend fails publishes loudly with no funds moved, which is the safe
direction — but **do not publish a bounty in the gap**.

### 5.5 Record the result

Update `deployments/mainnet.json` (version, both wasm hashes, `last_upgraded_at`)
and append to `deployments/mainnet-upgrades.jsonl`.

---

## 6. Rollback

The window closes in stages:

| Point | Recoverable? |
|---|---|
| After `propose_upgrade` | Yes — `cancel_pending_upgrade`. With a zero timelock this window is only as long as you leave it. |
| After `apply_upgrade`, before `migrate_events` | Yes — propose and apply the previous wasm. Records are still in 1.6.0 shape and untouched. |
| After `migrate_events` | **No.** Records are rewritten to `prize_floors`; 1.6.0 code cannot decode them, so reverting the wasm bricks reads. |
| After `migrate` | No, and `migrate()` cannot be re-run: it returns `#69 MigrationAlreadyApplied`. A defect found later cannot be fixed by re-migrating. |

Previous events wasm for a pre-migration rollback:
`43b83ae4e5ae689f004278f936a8616a2add5e09bb1024e8830c7f266f464963`.
Previous profile wasm:
`b9e3500cfb559781e68359655f37775c78afffc5a8436daca9d1d1f8e3569b02`.

If something looks wrong mid-sequence, `pause` is a 2-of-3 call and stops user
operations while you decide. See `docs/mainnet-deploy-runbook.md` §329.

---

## 7. What a signer should weigh

- **The third-party audit is an open P0** in `BACKLOG.md` under "blocking
  mainnet", and this change touches the money path.
- **The timelock is zero** on both contracts (§3).
- **PR #120 is unmerged**, with 6 review comments still open beyond the owed-
  reservation fix that is already in this build.
- **`deploy_and_upgrade.sh` has an argument-order bug** — `$5` is the source
  account and silently defaults to `alice`. Use explicit `stellar contract
  invoke` calls for mainnet, as written above.

Testing behind this build: 312 contract tests, 2208 backend tests, and the
22-script smoke suite green on testnet. Both binaries are deployed on testnet,
so what mainnet would run has run on a chain.
