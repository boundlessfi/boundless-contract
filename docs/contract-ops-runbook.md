# Boundless contract operations runbook (testnet drill + multisig ops)

**For:** the contracts engineer and the multisig signers.
**Companion docs:** `multisig-guide.md` (plain-English signer setup), `admin-custody-policy.md` (policy), `mainnet-deploy-runbook.md` (mainnet cold deploy).
**Why this exists:** the mainnet runbook tells you *what* to run. This runbook tells you *how it actually behaves* — the silent-failure modes we hit during the testnet dress rehearsal and the exact fixes. Read Section 1 before you touch any multisig op.

---

## 1. Hard-won rules (read this first)

These are the things that cost us hours on testnet. Every one of them fails **silently or confusingly** if you get it wrong.

1. **A Soroban contract call must be SIMULATED before it can be signed.**
   `--build-only` produces a transaction with **no footprint and no resource fee** (`ext = 0`, fee = base only). Submitting it gives `TxMalformed`. You MUST pipe it through `stellar tx simulate` to attach the Soroban data:
   ```bash
   stellar contract invoke --id <C> --source-account <SRC> --network testnet --build-only -- <fn> \
     | stellar tx simulate --source-account <SRC> --network testnet
   ```
   The output of `tx simulate` is what you sign. This applies to **every** contract call signed offline/multisig (`accept_admin`, `pause`, `set_fee_bps`, `register_supported_token`, `propose_upgrade`, …).

2. **Native account operations do NOT need simulate.** Building the multisig itself (`stellar tx new set-options` to add signers / set thresholds) is classic Stellar — the CLI signs and submits it directly. Only **Soroban contract invokes** need the simulate step.

3. **Multisig signing is sequential: each signer signs the PREVIOUS signer's output.**
   Signer 1 signs the prepared XDR → gets XDR-A. Signer 2 signs **XDR-A** (not the original) → gets XDR-B with both signatures. Two separate one-signature XDRs **do not combine** — you'll get `tx_bad_auth` / below threshold.

4. **Submit with the CLI, not Stellar Lab, while operating.**
   `stellar tx send "<final-xdr>" --network <net>` prints `SUCCESS` or the **exact** error. Lab's "Submit" can report success-looking states while the tx never lands. Use Lab only for *signing* (it talks to Freighter); submit from the CLI so failures are loud.

5. **Never mix `--network <name>` with `--network-passphrase`.**
   Doing both makes the CLI stop resolving the RPC URL → `error: network passphrase is used but rpc-url is missing`. Pick ONE:
   - `--network testnet` (carries passphrase + RPC), **or**
   - `--rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"` (no `--network`).

6. **`--source-account` accepts a public key (G-address) when `--build-only` is set.**
   That's how you build a transaction sourced from the multisig (which has no secret key locally). Without `--build-only`, the CLI tries to sign with that key and fails.

7. **Don't run `set_admin` and `accept_admin` instantly back-to-back.**
   `accept_admin` simulates against the latest closed ledger. If the `set_admin` nomination isn't in a closed ledger yet, you get `Error(Contract, #6)` = `PendingAdminMismatch`. Wait a few seconds (or run them as separate steps). On mainnet this never happens — the two halves are done by different people at different times.

8. **Simulate → sign → send promptly.** The prepared XDR has time bounds. If you sit on it, `tx send` returns `txTooLate`. Just regenerate (re-run the build-only | simulate) and sign again.

9. **The explorer defaults to mainnet.** Testnet accounts/contracts only show under the `/testnet/` path: `https://stellar.expert/explorer/testnet/account/<G...>` or `/contract/<C...>`. "Account not found" on the default explorer almost always means you're on the wrong network, not that the tx failed.

10. **Key/contract-id drift is real.** A keystore alias (e.g. `boundless-deployer`) can resolve to a different address than you remember if the local `.stellar` config was replaced by the global config. Always `stellar keys address <alias>` to confirm *which* key you're about to sign with, and keep deployed contract IDs written down (shell vars don't survive a new terminal).

---

## 2. Prerequisites

- `stellar` CLI ≥ 23.x (`stellar --version`). The `.stellar config migrate` / "new release" warnings are harmless noise.
- `jq` and `curl`.
- Each signer has Freighter set up on their own machine in a dedicated browser profile, set to the right network, and has sent you their G-address (see `multisig-guide.md` Part D).

---

## 3. The two transaction shapes

| Shape | Examples | How you build + sign |
|---|---|---|
| **Native account op** | `set-options` (add signer, set thresholds), payment | `stellar tx new <op> --source-account <key> --network <net>` — CLI signs + submits if the source key is local. For multisig: `--build-only`, then sign in Lab, then `stellar tx send`. **No simulate.** |
| **Soroban contract call** | `accept_admin`, `pause`, `set_fee_bps`, `register_supported_token`, `propose_upgrade`, `apply_upgrade`, `migrate` | `stellar contract invoke … --build-only -- <fn>` **then `| stellar tx simulate …`**, then sign in Lab, then `stellar tx send`. **Simulate is mandatory.** |

