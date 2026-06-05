# Admin key custody policy

**For:** the founder, the multi-sig signers, the contracts engineer.
**Status:** decided 2026-06-03; software-multi-sig launch baseline added 2026-06-05.
**Affects:** all platform governance.

The admin key controls the most sensitive operations on the Boundless contracts: setting fees, swapping the fee account, pausing, upgrading. A single-person admin is unacceptable. This policy defines the multi-sig structure, signer responsibilities, rotation cadence, and emergency procedures.

## Launch baseline vs target state

Two phases. Both are 2-of-3 multi-sig; they differ in how the per-signer keys are stored.

- **Launch baseline (current):** software signers — each signer runs Freighter (or an equivalent Stellar wallet) on their own machine, with a strong unique passphrase and a paper-only backup. This ships now.
- **Target state:** hardware-isolated signers per the original policy text below — Yubikey, Ledger, or air-gapped machine. **Hardware upgrade trigger:** when total escrow TVL crosses the threshold set in section 10 below.

Everything else in this document (thresholds, rotation, drills, operational hygiene) applies to **both phases identically.** The verify-multisig script does not care whether keys are hardware or software; it only checks the on-chain signer config.

Section 3 describes the target-state hardware procedure. Section 3-bis describes the launch-baseline software procedure with the trade-offs called out.

---

## 1. What the admin can do

The admin authority on `boundless-events` (and equivalently on `boundless-profile`) gates:

- `set_fee_bps(new_bps)` — change the global default fee rate.
- `set_fee_account(new_account)` — change the address that receives fee revenue.
- `set_profile_contract(new_addr)` — change which profile contract events writes to.
- `set_admin(new_admin)` — rotate the admin authority (two-step with `accept_admin`).
- `pause()` — emergency stop on all write ops.
- `unpause()` — resume write ops.
- `upgrade(new_wasm_hash)` — replace contract logic.
- `register_supported_token(token)` / `deregister_supported_token(token)` — token whitelist.

There is no other authority. The admin cannot move funds out of escrow directly; the contract enforces that. The admin can only change the rules of future operations and pause current ones.

---

## 2. Multi-sig composition

The admin authority is a **Stellar multi-sig account** with the following starting composition:

| Role | Signer | Notes (target) | Notes (launch baseline) |
|---|---|---|---|
| Founder primary | Collins Ikechukwu | Yubikey-held key, hardware-isolated | Freighter on dedicated browser profile, unique passphrase, paper backup |
| Lead engineer | (assigned) | Yubikey-held key, hardware-isolated | Freighter on a different machine + browser profile |
| Trusted third (advisor or co-founder) | (assigned) | Yubikey-held key | Freighter on a different machine; ideally on a different OS family than the other two |

**Threshold: 2 of 3** for standard operations. Asymmetric thresholds for specific ops are defined in Section 4.

The multi-sig account is a regular Stellar G-address with these signers configured. The contract sees this address as its admin and authorizes any operation that the multi-sig itself authorizes.

### 2.1 Why 2 of 3, not higher

- 1 of 3 (single sig) eliminates the multi-sig protection entirely.
- 2 of 3 prevents any single person from acting unilaterally.
- 3 of 3 means losing any signer locks the admin out (no recoverable state).

We accept the risk of one collusion (two of three signers act maliciously) in exchange for resilience to losing one signer. This is reviewed annually.

### 2.2 Why three, not five

Three is the smallest set that supports 2-of-N. As the team grows, expand to 5 (with 3-of-5) so two signers can be lost or compromised without locking the admin out. Trigger for the expansion: hiring of the fourth + fifth eligible signer.

---

## 3. Key generation and storage (target state — hardware)

Each signer generates their own key. We do not let any single person see another signer's key. The procedure per signer:

1. **Hardware-isolated key generation** on a Yubikey or equivalent (Ledger, dedicated air-gapped machine).
2. **Verify the public address** in two independent contexts (sign a test transaction, read the resulting account on the chain).
3. **Backup the secret** to a sealed envelope held by the signer personally. Recommended: split using Shamir's Secret Sharing with 2-of-3 backup shares stored in physically distinct locations (e.g. bank deposit box + lawyer's safe + signer's home safe).
4. **Confirm to the founder** in writing that the signer has read this policy, generated their key, and stored their backup.

