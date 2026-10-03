# Contract operations runbook

For the contracts engineer and the multisig signers. This is the canonical
description of how an admin transaction is built, simulated, signed and
submitted, written from the testnet dress rehearsal and the mainnet upgrades.
What to run for a deploy is in `docs/DEPLOYMENT.md`, for an upgrade in
`docs/upgrade-runbook.md`; signer setup is in `docs/multisig-guide.md` and
the policy in `docs/admin-custody-policy.md`. Read Section 1 before you touch
any multisig operation.

---

## 1. Rules learned the hard way

Each of these fails silently or confusingly when you get it wrong.

1. **A Soroban contract call must be simulated before it can be signed.**
   `--build-only` produces a transaction with no footprint and no resource fee
   (`ext = 0`, base fee only). Submitting it gives `TxMalformed`. Pipe it
   through `stellar tx simulate` to attach the Soroban data:
   ```bash
   stellar contract invoke --id <C> --source-account <SRC> --network testnet --build-only -- <fn> \
     | stellar tx simulate --source-account <SRC> --network testnet
   ```
   The output of `tx simulate` is what you sign. This applies to every
   contract call signed offline or by the multisig: `accept_admin`, `pause`,
   `set_fee_bps`, `register_supported_token`, `propose_release_validator`,
   `propose_upgrade`, `apply_upgrade`, `migrate_events`, `migrate`, and the
   rest. The `prepare-*` commands in `deploy_mainnet.sh` do this for you.

2. **Native account operations do not need simulate.** Building the multisig
   itself (`stellar tx new set-options` to add signers or set thresholds) is
   classic Stellar: the CLI signs and submits it directly. Only Soroban
   contract invokes need the simulate step.

3. **Multisig signing is sequential: each signer signs the previous signer's
   output.** Signer 1 signs the prepared XDR and gets XDR-A. Signer 2 signs
   XDR-A, not the original, and gets XDR-B with both signatures. Two separate
   one-signature XDRs do not combine; you get `tx_bad_auth` or below
   threshold.

4. **Submit with the CLI, not Stellar Lab.**
   `stellar tx send "<final-xdr>" --network <net>` prints `SUCCESS` or the
   exact error. Lab's Submit can report a success-looking state while the
   transaction never lands. Use Lab only for signing (it talks to Freighter).

5. **Never mix `--network <name>` with `--network-passphrase`.**
   Doing both makes the CLI stop resolving the RPC URL:
   `error: network passphrase is used but rpc-url is missing`. Pick one:
   - `--network testnet` (carries passphrase and RPC), or
   - `--rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"` with no `--network`.

   On mainnet use one named network (`docs/DEPLOYMENT.md`, 4.2);
   `deploy_mainnet.sh` refuses to run while `STELLAR_RPC_URL` or
   `STELLAR_NETWORK_PASSPHRASE` is set.

6. **`--source-account` accepts a public key (G-address) with `--build-only`.**
   That is how you build a transaction sourced from the multisig, which has no
   secret key locally. Without `--build-only` the CLI tries to sign with that
   key and fails.

7. **Do not run `set_admin` and `accept_admin` back to back.**
   `accept_admin` simulates against the latest closed ledger. If the
   `set_admin` nomination is not in a closed ledger yet, simulation fails with
   no pending rotation (events `#12`, profile `#6`). Wait a few seconds. On
   mainnet the two halves are done by different people at different times.

8. **Simulate, sign and send promptly.** The prepared XDR has time bounds. If
   you sit on it, `tx send` returns `txTooLate`. Rebuild and sign again.

9. **Build the next envelope only after the previous one lands.** Every
   envelope uses the multisig account's current sequence number, so two
   envelopes prepared together carry the same sequence and the second fails
   with `tx_bad_seq`.

10. **The explorer defaults to mainnet.** Testnet accounts and contracts only
    show under `/testnet/`: `https://stellar.expert/explorer/testnet/account/<G...>`
    or `/contract/<C...>`. "Account not found" usually means the wrong
    network, not a failed transaction.

