# Storage compatibility

`contracts/compatibility` replays the next mainnet upgrade against real mainnet
storage. It loads contract data captured from a mainnet ledger, runs the builds
deployed on mainnet today, then proposes, applies and migrates the release build
of this tree the way `docs/upgrade-runbook.md` does it on chain:

1. Read every event, every winner record and the reputation of every winner
   through the deployed events 1.7.0 and profile 1.2.0 builds. The two records
   still in the pre-1.7.0 layout cannot be read by 1.7.0, and the test requires
   exactly those two to fail.
2. Upgrade the profile (propose, apply, migrate; it stays running), then pause
   events, propose and apply its release build (the deployed builds have no
   upgrade timelock), page `migrate_events`, check that `migrate` refuses with
   `MigrationIncomplete` before the conversion and stamps after it, and
   unpause.
3. Read everything again through events 2.0.0 and profile 1.2.1 and require the
   same values. The two converted records must carry prize floors equal to what
   each position was paid.
4. Run a grant through the upgraded pair on a fresh token (selection, releases,
   a forfeit, the close that returns the forfeited share) and check that event
   ids continue from the deployed counter.

The test is marked `#[ignore]` because it needs the release wasm, which a plain
`cargo test` does not build. Run it with:

```bash
./scripts/test-storage-compat.sh
```

The script checks the fixture hashes, the contractmeta versions and the snapshot
provenance against `contracts/compatibility/fixtures/manifest.json`, builds the
release wasm with `stellar contract build --locked`, and runs the replay. CI runs
the same script after its reproducible build. `BOUNDLESS_WASM_DIR` points the
test at a different build directory.

## Fixtures

`mainnet-events-1.7.0.wasm` and `mainnet-profile-1.2.0.wasm` were fetched by hash
from the live mainnet contracts recorded in the manifest
(`stellar contract fetch --wasm-hash <hash> --network mainnet`). Uploading them is
what makes the captured contract instances runnable, so a wrong file fails the
test before any assertion.

`mainnet-state.json` holds every contract-data entry of the two contracts at
mainnet ledger 64,747,647 (protocol 29). Contract code is
excluded because the wasm fixtures supply it, and TTLs are normalised to
`u32::MAX` so nothing expires while the test advances the ledger. The SDK 28 test
host runs protocol 28, so the replay loads the snapshot at protocol 28; nothing
the contracts store depends on the protocol version.

To check that the fixture still reproduces from the history archive (Stellar CLI
28.1.0, read-only, downloads several gigabytes of archive buckets):

```bash
./scripts/capture-mainnet-compat-snapshot.sh /tmp/mainnet-state.json
```

## Refreshing for the next upgrade

Once mainnet runs 2.0.0, the replay should start from 2.0.0 instead:

1. Fetch the deployed wasm for both contracts by hash into the fixtures
   directory and replace the two `mainnet-*.wasm` files.
2. Capture a new ledger with
   `./scripts/capture-mainnet-compat-snapshot.sh /tmp/mainnet-state.json <ledger>`
   (an archive checkpoint), copy it to `fixtures/mainnet-state.json`, and record
   the printed ledger, timestamp, protocol and hash in the manifest.
3. Update the versions the test expects and drop the assertions about the
   pre-1.7.0 records, which will no longer exist.
