# Multi-sig provisioning pre-flight

**For:** the founder + the multi-sig signers.
**Gate for:** the mainnet admin rotation in `mainnet-deploy-runbook.md` §2.7.

Work this checklist top-to-bottom **before** running `set_admin` on a mainnet contract. Every item is either done by a human signer out of band or verified by `./scripts/admin/verify-multisig.sh`.

The policy this enforces lives in `admin-custody-policy.md`. If the two ever disagree, the policy wins; update this checklist to match.

---

## 0. Pre-conditions (both paths)

- [ ] `admin-custody-policy.md` re-read by the founder + every signer in the past 30 days.
- [ ] Three signers identified by name + role (founder primary, lead engineer, trusted third) and recorded in the founder's notes.
- [ ] **Choose a path:**
  - **Hardware path (target state):** Three Yubikeys (or equivalent hardware-isolated devices) procured, one per signer. Use Section 1.A below.
  - **Software path (launch baseline):** Three different machines, three different signers, three Freighter installs. Use Section 1.B below. Requires acknowledging the trade-offs in `admin-custody-policy.md` §3-bis.
- [ ] Each signer has Stellar CLI 23.x or newer installed and (for the hardware path) a hardware-signing tool that speaks Stellar (e.g. `stellar` CLI's Ledger integration, the Stellar Lab hardware-wallet tab, or LOBSTR Vault).
- [ ] Founder has personally verified each signer is reachable on a backup channel that is NOT the same one the original signer enrollment used.

## 1.A Key generation — hardware path (target state)

For each of the three signers, the signer performs the following on their own machine, with the Yubikey/hardware device present:

- [ ] **Generate** a new ed25519 keypair on the Yubikey (the secret never leaves the device).
- [ ] **Read** the resulting public address back from the device twice in two distinct ways (CLI + GUI tool) and confirm they match.
- [ ] **Sign** a Stellar tx on testnet using only the Yubikey to prove the workflow works.
- [ ] **Backup**: 2-of-3 Shamir's Secret Sharing split, shares placed in three physically distinct locations (per policy §3).
- [ ] **Write** the public address to the founder's collection channel, signed via the founder's PGP key or equivalent out-of-band auth.

Outcome: the founder has three new G-addresses, each tied to a hardware-isolated secret that only the matching signer can reach.

## 1.B Key generation — software path (launch baseline)

For each of the three signers, in isolation. Read `admin-custody-policy.md` §3-bis first; the hygiene requirements there are NOT optional.

- [ ] **Dedicated browser profile** created (Chrome / Firefox / Brave — pick one, dedicate it). No other browser extensions installed in that profile.
- [ ] **Freighter installed** in that profile; no other Stellar wallet active.
- [ ] **OS-level full disk encryption verified active** (FileVault / BitLocker / LUKS).
- [ ] **Generate a new keypair** in Freighter with a strong, unique passphrase (6-word Diceware or stronger; not reused for any other purpose).
- [ ] **Export the 12-word recovery phrase**, write it on paper, store in a physically secure location. No digital copy. **No photos. No notes app. No cloud drive.** A second paper copy in a second location is fine.
- [ ] **Read the public address** in two independent contexts: Freighter UI directly, and from `stellar.expert` after sending a small test transaction on testnet.
- [ ] **Sign** a test transaction on testnet from Freighter to prove the workflow works end to end.
- [ ] **Write** the public address to the founder's collection channel; ideally signed via the founder's PGP key or equivalent out-of-band auth.

Outcome: the founder has three new G-addresses, each tied to a software-stored secret protected by a unique passphrase + paper-only recovery, and the signer machines are minimally-attackable surfaces.

A note on the math: software signers' threat model is "compromised machine," so the value of the 2-of-3 quorum scales with how independent the three machines are. If two signers use the same dev workstation, the multi-sig is effectively single-sig for any attacker who owns that machine. Different people, different machines, different OS families if you can swing it.

## 2. Multi-sig account creation (both paths)

The founder (or an operations engineer trusted with this single workflow) does the following on a clean machine:

- [ ] Create a new Stellar account on mainnet (fund the minimum reserve from the founder's treasury).
- [ ] Add the three signer G-addresses with weight 1 each (`stellar tx set-options --signer ... --signer-weight 1`).
- [ ] Set the low / medium / high thresholds to 0 / 2 / 2 respectively.
- [ ] Set master key weight to 0 (this is the critical "disable the seed alone" step). **If the founder skips this on the software path, an attacker who steals the master seed can sign solo — the whole multi-sig is defeated.**
- [ ] Note the multi-sig G-address; this is what the contract will recognize as admin.

## 3. Verify

```bash
./scripts/admin/verify-multisig.sh <MULTISIG_G_ADDRESS> mainnet
```

The script must return all checks passed. It enforces:

- master key weight = 0
- low threshold = 0
- medium threshold = 2
- high threshold = 2
- exactly 3 non-zero-weight signers
- each signer weight = 1

**If any check fails, do not rotate admin authority.** Re-provision per the failing check.

After the script passes, manually compare the printed signer addresses against the roster the founder collected in §1.

## 4. Testnet drill

Before touching mainnet:

- [ ] Deploy a throwaway boundless-events contract on testnet with the SAME multi-sig as admin (use practice Yubikeys or practice Freighter installs, whichever path you're on).
- [ ] Exercise a 2-of-3 sign on a routine op (e.g., `set_fee_bps`).
- [ ] Exercise a 3-of-3 sign on a high-bar op (`set_fee_account` — see policy §4).
- [ ] Confirm that 1-of-3 signing on either op fails.
- [ ] Document the drill in the team's shared notes with the tx hashes.

The same procedure is required quarterly per policy §5.3; this just exercises it under realistic conditions before mainnet.

## 5. Rotate admin authority

Only after every box above is checked:

```bash
# From the deploy runbook §2.7
soroban contract invoke \
    --network mainnet \
    --source $INITIAL_ADMIN_KEY \
    --id $EVENTS_ID \
    -- set_admin --new_admin $MULTISIG_G_ADDRESS

# 2-of-3 quorum accepts. The multi-sig signs as event.owner.
soroban contract invoke \
    --network mainnet \
    --source $MULTISIG_G_ADDRESS \
    --id $EVENTS_ID \
    -- accept_admin
```

- [ ] `verify-multisig.sh` re-run **after** the rotation lands as a sanity check.
- [ ] `get_admin` on both contracts returns the multi-sig G-address.
- [ ] Initial admin key destroyed per runbook §2.8.

## 6. Update the BACKLOG

Once mainnet shows the multi-sig as admin and the drill succeeded:

- Replace the P0 line `Mainnet admin multi-sig provisioned per docs/admin-custody-policy.md (3 signers, Yubikey-backed, 2-of-3).` with the equivalent `[x] …` entry under `Done` in `BACKLOG.md`, citing the rotation tx hash + the verify-multisig output.

---

## Anti-patterns (do not do)

- Sharing a single Yubikey or a single Freighter install between two signers. The whole point is independent custody.
- Backing up a Freighter recovery phrase to 1Password, iCloud, Google Drive, or any cloud notes app. The recovery phrase IS the key.
- Generating a software signing keypair on a shared dev workstation. Use a dedicated browser profile on a personal machine.
- Skipping the testnet drill because "we already practiced." The drill is the unit test for the entire human procedure.
- Running `set_admin` from a machine that does general-purpose browsing in parallel. Close everything else; sign in a clean session.
- Treating "we'll upgrade to hardware later" as optional. The hardware-upgrade trigger in `admin-custody-policy.md` §10 is a commitment, not a hope.