11. **Know which key you are about to sign with.** CLI 28 reads identities and
    networks only from its global config (`~/.config/stellar`, or
    `--config-dir`); a `.stellar` directory in the repo is ignored. An alias
    such as `boundless-deployer` can therefore resolve to a different key on
    another machine. Run `stellar keys address <alias>` before every signing
    session, and keep contract ids in the local deployment record rather than
    in shell variables.

---

## 2. Prerequisites

- Stellar CLI 28.1.0 (`stellar --version`), the version CI and
  `deploy_mainnet.sh` require.
- `jq` and `curl`.
- Each signer has Freighter set up in a dedicated browser profile on their own
  machine, on the right network, and has sent you their G-address
  (`docs/multisig-guide.md`, Part D).

---

## 3. The two transaction shapes

| Shape | Examples | Build and sign |
|---|---|---|
| Native account operation | `set-options` (add signer, set thresholds), payment | `stellar tx new <op> --source-account <key> --network <net>`; the CLI signs and submits if the key is local. For the multisig: `--build-only`, sign in Lab, `stellar tx send`. No simulate. |
| Soroban contract call | `accept_admin`, `pause`, `set_fee_bps`, `register_supported_token`, `propose_upgrade`, `apply_upgrade`, `migrate_events`, `migrate` | `stellar contract invoke ... --build-only -- <fn>`, piped through `stellar tx simulate`, signed in Lab, sent with `stellar tx send`. Simulate is mandatory. |

`accept_release_validator` is the one admin-flow call the multisig does not
sign: the proposed validator key authorizes it. Build it with
`--source-account <validator G-address>` and have whoever holds that key sign
it.

---

## 4. Testnet dress rehearsal

This mirrors the mainnet sequence on throwaway testnet contracts. Reference
values from our drill are in `‹comments›`.

### 4.1 Build
```bash
cd boundless-contract
./scripts/build-release.sh --package boundless-events
./scripts/build-release.sh --package boundless-profile
# target/wasm32v1-none/release/boundless_{events,profile}.wasm
```

### 4.2 Deploy the profile contract (events depends on it)
```bash
DEPLOYER=$(stellar keys address boundless-deployer)   # confirm which key this is

PROFILE_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/boundless_profile.wasm \
  --source boundless-deployer --network testnet \
  -- \
  --admin "$DEPLOYER")
echo "PROFILE_ID=$PROFILE_ID"   # write this down  ‹drill: CA63ATN2…›
```

### 4.3 Deploy the events contract
```bash
FEE_ACCOUNT=$(stellar keys address boundless-fee)   # any valid G-address for the drill

EVENTS_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/boundless_events.wasm \
  --source boundless-deployer --network testnet \
  -- \
  --admin "$DEPLOYER" \
  --fee_account "$FEE_ACCOUNT" \
  --fee_bps 250 \
  --profile_contract "$PROFILE_ID")
echo "EVENTS_ID=$EVENTS_ID"   # write this down  ‹drill: CDP55GFH…›
```

### 4.4 Wire profile to events (first set only)
```bash
stellar contract invoke --id "$PROFILE_ID" --source boundless-deployer --network testnet \
  -- set_events_contract --new_addr "$EVENTS_ID"
```

### 4.5 Register the USDC token
```bash
USDC=CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA   # testnet USDC SAC
stellar contract invoke --id "$EVENTS_ID" --source boundless-deployer --network testnet \
  -- register_supported_token --token "$USDC"
```

### 4.6 Verify the deploy (reads, no signing)
```bash
for fn in version get_admin get_fee_bps get_fee_account get_profile_contract is_paused supported_token_count get_release_validator; do
  echo -n "$fn: "; stellar contract invoke --id "$EVENTS_ID" --source-account "$DEPLOYER" --network testnet --send no -- $fn
done
stellar contract invoke --id "$EVENTS_ID" --source-account "$DEPLOYER" --network testnet --send no \
  -- is_supported_token --token "$USDC"
```
Expect `get_admin` = deployer, `is_supported_token` = `true`,
`supported_token_count` = `1`, `is_paused` = `false`,
`get_release_validator` = `null`.