---

## 4. Full testnet dress rehearsal

This mirrors the mainnet sequence end to end on throwaway testnet contracts. Worked reference values from our drill are shown in `‹comments›`.

### 4.1 Build
```bash
cd boundless-contract
stellar contract build
# → target/wasm32v1-none/release/boundless_{events,profile}.wasm
```

### 4.2 Deploy the profile contract (events depends on it)
```bash
DEPLOYER=$(stellar keys address boundless-deployer)   # confirm WHICH key this is

PROFILE_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/boundless_profile.wasm \
  --source boundless-deployer --network testnet \
  -- \
  --admin "$DEPLOYER" \
  --default_bootstrap_credits 10)
echo "PROFILE_ID=$PROFILE_ID"   # WRITE THIS DOWN  ‹drill: CA63ATN2…›
```
> Constructor arg is **`--default_bootstrap_credits`** (not `--bootstrap_credits`). See Section 7.

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
echo "EVENTS_ID=$EVENTS_ID"   # WRITE THIS DOWN  ‹drill: CDP55GFH…›
```

### 4.4 Wire profile → events (first-set-only)
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
for fn in version get_admin get_fee_bps get_fee_account get_profile_contract is_paused supported_token_count ; do
  echo -n "$fn: "; stellar contract invoke --id "$EVENTS_ID" --source-account "$DEPLOYER" --network testnet --send no -- $fn
done
stellar contract invoke --id "$EVENTS_ID" --source-account "$DEPLOYER" --network testnet --send no \
  -- is_supported_token --token "$USDC"
```
Expect `get_admin` = deployer, `is_supported_token` = `true`, `supported_token_count` = `1`, `is_paused` = `false`.

### 4.7 Build the 2-of-3 multisig
(Signers are: you, your co-founder, and a **cold recovery key** held offline. Thresholds 0/2/2, master disabled.)
```bash
stellar keys generate boundless-multisig-bootstrap --network testnet
BOOT=$(stellar keys address boundless-multisig-bootstrap)   # ‹drill: GDLEW45L…›
curl "https://friendbot.stellar.org/?addr=$BOOT"

# Add the 3 signers (NATIVE op, no simulate) — one tx each
for G in G_YOU... G_COFOUNDER... G_COLD... ; do
  stellar tx new set-options --source-account boundless-multisig-bootstrap \
    --signer "$G" --signer-weight 1 --network testnet
done

# Lock it: 2-of-3 + disable the bootstrap master key (the critical flag)
stellar tx new set-options --source-account boundless-multisig-bootstrap \
  --low-threshold 0 --med-threshold 2 --high-threshold 2 --master-weight 0 \
  --network testnet

./scripts/admin/verify-multisig.sh "$BOOT" testnet   # must print: PASS: all 6 checks passed
```
Read the 3 signer addresses the script prints and confirm them by eye.

### 4.8 Rotate admin: deployer → multisig (the heart of it)
Do this for **both** `$EVENTS_ID` and `$PROFILE_ID`.

```bash
# Step A — current admin (deployer) nominates the multisig. CLI signs + submits.
stellar contract invoke --id "$EVENTS_ID" --source boundless-deployer --network testnet \
  -- set_admin --new_admin "$BOOT"
```
**Wait ~10 seconds** (Rule 7), then:
```bash
# Step B — build + SIMULATE the accept_admin (multisig as source)
stellar contract invoke --id "$EVENTS_ID" --source-account "$BOOT" --network testnet --build-only -- accept_admin \
  | stellar tx simulate --source-account "$BOOT" --network testnet
```
Take that prepared XDR → **Lab sign (you → co-founder)** → submit:
```bash
stellar tx send "<XDR-with-both-signatures>" --network testnet
```
Verify:
```bash
stellar contract invoke --id "$EVENTS_ID" --source-account "$BOOT" --network testnet --send no -- get_admin
# → $BOOT
```
Repeat A→B for `$PROFILE_ID`.

### 4.9 Operate as the multisig (and see the failure mode)
The deployer now has zero admin power. Prove the multisig can run ops:
```bash
# pause (simulate is mandatory)
stellar contract invoke --id "$EVENTS_ID" --source-account "$BOOT" --network testnet --build-only -- pause \
  | stellar tx simulate --source-account "$BOOT" --network testnet
```
→ Lab 2-of-3 → `stellar tx send` → `is_paused` should be `true`. Then `unpause` the same way.

**The "1-of-3 must fail" drill:** build a `pause`, submit with **only one** signature → `tx send` rejects it (`tx_bad_auth`). Seeing this on purpose is the point.

---

