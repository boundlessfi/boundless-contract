# Admin key custody policy

**For:** the founder, the multi-sig signers, the contracts engineer.
**Status:** decided 2026-06-03.
**Affects:** all platform governance.

The admin key controls the most sensitive operations on the Boundless contracts: setting fees, swapping the fee account, pausing, upgrading. A single-person admin is unacceptable. This policy defines the multi-sig structure, signer responsibilities, rotation cadence, and emergency procedures.

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

| Role | Signer | Notes |
|---|---|---|
| Founder primary | Collins Ikechukwu | Yubikey-held key, hardware-isolated |
| Lead engineer | (assigned) | Yubikey-held key, hardware-isolated |
| Trusted third (advisor or co-founder) | (assigned) | Yubikey-held key |

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

## 3. Key generation and storage

Each signer generates their own key. We do not let any single person see another signer's key. The procedure per signer:

1. **Hardware-isolated key generation** on a Yubikey or equivalent (Ledger, dedicated air-gapped machine).
2. **Verify the public address** in two independent contexts (sign a test transaction, read the resulting account on the chain).
3. **Backup the secret** to a sealed envelope held by the signer personally. Recommended: split using Shamir's Secret Sharing with 2-of-3 backup shares stored in physically distinct locations (e.g. bank deposit box + lawyer's safe + signer's home safe).
4. **Confirm to the founder** in writing that the signer has read this policy, generated their key, and stored their backup.

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