### 4.7 Build the 2-of-3 multisig
Signers are you, your co-founder, and a cold recovery key held offline.
Thresholds 0/2/2, master disabled.
```bash
stellar keys generate boundless-multisig-bootstrap --network testnet
BOOT=$(stellar keys address boundless-multisig-bootstrap)   # ‹drill: GDLEW45L…›
curl "https://friendbot.stellar.org/?addr=$BOOT"

# Add the three signers (native op, no simulate), one transaction each
for G in G_YOU... G_COFOUNDER... G_COLD... ; do
  stellar tx new set-options --source-account boundless-multisig-bootstrap \
    --signer "$G" --signer-weight 1 --network testnet
done

# Lock it: 2-of-3 and disable the bootstrap master key (the critical flag)
stellar tx new set-options --source-account boundless-multisig-bootstrap \
  --low-threshold 0 --med-threshold 2 --high-threshold 2 --master-weight 0 \
  --network testnet

./scripts/admin/verify-multisig.sh "$BOOT" testnet   # must print: PASS: all 8 checks passed
```
Read the three signer addresses the script prints and confirm them by eye.
With `EXPECTED_SIGNERS=<G1>,<G2>,<G3>` set, the script also pins the roster
and runs a ninth check.

### 4.8 Rotate admin: deployer to multisig
Do this for both `$EVENTS_ID` and `$PROFILE_ID`.

```bash
# Step A: the current admin (deployer) nominates the multisig. CLI signs and submits.
stellar contract invoke --id "$EVENTS_ID" --source boundless-deployer --network testnet \
  -- set_admin --new_admin "$BOOT"
```
Wait about ten seconds (Rule 7), then:
```bash
# Step B: build and simulate accept_admin with the multisig as source
stellar contract invoke --id "$EVENTS_ID" --source-account "$BOOT" --network testnet --build-only -- accept_admin \
  | stellar tx simulate --source-account "$BOOT" --network testnet
```
Sign the prepared XDR in Lab (you, then co-founder) and submit:
```bash
stellar tx send "<XDR-with-both-signatures>" --network testnet
```
Verify:
```bash
stellar contract invoke --id "$EVENTS_ID" --source-account "$BOOT" --network testnet --send no -- get_admin
# $BOOT
```
Repeat A and B for `$PROFILE_ID`.

### 4.9 Operate as the multisig, and see the failure mode
The deployer now has no admin power. Prove the multisig can run operations:
```bash
stellar contract invoke --id "$EVENTS_ID" --source-account "$BOOT" --network testnet --build-only -- pause \
  | stellar tx simulate --source-account "$BOOT" --network testnet
```
Sign 2-of-3 in Lab, `stellar tx send`, and `is_paused` reads `true`. Then
`unpause` the same way.

The 1-of-3 drill: build a `pause`, submit it with one signature, and
`tx send` rejects it (`tx_bad_auth`). Seeing this on purpose is the point.

---

## 5. Day-to-day multisig operation

Any time the multisig makes a contract call (`set_fee_bps`, `pause`,
`register_supported_token`, a release-validator change, an upgrade step):

```bash
# 0. Confirm the admin account is still the multisig on file
EXPECTED_SIGNERS=<G1>,<G2>,<G3> ./scripts/admin/verify-multisig.sh "$BOOT" <testnet|mainnet>

# 1. Build and simulate: a prepared, unsigned XDR
stellar contract invoke --id <CONTRACT_ID> --source-account "$BOOT" --network <net> --build-only -- <fn> [--arg val ...] \
  | stellar tx simulate --source-account "$BOOT" --network <net> > op.xdr

# 2. Every signer decodes it and checks source, contract, function and arguments
stellar tx decode --output json-formatted < op.xdr

# 3. Sign in Lab, in order
#    signer 1 signs the prepared XDR -> XDR-A
#    signer 2 signs XDR-A            -> XDR-B

# 4. Submit from the CLI, so errors are visible
stellar tx send "<XDR-B>" --network <net>

# 5. Verify with a read
stellar contract invoke --id <CONTRACT_ID> --source-account "$BOOT" --network <net> --send no -- <getter>
```