We do not have a "platform-held" backup. Loss of a signer's key without their personal recovery is a real operational risk; that is why the threshold is 2 of 3 (we can lose one and still recover).

## 3-bis. Key generation and storage (launch baseline — Freighter)

Per signer, in isolation. **The threat model here is "compromised machine" — software keys are extractable from any machine that gets owned.** The hygiene below is what makes this safe-enough to ship and is required, not optional.

1. **Dedicated browser profile** on a personal machine (not a shared workstation, not a CI runner). No other browser extensions installed in that profile beyond Freighter. No untrusted tabs while signing.
2. **Generate the keypair in Freighter** with a strong, unique passphrase. Recommended: 6-word [Diceware](https://en.wikipedia.org/wiki/Diceware) or equivalent, never reused for any other purpose. Not your 1Password master, not your email password.
3. **Export the 12-word recovery phrase** and write it on paper. Store the paper in a physically secure location that is not the same machine, not a cloud drive, not a photo. **No digital copy ever.** A second paper copy in a second location is fine (e.g. signer's home safe + bank deposit box). Shamir 2-of-3 split is still recommended if the signer wants belt-and-suspenders.
4. **Verify the public address** in two independent contexts: read it from Freighter directly, and look up the funded account on `stellar.expert` after the first transaction lands.
5. **Confirm to the founder** in writing that the signer has read this policy, generated their key, and stored the paper backup.

### Hygiene that matters

| Practice | Why |
|---|---|
| Different machine per signer | A single supply-chain attack (npm install, browser-extension update) hitting all 3 signers at once kills the multi-sig premise. |
| OS-level full disk encryption | Defensive against laptop theft + cold-boot RAM extraction. macOS FileVault + Linux LUKS + BitLocker all work. |
| No browser extensions besides Freighter in the signing profile | Any extension can read page contents. A malicious one can show a doctored tx while signing. |
| Strong unique passphrase | Encrypts the locally-stored key; an attacker with the file but not the passphrase still can't sign. |
| Paper-only backup | The recovery phrase IS the key. Photographing it, syncing it to iCloud, or pasting it into a notes app all defeat the multi-sig. |
| Sign on a fresh tab, after reading the tx | Freighter shows the tx hash + operation. Read it. Don't autopilot. |

### Anti-patterns (do not do, even temporarily)

- Sharing a Freighter install across two signers.
- Backing up the recovery phrase in 1Password, iCloud, Google Drive, or any cloud notes app.
- Generating keys on a shared dev workstation, even briefly.
- Signing from a phone or any device without full disk encryption.

We do not have a "platform-held" backup. Loss of a signer's key without their personal recovery is a real operational risk; that is why the threshold is 2 of 3 (we can lose one and still recover).

---

## 4. Operation-specific thresholds

The standard threshold is 2 of 3. The following operations have different requirements:

| Operation | Threshold | Reasoning |
|---|---|---|
| `pause()` | **2 of 3 (lower bar)** | Emergency. Cap damage first; recover later. |
| `unpause()` | 2 of 3 | Resume after fix is in place. |
| `upgrade(new_wasm_hash)` | 2 of 3 | Standard. Requires both pre-deploy review + audit refresh. |
| `set_fee_bps(new_bps)` | 2 of 3 | Standard. |
| `set_fee_account(new_account)` | **3 of 3 (higher bar)** | Most-sensitive op. Wrong address sends fees somewhere we cannot retrieve. |
| `set_admin(new_admin)` | **3 of 3** | Rotating the admin itself. Highest-stakes change. |
| `set_profile_contract` | 2 of 3 | Standard. |
| `register_supported_token` / `deregister_supported_token` | 2 of 3 | Routine policy. |

The Stellar multi-sig itself does not support per-operation thresholds directly; we enforce these by signer convention with logged sign-off. Tools we build (admin signing portal) will enforce them programmatically.

---

## 5. Signer rotation

### 5.1 Planned rotation (signer leaves the team)

1. Outgoing signer notifies the founder in writing.
2. Successor is identified and onboarded per Section 3.
3. New multi-sig account is provisioned with the new signer set.
4. Quorum on the current multi-sig executes `set_admin(new_multisig_address)`.
5. New multi-sig accepts via `accept_admin`.
6. Verify on-chain via `get_admin`.
7. Outgoing signer destroys their key copy.
8. Document the rotation in `deployments/admin-rotations.jsonl`.

### 5.2 Emergency rotation (signer key compromised or lost)

1. Surviving quorum (2 of 3 with the compromised signer removed) executes `pause()` immediately as a safety measure.
2. Provision a new signer's key per Section 3.
3. Build a new multi-sig account with the updated signer set.
4. Quorum executes `set_admin(new_multisig_address)`.
5. Once accepted, `unpause()`.
6. Forensics on the compromise.

### 5.3 Drill

Quarterly rotation drill on testnet using practice keys. Validates the procedure works without touching mainnet.

---

## 6. Lost-signer scenarios

### 6.1 One signer permanently lost (key gone, signer unreachable)

With 2 of 3, the remaining two can still operate. Rotate per Section 5.1.

### 6.2 Two signers lost simultaneously

This is the catastrophic case. We cannot meet quorum.

- The contract continues to function for non-admin operations (settle, claim, refund all work).
- We cannot pause, upgrade, or change fees.
- Fee rates stay at whatever they were last set to.
- Long-term mitigation: at scale, expand to 3 of 5 to make this less likely.

There is no escape hatch by design. A backdoor that recovered admin from below-threshold loss would defeat the multi-sig purpose. The contract is intentionally designed to keep working for end users even if we lose admin control.

### 6.3 Three signers lost simultaneously

Same as 6.2, plus we cannot rotate. The contract is admin-frozen at its current state but functionally operational.

---

## 7. Operational hygiene

For every admin operation:

- [ ] Written request in `#ops-admin-requests` Slack channel with the proposed operation and reasoning.
- [ ] Each signer confirms review in the same channel before signing.
- [ ] Signers coordinate on a Soroban CLI invocation; or use an admin signing portal (when built).
- [ ] On-chain operation is verified by the requester before the channel thread is closed.
- [ ] Operation logged in `deployments/admin-operations.jsonl`.

This is not paranoia; it is a paper trail. Every fee change, every upgrade, every signer rotation has an auditable record.

---

## 8. Emergency contact + escalation

| Scenario | First call | Second call |
|---|---|---|
| Key compromise suspected | Founder | Lead engineer |
| Multi-sig coordination needed during outage | Founder | Trusted third signer |
| Founder unreachable | Lead engineer | Trusted third signer |

Maintain a written escalation list in 1Password (Boundless Ops vault) with phone numbers + alternates.

---

## 9. Open follow-ups

- Build an admin signing portal that enforces operation-specific thresholds programmatically.
- Quarterly signer rotation drill on testnet.
- At scale: expand to 3-of-5 and add two additional signers.
- Insurance: explore key-loss insurance once we have a track record of clean operations.
- Annual review of this policy on each deploy anniversary.

---

## 10. Hardware-upgrade trigger

Software multi-sig is the launch baseline (sections 2 and 3-bis). The upgrade to hardware-isolated signers happens when **any** of the following fires:

| Trigger | Threshold |
|---|---|
| Total live escrow TVL across all pillars | **$250,000 USDC-equivalent** at any single point in time, or |
| Sustained daily settlement volume | **$50,000 USDC-equivalent / day for 7 consecutive days**, or |
| First confirmed security incident affecting any signer machine | immediate (regardless of TVL) |

When the trigger fires, the upgrade procedure is:

1. Procure 3 Yubikeys (or equivalent hardware-isolated devices).
2. Each signer re-runs section 3 (hardware path) on their device.
3. Create a NEW multi-sig account with the three NEW hardware-backed addresses; verify with `./scripts/admin/verify-multisig.sh <new-multisig> mainnet`.
4. Run the rotation per section 5.1 (`set_admin(new_multisig) → accept_admin`).
5. Destroy the software keys on each signer machine. Burn the paper backups.
6. Run the testnet drill on the new hardware multi-sig per `docs/multisig-preflight.md` §4.
7. Log the rotation in `deployments/admin-rotations.jsonl` with both old and new multi-sig addresses + the threshold that triggered.

The thresholds above are starting points. Review them at the same cadence as the annual policy review, or sooner if the team's risk tolerance changes.
