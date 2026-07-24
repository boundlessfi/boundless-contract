# SDK 27 compatibility fixture provenance

Fixture hashes and source details are pinned in
`contracts/compatibility/fixtures/manifest.json`.

## Mainnet ledger snapshot

The snapshot is mainnet ledger `63617727`, timestamp `1784850182`, protocol
`27`. It contains all contract-data entries for the events and profile IDs in
the manifest: two contract instances, five profile rows, and three legacy
operation markers. Contract code is loaded separately. TTLs are normalized to
`u32::MAX` so rows do not expire while the host test advances the ledger.

Recreate it with Stellar CLI 27.0.0:

```bash
./scripts/capture-mainnet-compat-snapshot.sh /tmp/mainnet-state-63617727.json
cmp /tmp/mainnet-state-63617727.json \
  contracts/compatibility/fixtures/mainnet-state-63617727.json
```

The script uses SDF's mainnet history archive by default and rejects any
header, selection, count, or hash mismatch. Set `STELLAR_ARCHIVE_URL` to use
another complete mainnet archive.

## WASM provenance and version checks

The two `mainnet-*-1.1.0-sdk23.wasm` files were fetched from the live mainnet
contract IDs above. Their SHA-256 values match the deployed hashes recorded in
the manifest.

The SDK 23 prize/cancellation fixture comes from commit `57aa591`. Its runtime
version is `1.3.0`, but its stale embedded contractmeta says `1.2.0`. Both are
recorded and asserted without changing the fixture.

The SDK 27 events `1.5.0` and profile `1.2.0` files are pinned artifacts built
from release source based on commit `8e15770`. The compatibility gate requires
a fresh locked build to match them before running the cross-version tests.

Synthetic compatibility cases use deterministic operation IDs so their
Soroban ledger snapshots are reproducible. CI reruns the suite and rejects any
snapshot diff.