## 5. Day-to-day multisig operation (the canonical flow)

Any time the multisig needs to do a contract op (`set_fee_bps`, `pause`, `register_supported_token`, an upgrade step, …):

```bash
# 1. BUILD + SIMULATE  → prepared, unsigned XDR
stellar contract invoke --id <CONTRACT_ID> --source-account "$BOOT" --network <net> --build-only -- <fn> [--arg val ...] \
  | stellar tx simulate --source-account "$BOOT" --network <net>

# 2. SIGN (Lab, in order)
#    signer 1 signs the prepared XDR  → XDR-A
#    signer 2 signs XDR-A             → XDR-B   (2 of 3 — pull the cold key only for set_fee_account)

# 3. SUBMIT (CLI, so errors are visible)
stellar tx send "<XDR-B>" --network <net>

# 4. VERIFY with a read
stellar contract invoke --id <CONTRACT_ID> --source-account "$BOOT" --network <net> --send no -- <getter>
```

`set_fee_account` is the only op the policy keeps at **3-of-3** — and that's a *process* rule (collect all three signatures, including the cold key), not enforced on-chain. See `admin-custody-policy.md` §4.

---

## 6. Troubleshooting (error → cause → fix)

| Symptom | Cause | Fix |
|---|---|---|
| `network passphrase is used but rpc-url is missing` | Mixed `--network <name>` with `--network-passphrase` | Use `--network testnet` alone, or go fully explicit with `--rpc-url … --network-passphrase …` (Rule 5) |
| `TxMalformed` on submit of a contract call | The XDR was `--build-only` and never simulated (no footprint/resource fee) | Pipe through `stellar tx simulate` and sign that output (Rule 1) |
| `tx_bad_auth` / `op_low_threshold` | Not enough signatures combined — usually the 2nd signer signed the original, not the 1st signer's output | Re-sign in order: each signer signs the previous XDR (Rule 3) |
| `Error(Contract, #6)` (`PendingAdminMismatch`) on `accept_admin` | `set_admin` not yet in a closed ledger | Wait a few seconds and re-run the simulate (Rule 7). If still failing, re-run `set_admin` |
| `Error(Contract, #5)` (`NotAdmin`) | The source isn't the current admin (e.g. already rotated to the multisig) | Check `get_admin`; act as the real admin |
| `Error(Contract, #7)` (`PendingAdminExpired`) | The nomination window lapsed | Re-run `set_admin`, then accept promptly |
| `txTooLate` | Time bound elapsed between simulate and submit | Regenerate the prepared XDR and sign quickly (Rule 8) |
| "Account/contract not found" in explorer | Looking at mainnet | Use the `/testnet/` URL (Rule 9) |
| Lab says submitted but admin unchanged / no tx on the account | Submit silently failed in Lab | Submit from the CLI with `stellar tx send` to see the real error (Rule 4) |

**Diagnosis tools (read-only):**
```bash
# Did a tx actually land on an account?
curl -s "https://horizon-testnet.stellar.org/accounts/<G>/transactions?order=desc&limit=5" \
  | jq -r '._embedded.records[] | "\(.created_at) successful=\(.successful) \(.hash)"'
# Multisig config sanity
curl -s "https://horizon-testnet.stellar.org/accounts/<BOOT>" | jq '{signers,thresholds}'
```

---

## 7. Pre-mainnet code fixes (found during the drill)

1. **`deploy_mainnet.sh` deploy-profile uses `--bootstrap_credits`**, but the profile constructor arg is **`--default_bootstrap_credits`**. As-is, the mainnet profile deploy would fail. Fix the flag in the script.
2. **`INITIAL_VERSION` is still `0.2.0`** while the code now includes the supported-token enumeration. A fresh mainnet deploy would stamp `0.2.0` for a contract that differs from the audited `0.2.0` surface. Bump it (e.g. `1.0.0`) and update the upgrade-test fixtures + the runbook's expected-version checks.
3. **Re-audit the supported-token enumeration** — it's new contract code added after the last audit; the mainnet pre-flight gate ("all critical/high resolved") must cover it.

---

## 8. Mainnet deltas

Everything in Section 4–5 is identical on mainnet except:
- Build **without** `--features testnet` (full upgrade timelock). `deploy_mainnet.sh` already does this.
- `--network mainnet` (or explicit `--rpc-url <mainnet> --network-passphrase "Public Global Stellar Network ; September 2015"`).
- The bootstrap is funded with **real XLM** (≥5), not friendbot.
- The cold recovery key actually lives in a safe; the daily signers are you + co-founder.
- After rotation, **destroy the initial deploy key** (`shred -u`); it has no power post-rotation but leave nothing lying around.
- The enumerable token index is complete **from genesis** — register USDC at deploy time and state enumeration is authoritative forever (no import-by-address needed, unlike the in-place-upgraded testnet contract).