For upgrades, pause and unpause on mainnet, `deploy_mainnet.sh prepare-*`
replaces step 1 and adds the state checks in `docs/upgrade-runbook.md`.

An emergency `pause` is the one operation that may go ahead before the paper
trail. Pause first, then write the incident record in `#ops-admin-requests`:
what was seen, who signed, the transaction hash, and what has to be true
before anyone prepares the `unpause`.

The policy keeps `set_fee_account`, `set_admin` and
`propose_release_validator` at 3-of-3 (`docs/admin-custody-policy.md`,
Section 4). That is a process rule: collect all three signatures, including
the cold key. The chain only enforces 2-of-3.

---

## 6. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `network passphrase is used but rpc-url is missing` | Mixed `--network <name>` with `--network-passphrase` | Use the named network alone, or go fully explicit with `--rpc-url ... --network-passphrase ...` (Rule 5) |
| `TxMalformed` when submitting a contract call | The XDR was `--build-only` and never simulated | Pipe through `stellar tx simulate` and sign that output (Rule 1) |
| `tx_bad_auth` / `op_low_threshold` | Not enough signatures combined, usually because the second signer signed the original instead of the first signer's output | Re-sign in order (Rule 3) |
| `tx_bad_seq` | Two envelopes were prepared against the same sequence | Rebuild the second after the first lands (Rule 9) |
| `Error(Contract, #12)` on events or `#6` on profile from `accept_admin` | No pending rotation yet: `set_admin` is not in a closed ledger | Wait a few seconds and simulate again (Rule 7); if it persists, re-run `set_admin` |
| `Error(Contract, #13)` on events or `#7` on profile | The nomination expired (120,960 ledgers) | Re-run `set_admin`, then accept promptly |
| Submit fails with an auth error although simulation passed | The source is not the current admin, so the recorded authorization belongs to an address nobody signed for | Check `get_admin` and build as the real admin |
| `txTooLate` | Time bounds passed between simulate and submit | Rebuild and sign quickly (Rule 8) |
| "Account/contract not found" in the explorer | Looking at mainnet | Use the `/testnet/` URL (Rule 10) |
| Lab says submitted but nothing changed | Lab's submit failed silently | Submit with `stellar tx send` to see the real error (Rule 4) |

Upgrade-specific errors are listed in `docs/upgrade-runbook.md`, Section 6.

Read-only diagnosis:
```bash
# Did a transaction land on an account?
curl -s "https://horizon-testnet.stellar.org/accounts/<G>/transactions?order=desc&limit=5" \
  | jq -r '._embedded.records[] | "\(.created_at) successful=\(.successful) \(.hash)"'
# Multisig configuration
curl -s "https://horizon-testnet.stellar.org/accounts/<BOOT>" | jq '{signers,thresholds}'
```

---

## 7. Mainnet differences

Sections 4 and 5 hold on mainnet except:
- Build without `--features testnet`, so the 17,280-ledger upgrade timelock is
  compiled in. `deploy_mainnet.sh` does this.
- Use the named mainnet network from `docs/DEPLOYMENT.md` 4.2.
- Fund the bootstrap with real XLM (at least 5), not friendbot.
- The cold recovery key lives in a safe; the daily signers are you and your
  co-founder.
- After the rotation, destroy the deployer key (`stellar keys rm`); it has no
  power left, but leave nothing lying around.
- Register USDC at deploy time. The token index is then complete from genesis,
  so `supported_token_count` and `supported_token_at` are authoritative.
