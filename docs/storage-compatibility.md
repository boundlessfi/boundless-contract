# Storage compatibility

`contracts/compatibility` replays the next mainnet upgrade against real mainnet
storage. It loads contract data captured from a mainnet ledger, runs the builds
deployed on mainnet at that ledger, then proposes, applies and migrates the
release build of this tree the way `docs/upgrade-runbook.md` does it on chain.
Mainnet runs events 1.8.0 and profile 1.2.1, and this tree's profile is still
1.2.1, so only events upgrades:

1. Read every event, every winner record and the reputation of every winner
   through the deployed events 1.8.0 and profile 1.2.1 builds. Every event must
   read.
2. Propose the release build of events, check that `apply_upgrade` refuses
   until the 17,280-ledger timelock the deployed build carries has passed, then
   advance the ledger, pause, apply, page `migrate_events` until it reports 0,
   stamp with `migrate`, and unpause.
3. Read everything again through events 2.1.0 and the same profile and require
   the same values.
4. Run a grant through the upgraded events contract on a fresh token
   (selection, releases, a forfeit, the close that returns the forfeited share)
   and check that event ids continue from the deployed counter, then a grant
   published with a milestone split, paid in full.

The test fails straight away if `contracts/profile` no longer reports the
deployed profile version, because a profile change has to be added to the
replay before it ships.

The test is marked `#[ignore]` because it needs the release wasm, which a plain
`cargo test` does not build. Run it with:

```bash
./scripts/test-storage-compat.sh
```

The script checks the fixture hashes, the contractmeta versions and the snapshot
provenance against `contracts/compatibility/fixtures/manifest.json`, builds the
release wasm with `scripts/build-release.sh`, and runs the replay. CI runs
the same script after its reproducible build. `BOUNDLESS_WASM_DIR` points the
test at a different build directory.

## Fixtures

`mainnet-events-1.8.0.wasm` and `mainnet-profile-1.2.1.wasm` were fetched by hash
from the live mainnet contracts recorded in the manifest
(`stellar contract fetch --wasm-hash <hash> --network mainnet`). Uploading them is
what makes the captured contract instances runnable, so a wrong file fails the
test before any assertion.

`mainnet-state.json` holds every contract-data entry of the two contracts at
mainnet ledger 65,003,455 (protocol 29), the first archive checkpoint after
the 1.8.0 upgrade was published. Contract code is
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

Once mainnet runs a newer build, the replay should start from it instead:

1. Fetch the deployed wasm for both contracts by hash into the fixtures
   directory and replace the two `mainnet-*.wasm` files.
2. Capture a new ledger with
   `./scripts/capture-mainnet-compat-snapshot.sh /tmp/mainnet-state.json <ledger>`
   (an archive checkpoint), copy it to `fixtures/mainnet-state.json`, and record
   the printed ledger, timestamp, protocol and hash in the manifest.
3. Update the deployed and release versions the test expects
   (`DEPLOYED_EVENTS`, `DEPLOYED_PROFILE`, `RELEASE_EVENTS`), and add a profile
   upgrade to the replay if this tree changes the profile.
