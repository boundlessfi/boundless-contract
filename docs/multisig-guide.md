# The Boundless multi-sig guide (plain English)

This guide is for the three signers and anyone at Boundless who needs to understand how the admin multi-sig works. It assumes no prior Stellar or crypto knowledge.

If you read only one thing, read **Part A: The 60-second version** below.

Where this fits in:

- **If you are one of the three signers:** read Parts A, B, C, D, F.1 to F.4, H, I, J, K, L. You can skim Parts E and F.0 to understand what the founder is doing on the other side.
- **If you are a Boundless employee who is not a signer:** read Parts A, B, F (skim F.0), G, I, J, K, L. Knowing what the admin can and cannot do (Part I) is what keeps you from being phished.
- **If you are the founder running setup:** read everything in this guide, then also read `docs/admin-custody-policy.md` (the formal policy) and `docs/multisig-preflight.md` (the engineer-facing checklist).

---

## Part A. The 60-second version

Three people each hold a different key. Doing anything sensitive on the Boundless contract (changing fees, pausing the app, upgrading the code) needs two of those three people to sign. No one person can do it alone.

Think of it like a bank safe with three locks. The bank manager has one key. The vice manager has one key. The branch auditor has one key. You need any two of those three to open the safe.

The "keys" here are crypto wallet keys stored in Freighter (a browser extension). The "bank safe" is a Stellar account that owns the Boundless admin authority. The "opening the safe" is signing a transaction.

---

## Part B. The exact pieces, named

| Name | What it actually is |
|---|---|
| **The multi-sig account** | A Stellar account (a G-address) that the contract treats as admin. It has no individual owner. It only acts when two of three signers agree. |
| **Signer 1 / 2 / 3** | Three people. Each has their own Freighter wallet on their own computer. Each holds one key. |
| **Threshold** | The number of signatures needed to do something. Boundless uses 2-of-3 for normal stuff and 3-of-3 for the most dangerous stuff (changing the fee account, swapping admin). |
| **Freighter** | A free browser extension that holds your Stellar wallet keys. Available at [freighter.app](https://freighter.app). |
| **Stellar Lab** | A free website at [lab.stellar.org](https://lab.stellar.org) that lets you build, sign, and submit transactions. We use it to sign multi-sig transactions because Freighter can connect to it. |
| **Recovery phrase** | 12 random words Freighter shows you once when you first set it up. Whoever has these words can become you. Treat them like a winning lottery ticket. |
| **Passphrase** | A password you pick when you set up Freighter. It locks Freighter on your machine. If someone steals your laptop, the passphrase is what stops them from opening Freighter. |
| **Testnet** | The Stellar practice network. Free fake money. Nothing on testnet is real. This is where we practice. |
| **Mainnet** | The Stellar production network. Real money. This is where we operate Boundless for real customers. |

---

## Part C. Before you start (signer pre-flight)

If you are about to set up as a signer, walk through this checklist before you touch Freighter. The whole setup takes about 30 minutes if you have everything ready.

You need:

- [ ] A personal computer (not a shared work machine, not a friend's laptop). Mac, Linux, or Windows is fine. The disk needs to be encrypted: FileVault on Mac, BitLocker on Windows, LUKS on Linux. If you do not know if disk encryption is on, look it up before continuing.
- [ ] A web browser you actually use. Chrome, Brave, or Firefox is fine. Edge works too.
- [ ] About 30 minutes of uninterrupted time, ideally with the founder available on a call.
- [ ] A piece of paper and a pen. Real paper. Real pen. Not a notes app.
- [ ] A safe place to keep that piece of paper for the next many years. A home safe, a bank deposit box, or a sealed envelope in a locked drawer all work. Not your sock drawer. Not "I'll figure it out later."
- [ ] Your phone, to verify you can reach the founder on a backup channel if anything goes weird.

If any of those is missing, stop and fix it first. Do not skip ahead.

---

## Part D. Part 1: Set up Freighter (signer task)

Estimated time: 10 minutes.

### D.1. Make a separate browser profile

This is the most-skipped step and it matters. A browser profile is a fresh copy of your browser with no extensions, no history, no logins. We want signing to happen in a clean profile so nothing else can interfere.

In Chrome or Brave:

1. Click your profile picture in the top right.
2. Click "Add" or "Add Profile."
3. Name it "Boundless Signer" or similar.
4. A new browser window will open with that profile.

In Firefox:

1. Type `about:profiles` in the address bar and press Enter.
2. Click "Create a New Profile."
3. Name it "Boundless Signer."
4. Launch it.

From now on, all the steps in this guide happen in that profile. Do not install other extensions in it. Do not log into your personal Google / iCloud / work email here. This profile is only for signing.

### D.2. Install Freighter

1. In your new Boundless Signer profile, go to [freighter.app](https://freighter.app).
2. Click "Add to Chrome" (or Firefox, etc.).
3. Approve the installation.
4. Pin Freighter to your toolbar so you can find it: click the puzzle icon, then the pin next to "Freighter."

### D.3. Create your wallet

1. Click the Freighter icon. It will say "Welcome to Freighter."
2. Click "Create wallet."
3. Pick a strong passphrase. Use six random English words like `correct horse battery staple ocean ladder`. Do not reuse your email password, your 1Password master password, or anything else you use elsewhere. Write it down on paper too (separate from your recovery phrase).
4. Freighter will show you 12 words. **This is your recovery phrase.** Write each word on paper, in order. Number them 1 through 12. Double-check spelling.
5. Freighter will ask you to confirm a few of those words to make sure you wrote them correctly. Do it.

### D.4. Save your recovery phrase the right way

Your recovery phrase is more important than your passphrase. With your phrase, anyone can become you on any computer. Without it, you cannot recover your wallet if your laptop dies.

**Do:**

- Write it on paper. Pen, not pencil. Block capitals so the letters are clear.
- Make two copies. Keep them in two different places. For example: one in a home safe, one at your bank's safe deposit box. Or one at home, one given (sealed) to your lawyer.
- Tell one trusted person (probably the founder) that the backup exists, where it is, and how to retrieve it if you cannot.

**Do not:**

- Take a photo of it.
- Type it into Notes, Apple Notes, Google Keep, Notion, Slack, or any other app.
- Email it to yourself.
- Put it in 1Password, LastPass, iCloud Keychain, or any password manager.
- Store it in any cloud drive (iCloud, Google Drive, Dropbox).
- Put it on a USB stick that is also used for anything else.

This sounds paranoid. It is the part everyone gets wrong. The recovery phrase is the wallet. A photo in your Photos app means anyone who can see your Photos app can drain the wallet.

### D.5. Switch Freighter to testnet

We are practicing on testnet first.

1. Click the Freighter icon.
2. Click the gear icon (Settings) in the top right.
3. Click "Preferences" or "Network."
4. Switch network to "Test Net."
5. Your wallet now shows a different balance. That is expected. Your testnet wallet is empty.

### D.6. Get free testnet money

You need a small amount of testnet XLM (about 1 XLM is enough) to demonstrate signing. Stellar gives this away for free for testing.

1. Copy your wallet's public address. In Freighter, it looks like `GDSBURJQPMB7HW7TYN3AL2RSISUHIJIWBWSEM2UQFZJFAP7FX2SU2A4K`. Click the address to copy it.
2. Open a new tab and go to `https://friendbot.stellar.org/?addr=YOUR_ADDRESS` where you paste your address after the `=`.
3. Press Enter. It will load for a moment, then return some JSON. That is success.
4. Go back to Freighter. You should now see 10,000 XLM (testnet only, not real).

### D.7. Send your public address to the founder

1. In Freighter, copy your public address. It starts with G and has lots of letters and numbers.
2. Send it to the founder via the agreed channel (probably Signal or a written delivery). Include both:
   - Your name
   - Your public address (paste it carefully; one wrong letter and the multi-sig will not include you)
3. The founder will confirm receipt and read the address back to you to double-check.

That is the end of your setup. You now have a wallet on a clean browser profile, your recovery phrase is on paper in a safe place, and the founder has your public address.

---

## Part E. Part 2: The founder builds the multi-sig

This part is for the founder (or whoever has been designated to run the operations workflow). The signers do not do anything here; they wait for the founder to confirm the multi-sig is set up.

Estimated time: 45 minutes on testnet, plus a 1-day cooldown before repeating on mainnet so any mistakes surface before real money is on the line.

### E.0. Where the multi-sig fits in the deploy lifecycle (read this first)

A common point of confusion: **the multi-sig is NOT the same account that deploys the contract.** They are two different Stellar accounts with different lifetimes and different security models.

There are two distinct accounts in play:

| Account | Lifetime | Role | Signature model |
|---|---|---|---|
| **Deployer (initial-admin key)** | Short. Created right before deploy, destroyed right after rotation. | Pays the deploy fee. Runs `stellar contract deploy`. Becomes admin via the contract's constructor. Then hands admin off to the multi-sig and dies. | Single-sig (just one normal Stellar key). |
| **Multi-sig (the bootstrap account in §E.2 below)** | Long. Lives as long as Boundless does. | Never deploys anything. Becomes admin AFTER the deploy, via `set_admin` + `accept_admin`. Holds admin authority from then on. | 2-of-3 (after §E.5). |

The handoff sequence, from `docs/mainnet-deploy-runbook.md` §2.7:

```bash
# 1. Founder deploys with the throwaway single-sig deployer key.
stellar contract deploy --source-account boundless-deployer ...

# 2. Founder rotates admin from deployer to multi-sig.
stellar contract invoke \
  --source-account boundless-deployer \
  --id <EVENTS_CONTRACT_ID> \
  -- set_admin --new_admin <MULTISIG_G_ADDRESS>

# 3. Multi-sig (2-of-3) accepts admin authority.
stellar contract invoke \
  --source-account <MULTISIG_G_ADDRESS> \
  --id <EVENTS_CONTRACT_ID> \
  -- accept_admin

# 4. Founder destroys the deployer key. It is no longer needed.
stellar keys rm boundless-deployer
```

After step 4, the only thing that can change contract admin settings is the multi-sig. The deployer key does not exist. There is no path back through the old key, and that is the point.

Why this separation matters:

- **Deploying with a single-sig key is fast.** Push the wasm, run the constructor, done. Coordinating three signers just to push contract code would be painful and would give an attacker more windows to interpose.
- **The deployer is disposable.** If the deployer key leaks before §E.0 step 2, the damage is bounded to a throwaway account that holds no funds. The multi-sig stays clean.
- **The multi-sig is permanent.** It outlives the deployer. It also outlives any individual signer (Part J covers rotation). The deployer dies young; the multi-sig is built to last.

So when §E.2 below tells you to create a bootstrap account, that account is destined to become the multi-sig in §E.5. It is not the account you will use to run `stellar contract deploy`. Keep them separate.

### E.0.B. The complete end-to-end sequence (founder cheat sheet)

This is the full timeline from a clean slate to a fully-rotated mainnet deploy. Twelve steps. Read it once before you start so you have the whole map, then come back and walk through it in order.

Steps that are detailed elsewhere in this guide are cross-linked. Steps that come from other docs (`mainnet-deploy-runbook.md`, `multisig-preflight.md`) are flagged.

#### Step 1. Configure .env.deploy

The deploy scripts read parameters from `.env.deploy` at the repo root. Copy the template and fill in the values.

```bash
cd boundless-contract
cp .env.deploy.example .env.deploy
```

Open `.env.deploy` in your editor and set:

| Variable | What it is |
|---|---|
| `ADMIN_IDENTITY` | The name of the Stellar CLI identity that will deploy and hold initial admin authority. Default is `boundless-admin`. This is the throwaway deployer per §E.0, NOT the final admin. Rename to `boundless-deployer` if the default name reads as misleading; the scripts only care about the value. |
| `FEE_ACCOUNT` | The G-address that receives platform fees. **Separate account.** Not the deployer, not the multi-sig. Must hold a trustline for every token you register; the contract does not enforce this at registration. |
| `FEE_BPS` | Platform fee in basis points (`100 = 1%`, `250 = 2.5%`). The `.env.deploy.example` template and `deploy.sh` validate the range as `[0, 1000]`, matching the contract's `MAX_FEE_BPS = 1000` (10%) cap per the 2026-06 audit (L4 finding). Values above 1000 are rejected at the script level before any deploy work happens. |
| `BOOTSTRAP_CREDITS` | Starting credit balance assigned to newly-bootstrapped profiles. Default is 10 per the credits / reputation PRD. |

`.env.deploy` is gitignored. Never commit a populated file.

#### Step 2. Create the deployer Stellar identity

The deployer is the throwaway single-sig key per §E.0. Generate it under the name you set as `ADMIN_IDENTITY` in `.env.deploy`.

```bash
stellar keys generate boundless-deployer --network mainnet
stellar keys address boundless-deployer       # write this down
```

(Use `--network testnet` and the testnet name `boundless-admin` if rehearsing on testnet.)

The secret stays in the OS keychain. You will not see it.

#### Step 3. Fund the deployer

Send about 10 XLM to the deployer address from your treasury. That budget covers:

- Account reserve (currently 1 XLM).
- Both contract uploads + deploys (Soroban resource fees: typically <1 XLM each).
- One `set_events_contract` wiring call.
- One `register_supported_token` per token.
- Two `set_admin` rotation calls in Steps 9 + 10.
- A small headroom.

Verify funding:

```bash
curl -s "https://horizon.stellar.org/accounts/$(stellar keys address boundless-deployer)" | jq '.balances'
```

On testnet, friendbot funds it for free:

```bash
curl "https://friendbot.stellar.org/?addr=$(stellar keys address boundless-admin)"
```

#### Step 4. Run the deploy script

The script `scripts/deploy/deploy.sh` does the whole deploy in one shot: builds both contract wasms via `stellar contract build`, deploys `boundless-profile`, deploys `boundless-events` wired to profile, calls `profile.set_events_contract` to complete the back-wiring, and writes a deployment record at `deployments/<network>.json`.

```bash
./scripts/deploy/deploy.sh mainnet
```

The script:

- Aborts if `stellar` CLI is older than 26.0.0 (Rust 1.90.0 emits reference-types wasm that earlier CLIs reject at simulation time).
- Aborts if any of `ADMIN_IDENTITY`, `FEE_ACCOUNT`, `FEE_BPS`, `BOOTSTRAP_CREDITS` is unset.
- Aborts if `FEE_BPS` is outside `[0, 1000]` (matches the contract's `MAX_FEE_BPS = 1000` cap per audit L4; see Step 1).
- Prints the profile contract id and events contract id on success.

Copy both ids into your team notes and into the boundless-nestjs deployment env:

```
BOUNDLESS_EVENTS_CONTRACT_ADDRESS=<events id>
BOUNDLESS_PROFILE_CONTRACT_ADDRESS=<profile id>
```

Under the hood, the constructor invocations are:

- `boundless-profile`: `--admin <DEPLOYER_ADDR> --default_bootstrap_credits <BOOTSTRAP_CREDITS>`.
- `boundless-events`: `--admin <DEPLOYER_ADDR> --fee_account <FEE_ACCOUNT> --fee_bps <FEE_BPS> --profile_contract <PROFILE_ID>`.
- Wiring call: `profile.set_events_contract --new_addr <EVENTS_ID>`.

You generally do not need to invoke these by hand. The script is the source of truth; if it breaks, fix the script, do not work around it.

#### Step 5. Register each supported token

```bash
./scripts/deploy/register_token.sh mainnet <USDC_MAINNET_SAC>
```

The script:

- Reads the events contract id and fee account from `deployments/mainnet.json` (so Step 4 must have succeeded).
- Prompts: `fee account holds an active trustline for this token? [y/N]`. **Do not skip this prompt.** The contract does NOT enforce the trustline at registration; missing it means every deposit will fail at runtime.
- On confirmation, calls `register_supported_token --token <TOKEN>` from the deployer.
- Appends the token to `supported_tokens` in the deployment record.

To verify the trustline before answering `y`:

```bash
stellar account get --account "$FEE_ACCOUNT" --network mainnet | jq '.balances'
```

The asset code + issuer must appear in `balances[]`. If they do not, build a Change Trust operation on the fee account in Stellar Lab first, then re-run the script.

Repeat for each token you support (USDC mandatory, native XLM SAC if you accept XLM).

#### Step 6. Verify on-chain state

```bash
./scripts/deploy/verify.sh mainnet
```

The script reads `deployments/mainnet.json` and queries each contract for its full admin-visible state:

- **Events:** `get_admin`, `get_fee_account`, `get_fee_bps`, `get_profile_contract`, `is_paused`.
- **Profile:** `get_admin`, `get_events_contract`, `get_default_bootstrap_credits`, `is_paused`.

Expected output at this point:

- Both `admin` fields return the **deployer address** (rotation has not happened yet).
- `events.fee_account` matches `.env.deploy`.
- `events.fee_bps` matches `.env.deploy`.
- `events.profile_contract` matches the profile id.
- `profile.events_contract` matches the events id.
- `profile.default_bootstrap_credits` matches `.env.deploy`.
- Both `is_paused` return `false`.

If anything is wrong, fix it now. The deployer still has single-sig admin authority through Step 8; from Step 9 onward every fix needs two signatures.

#### Step 7. (Optional) Smoke from boundless-nestjs

For extra confidence before the rotation, run the contract smoke battery from boundless-nestjs against the live mainnet deploy:

```bash
cd ../boundless-nestjs
BOUNDLESS_EVENTS_CONTRACT_ADDRESS=<events id> \
BOUNDLESS_PROFILE_CONTRACT_ADDRESS=<profile id> \
STELLAR_NETWORK_PASSPHRASE="Public Global Stellar Network ; September 2015" \
SMOKE_OWNER_ADDRESS=$(stellar keys address boundless-deployer) \
SMOKE_TOKEN=<USDC_MAINNET_SAC> \
npx ts-node -r tsconfig-paths/register scripts/smoke/escrow-contract.ts
```

The read-path + build-path + drift checks should all pass. Any failure here means the off-chain orchestrator does not agree with the freshly-deployed contract; do NOT rotate admin until that is resolved.

#### Step 8. Build the multi-sig (Parts E.1 through E.8)

This is the middle of the timeline. Walk through:

- §E.1: Decide the three signers (founder + internal employee + external trusted person).
- Have each signer run Part D to set up Freighter and send you their G-address.
- §E.2: Generate the bootstrap account (`boundless-multisig-bootstrap`).
- §E.3: Fund it with ~5 XLM.
- §E.4: Add the three signer addresses (one `set-options` tx each).
- §E.5: Set thresholds 0/2/2 AND `--master-weight 0` in a single tx.
- §E.6: Run `verify-multisig.sh`. Must print `PASS: all 6 checks passed`.
- §E.7: Tell the signers it is ready.
- §E.8: 1-day cooldown on mainnet so any mistake surfaces before the rotation.

At the end of Step 8 you have a working multi-sig that is NOT yet recognized by the contract.

#### Step 9. Rotate admin on the events contract

This is the moment the multi-sig becomes real. Two transactions:

```bash
# 9.a. From the deployer (single-sig): hand admin to the multi-sig.
EVENTS_ID=$(jq -r '.events_contract' deployments/mainnet.json)
stellar contract invoke \
  --source-account boundless-deployer \
  --id "$EVENTS_ID" \
  --network mainnet \
  -- \
  set_admin \
  --new_admin <MULTISIG_G_ADDRESS>

# 9.b. From the multi-sig (needs 2-of-3 signatures): accept admin.
# Build per §F.0.B, get the Lab signing link, collect two signatures,
# then submit per §F.0.C.
stellar contract invoke \
  --source-account <MULTISIG_G_ADDRESS> \
  --id "$EVENTS_ID" \
  --network mainnet \
  --build-only \
  -- \
  accept_admin
```

Step 9.b is the first time the multi-sig signs anything on mainnet. Treat it like a drill: clear comms in `#ops-admin-requests`, both signers verify the Lab tx matches `accept_admin` against the events contract id, two signatures collected, then submit.

**Verification:** `./scripts/deploy/verify.sh mainnet` now shows `events.admin` = multi-sig address (profile.admin still = deployer until Step 10).

#### Step 10. Rotate admin on the profile contract

Repeat Step 9 against the profile contract.

```bash
PROFILE_ID=$(jq -r '.profile_contract' deployments/mainnet.json)
stellar contract invoke \
  --source-account boundless-deployer \
  --id "$PROFILE_ID" \
  --network mainnet \
  -- \
  set_admin \
  --new_admin <MULTISIG_G_ADDRESS>

# Then multi-sig accept_admin per §F.0.B.
stellar contract invoke \
  --source-account <MULTISIG_G_ADDRESS> \
  --id "$PROFILE_ID" \
  --network mainnet \
  --build-only \
  -- \
  accept_admin
```

**Verification:** `./scripts/deploy/verify.sh mainnet` now shows BOTH contracts' `admin` = multi-sig address.

#### Step 11. Final verification + proof-of-life

Re-run both verification scripts. Both must pass cleanly.

```bash
./scripts/admin/verify-multisig.sh <MULTISIG_G_ADDRESS> mainnet
./scripts/deploy/verify.sh mainnet
```

- `verify-multisig.sh` must still print `PASS: all 6 checks passed`. Rotation should not have touched the multi-sig itself.
- `verify.sh` must show both `events.admin` and `profile.admin` as the multi-sig address.

Then do one real admin op end to end to prove the multi-sig signing path works in production. The lowest-risk option is a no-op `set_fee_bps --new_bps <current value>` (e.g., re-set to whatever `events.fee_bps` already reads). It changes nothing but exercises the full multi-sig build + sign + submit flow against the live contract.

Record in team notes:

- The events `set_admin` tx hash and `accept_admin` tx hash.
- The profile `set_admin` tx hash and `accept_admin` tx hash.
- The proof-of-life multi-sig tx hash.
- The output of `verify-multisig.sh`.
- The output of `verify.sh`.

This is your evidence the rotation happened cleanly.

#### Step 12. Destroy the deployer + close out BACKLOG

Once Step 11 is fully green:

```bash
stellar keys rm boundless-deployer
```

The deployer secret is gone from your OS keychain. The remaining XLM balance on the deployer account is stranded (you can sweep it to the treasury before this step if you want; the gas headroom is small enough that abandoning it is also fine).

Update `BACKLOG.md`:

- Move `Mainnet admin multi-sig provisioned per docs/admin-custody-policy.md (3 signers, 2-of-3).` from open to Done. Link the four rotation tx hashes + the `verify-multisig` + `verify.sh` outputs.
- If a `Destroy deployer key after mainnet rotation` entry exists, mark it Done with the date.

After Step 12, the contracts are live, the multi-sig is admin on both, and there is no single-sig back door anywhere. From this point on every admin op runs through Parts F and G of this guide.

#### Recap of the whole sequence

1. Configure `.env.deploy` (ADMIN_IDENTITY, FEE_ACCOUNT, FEE_BPS, BOOTSTRAP_CREDITS).
2. Generate the deployer identity (`stellar keys generate boundless-deployer`).
3. Fund the deployer (~10 XLM).
4. Run `./scripts/deploy/deploy.sh mainnet` (builds + deploys both contracts + wires `set_events_contract` + writes `deployments/mainnet.json`).
5. Run `./scripts/deploy/register_token.sh mainnet <token>` per supported token (confirm fee-account trustline at the prompt).
6. Run `./scripts/deploy/verify.sh mainnet` to confirm on-chain state matches the deployment record.
7. (Optional) Smoke from boundless-nestjs against the deployed contracts.
8. Build the multi-sig (§E.1 through §E.8).
9. Rotate events admin to multi-sig (`set_admin` from deployer + `accept_admin` from multi-sig).
10. Rotate profile admin to multi-sig (same two transactions on profile).
11. Re-run both verifiers + do one proof-of-life multi-sig op. Record all hashes.
12. `stellar keys rm boundless-deployer` + close out `BACKLOG.md`.

Estimated wall time for a confident operator: half a day on testnet, a full day on mainnet (mostly the §E.8 cooldown + waiting for signers to arrive in the channel).

### E.1. Decide who the three signers are

This is the single most important decision in the whole process. Pick wrong and you weaken the multi-sig before it is even built. No amount of better code or stricter process fixes a bad signer roster.

The rule, from `admin-custody-policy.md` §2:

| Slot | Who | Why |
|---|---|---|
| Signer 1 | **The founder** (you, Collins) | The person ultimately accountable for the contract. Always one slot. |
| Signer 2 | **A second internal trusted person**: co-founder, lead engineer, COO, or whoever your second-in-command is at Boundless. Full-time on the company. | If the founder is the only insider with a key, the founder is a single point of failure. The second insider solves that. |
| Signer 3 | **An external trusted person**: an advisor, a board member, a lawyer, or a founder-friend at another company. Aligned with Boundless but NOT a full-time Boundless employee. | If both insiders align to do something bad together, the external person is the check. If both insider machines are compromised in the same incident, the external person stops the attack. |

So the breakdown is:

- **1 founder address.** (You.)
- **1 internal employee address.** (The lead engineer is the obvious default at Boundless today. Pick the most senior full-time engineer or operator you trust.)
- **1 external trusted-person address.** (Someone outside the company. Same person you would put on the board if Boundless had a board today.)

What NOT to do:

- Do not put two of your own addresses in the three. Multi-sig with two of the same person is single-sig.
- Do not put two close family members (spouse + sibling, parent + child) in the three. They are likely to share a home, share a wifi network, and be attacked in the same incident.
- Do not put a vendor, contractor, or part-time freelancer in the three. They may rotate off the project before you rotate them out of the multi-sig.
- Do not pick the external signer for being "easy to reach." Pick them for being trustworthy and reachable on a backup channel (phone, not just Slack or email).
- Do not make all three signers employees. The external slot is the load-bearing piece of the design.

Once you pick the three, each of them walks through Part D of this guide to set up Freighter and send you their G-address. From here on we assume all three G-addresses are in hand.

### E.2. Create the bootstrap Stellar account

The "bootstrap" account is a regular Stellar account that will turn into the multi-sig. We create it with one normal key, fund it, configure it, and then disable that original key. After §E.5 it can only act when two of the three signers agree.

You will need:

- Stellar CLI version 23.x or newer installed (run `stellar --version` to check).
- A clean machine you trust (your own laptop with disk encryption on, not a shared workstation).
- Internet access.

Generate a fresh keypair for the bootstrap. Replace `testnet` with `mainnet` when you do this for real.

```bash
stellar keys generate boundless-multisig-bootstrap --network testnet
```

This creates a new keypair and stores the secret in the Stellar CLI keystore, encrypted by your OS keychain. **The secret key never appears in your terminal.** That is intentional. Do not try to extract it; you do not need to.

Print the public address so you can see what was generated:

```bash
stellar keys address boundless-multisig-bootstrap
```

You will get back a G-address such as `GAVSIZQZFWUOFMCTDEBY43DBE5GV4RJRW423QQE5QIITLUTVNF34GY4J`. Write this down. After §E.5 this is the address the Boundless contract will recognize as admin.

### E.3. Fund the bootstrap account

A Stellar account does not exist on the network until it has the minimum balance (currently 1 XLM plus 0.5 XLM per data entry such as a signer or trustline). Fund it before you try to configure it, or the configuration transactions will fail with `account_not_found`.

**Testnet:** use friendbot. Replace `<ADDRESS>` with the address you printed in §E.2.

```bash
curl "https://friendbot.stellar.org/?addr=<ADDRESS>"
```

You should get back a JSON payload. The bootstrap will now have 10,000 testnet XLM. (Testnet XLM is fake; it is not worth anything.)

**Mainnet:** send at least 5 XLM to the bootstrap address from the founder's existing mainnet Stellar account (your treasury or your personal wallet). 5 XLM covers the minimum reserve plus headroom for four signer entries (master + three) and a small fee budget for the setup transactions.

### E.4. Add the three signer addresses

Each signer is added in a separate `set-options` transaction. Each transaction is signed automatically by the bootstrap's master key (which still has weight 1 at this point; the CLI handles the signing using the secret it stored in §E.2). You will not see a sign prompt.

Adapt `--network` and `--network-passphrase` if you are on mainnet (use `mainnet` and `"Public Global Stellar Network ; September 2015"`).

```bash
# Signer 1: the founder's Freighter address
stellar tx new set-options \
  --source-account boundless-multisig-bootstrap \
  --signer GDSBURJQPMB7HW7TYN3AL2RSISUHIJIWBWSEM2UQFZJFAP7FX2SU2A4K \
  --signer-weight 1 \
  --network testnet \
  --network-passphrase "Test SDF Network ; September 2015"

# Signer 2: the internal employee's Freighter address
stellar tx new set-options \
  --source-account boundless-multisig-bootstrap \
  --signer GB6ZG4GIPF2YVYPVC4YKLADM2DT73UMV7TR5IERI3YJJOV4NEHLWTWE5 \
  --signer-weight 1 \
  --network testnet \
  --network-passphrase "Test SDF Network ; September 2015"

# Signer 3: the external trusted person's Freighter address
stellar tx new set-options \
  --source-account boundless-multisig-bootstrap \
  --signer GCHITNTBTYD6P76GLPGVUXVDFPPF6EAXZ23QEUXVSFFLV3IQDPIRQRQN \
  --signer-weight 1 \
  --network testnet \
  --network-passphrase "Test SDF Network ; September 2015"
```

What each flag does:

- `--source-account boundless-multisig-bootstrap`: tells the CLI to use the keystore alias from §E.2 as both the transaction source AND the signer. The CLI loads the secret from your OS keychain, signs the tx, and submits it. You never see the secret.
- `--signer <G-address>`: the public address of the person we are adding. Copy this from the message that signer sent you. **Triple-check this string.** A single wrong character means the multi-sig adds the wrong person and that signer's real address is silently excluded.
- `--signer-weight 1`: their signing weight. Weight 1 means each signer counts as one vote. Weight 0 would remove the signer; weights above 1 would give that signer disproportionate power.
- `--network` and `--network-passphrase`: which Stellar network to use. Keep these consistent across all three calls or you will end up with signers on one network and configuration on another.

After all three calls succeed, the bootstrap account has four signers total: the original master key (weight 1) plus the three new G-addresses (weight 1 each). The thresholds are still 0/0/0 at this point, so any one of those four keys can authorize anything. The multi-sig is not yet locked. We fix that in the next step.

### E.5. Set the thresholds AND disable the master key

This is the step that turns the account into a real multi-sig. It is the most important transaction in the whole setup; if you skip the `--master-weight 0` part, the multi-sig is defeated because the original bootstrap key can still sign solo. (This is the trap call-out in `docs/multisig-preflight.md` §2.)

Do it all in a single transaction:

```bash
stellar tx new set-options \
  --source-account boundless-multisig-bootstrap \
  --low-threshold 0 \
  --med-threshold 2 \
  --high-threshold 2 \
  --master-weight 0 \
  --network testnet \
  --network-passphrase "Test SDF Network ; September 2015"
```

What each flag means in plain English:

- `--low-threshold 0`: read-only operations (such as `bump_sequence`) need no signatures. There is nothing sensitive at this tier.
- `--med-threshold 2`: normal operations (`set_fee_bps`, `pause`, `unpause`, contract upgrades) need two signatures from the three signers.
- `--high-threshold 2`: dangerous operations (`set_fee_account`, `set_admin`, adding or removing signers) need two signatures by Stellar's account model. The Boundless policy in `admin-custody-policy.md` §4 requires three-of-three for these operations in practice. The three-of-three rule is enforced by the founder requesting all three signatures in Slack, not by the threshold itself.
- `--master-weight 0`: the original bootstrap key (the one you generated in §E.2) can no longer sign. From this moment on, the only way to do anything with this account is to collect two signatures from the three signers added in §E.4.

After this transaction lands, the bootstrap secret in the CLI keystore is useless. Keep it as evidence but understand it is now a dead key. If you lose it, that is fine. If someone steals it, that is also fine. There is nothing it can do alone.

### E.6. Verify with the script

Run the verification script. It is the safety net that catches any mistake you made above.

```bash
./scripts/admin/verify-multisig.sh <BOOTSTRAP_ADDRESS> testnet
```

It must print exactly:

```
PASS: all 6 checks passed
```

The 6 checks are:

1. Master key weight = 0.
2. Low threshold = 0.
3. Medium threshold = 2.
4. High threshold = 2.
5. Exactly 3 non-master signers.
6. Each signer has weight 1.

Below that, the script prints the three signer addresses it found. **Read them.** Confirm by eye that each one matches one of the three G-addresses the signers sent you. A typo in §E.4 means the multi-sig is configured but with the wrong people in it, and you only discover that when you try to use it for real.

If verification fails:

- Re-read which check failed.
- If you have NOT yet flipped `--master-weight 0`, you can fix things with another `set-options` transaction from the bootstrap. For example, if you typoed Signer 2's address, run `set-options --signer <wrong-address> --signer-weight 0` to remove the wrong entry, then `set-options --signer <correct-address> --signer-weight 1` to add the right one.
- If you HAVE already flipped `--master-weight 0` and the multi-sig is broken, you cannot fix it with the bootstrap. You start over from §E.2 with a fresh bootstrap account. The old one is abandoned; the small XLM balance is the cost of the mistake.

Do NOT move on with a failing `verify-multisig`.

### E.7. Tell the signers it is ready

Share the multi-sig public address (the bootstrap address from §E.2) with all three signers via the agreed trusted channel (Signal, secure video, or in person, NOT email and NOT public Slack). Each signer should:

- Record the address in their local notes.
- Add it as a "Watch-only" account in their Freighter so they can see whenever a transaction touches it.
- Acknowledge receipt back to you so you have written confirmation each signer got it.

### E.8. The 1-day cooldown (mainnet only)

Wait 24 hours before doing anything important with the new multi-sig on mainnet. During this window:

- Re-run the testnet drill (Part H) with all three signers.
- Have each signer practice signing one throwaway transaction on testnet, end to end.
- Verify each signer can reach you on the backup channel within 1 hour.

This cooldown catches "Signer 3 forgot their Freighter passphrase" and "Signer 2's recovery phrase is at the wrong house this week." Better to find out before the multi-sig is live as Boundless admin.

---

## Part F. Part 3: How to sign a real transaction (day-to-day signer task)

This is the bit you will do over and over. It happens any time the founder or operations team needs to do something with the Boundless admin authority: change a fee rate, pause the app in an emergency, ship a contract upgrade.

The flow happens on [Stellar Lab](https://lab.stellar.org). Lab is a free website made by the Stellar Foundation. It is the easiest tool for multi-sig because it can connect directly to Freighter.

### F.0. How the founder builds the transaction in the first place

*(Signers can skim this section. The founder must know it cold.)*

Before any signer signs anything, someone has to build the transaction. That someone is the founder (or the operations engineer the founder designates).

There are two shapes of admin transaction. You need to know which shape applies because the build flow differs.

**Shape 1: Stellar native operations.** Anything that changes the multi-sig account itself: adding or removing a signer, changing thresholds, paying out XLM, setting up a trustline. These are built directly in Stellar Lab.

**Shape 2: Soroban contract operations.** Anything that calls into the Boundless contract: `set_fee_bps`, `pause`, `unpause`, `propose_upgrade`, `apply_upgrade`, `set_admin`, `accept_admin`, `register_supported_token`, `cancel_pending_upgrade`, `migrate`. These are built via the Stellar CLI (or an admin script), then loaded into Stellar Lab for the signers.

#### F.0.A. Building a native op (Shape 1) in Stellar Lab

Example use case: rotating Signer 2 because their laptop was stolen.

1. Open [lab.stellar.org](https://lab.stellar.org). Switch the network in the top-right corner to "Test Net" or "Public Net" depending on where you are.
2. Click "Build Transaction" in the left nav.
3. Set **Source Account** to the multi-sig G-address (the bootstrap address from §E.2).
4. Click **Fetch Next Sequence Number** next to the Sequence field. Lab pulls the current sequence from Horizon and fills it in.
5. Set **Base Fee** to `1000` (which is 0.0001 XLM). Bump it higher if the network is congested.
6. Click **Add Operation** and pick **Set Options** from the drop-down. Fill in only the fields you want to change:
   - To remove a signer: **Signer Public Key** = the old signer's G-address. **Signer Weight** = `0` (weight 0 removes them).
   - To add a signer: **Signer Public Key** = the new signer's G-address. **Signer Weight** = `1`.
   - To change a threshold: **Medium Threshold** = `2`, etc.
   - You can stack multiple Set Options operations in one transaction (e.g., add one + remove one in the same tx).
7. Click **Build**. Lab produces an unsigned XDR string (a long block of letters and numbers).
8. Click **Sign Transaction** at the top. Lab opens its signing view with your tx loaded.
9. Copy the URL of that signing view. The URL embeds the XDR after `?xdr=`. This URL is what you post to the signers in Slack.

That is the entire founder side for a Shape 1 op. The signers then walk through §F.2 to sign and you submit at the end.

#### F.0.B. Building a contract op (Shape 2) via Stellar CLI

Example use case: changing the platform fee from 1.5% to 1.7%.

Soroban contract operations are more complex than native ops because the contract requires `admin.require_auth()`. That auth has to be packaged inside the transaction's Soroban authorization entries, not just the outer Stellar signature. The Stellar CLI handles most of this for you.

```bash
stellar contract invoke \
  --network testnet \
  --source-account boundless-ops-builder \
  --id <EVENTS_CONTRACT_ID> \
  --build-only \
  -- \
  set_fee_bps \
  --new_bps 170
```

What the flags mean:

- `--source-account boundless-ops-builder`: any funded Stellar account on the network. Used to simulate and pay the network fee. It does NOT need to be the admin. Use a low-balance throwaway "builder" account, not the multi-sig.
- `--id <EVENTS_CONTRACT_ID>`: the boundless-events contract address on the network. Get this from `BACKLOG.md` (testnet) or the deploy notes (mainnet).
- `--build-only`: tells the CLI to print the unsigned XDR instead of trying to sign and submit. Without this, the CLI would try to send the tx with just the builder's signature, which the contract would reject because the builder is not admin.
- After the `--` separator: the contract function name (`set_fee_bps`) and its arguments. `170` is the new fee in basis points; 170 bps = 1.7%.

The CLI prints an XDR string. That is your unsigned transaction with Soroban auth placeholders.

You now need two things:

1. **Replace the source account** from `boundless-ops-builder` to the multi-sig, because the contract checks the admin via auth, but Lab and Horizon also expect a coherent source. The cleanest way: paste the XDR into Stellar Lab's **View XDR** tab, change the Source Account field to the multi-sig address, click **Save** → Lab re-encodes it.
2. **Re-simulate** so the Soroban resource fee and footprint match the new source. Lab does this for you when you click **Sign Transaction**.

Then copy the Lab signing URL and post it to the signers.

For ops that are tricky to hand-assemble (any `propose_upgrade`, `apply_upgrade`, or `migrate`), there are helper scripts in `boundless-nestjs/scripts/admin/` that produce the correct XDR directly. Use those if they exist for your op; they handle the source-account swap and Soroban auth packaging for you. If a helper does not exist yet for the op you need, write one before you do this manually on mainnet.

#### F.0.C. Posting the request to signers

In `#ops-admin-requests` Slack channel, post a message that includes:

- A one-line plain-English description of what the transaction does (`Adjusting hackathon fee from 1.5% to 1.7%`).
- The exact operation name (`set_fee_bps`, `pause`, etc.).
- The reason and the business context (`Per the pricing review on 2026-06-04, link to doc.`).
- The Lab signing link (`https://lab.stellar.org/transaction/sign?xdr=...`).
- Which threshold applies (`medium = 2 sigs needed` or `high per-policy = 3 sigs needed`).
- A deadline (`Please sign by EOD Friday.`).

Then tag the signers you need. For a 2-sig op, tag two of the three (rotate which two over time so all three stay practiced). For a 3-sig op, tag all three.

Wait for the signed XDRs to come back in the thread. Once you have a quorum:

1. Paste the final fully-signed XDR into Stellar Lab's **Submit Transaction** tab.
2. Click **Submit**. Lab posts it to Horizon.
3. The tx hash comes back. Paste the hash in the Slack thread to confirm: `Landed: <https://stellar.expert/explorer/testnet/tx/HASH|HASH>`.

That closes the loop for everyone watching.

### F.1. The founder posts a signing request

In the agreed channel (probably a Slack channel like `#ops-admin-requests`), the founder will post a message like:

> Operation: `set_fee_bps`
> Reason: Adjusting hackathon fee from 1.5% to 1.7% per the pricing review.
> Lab transaction link: https://lab.stellar.org/transaction/sign?xdr=AAAAA...
> Threshold required: 2 of 3

You should:

1. Read the operation name and the reason.
2. Click the Lab link.
3. **Compare what Lab shows to what the founder said.** If the founder said `set_fee_bps` but Lab shows `set_fee_account`, stop and call the founder. Something is wrong.

### F.2. Sign on Stellar Lab

When you click the link, Lab opens with the transaction already loaded. It shows the source account, the operations, and the amounts.

1. Scroll through the operations. Make sure they match what the founder said in plain English.
2. Click "Sign with Freighter."
3. Freighter pops up. Read the popup. The signer address in Freighter must match your signer address.
4. Click "Approve" in Freighter.
5. Lab shows that your signature was added. There is now an updated transaction with your signature in it.
6. Copy the updated transaction XDR (a long string of letters and numbers) and post it back in the Slack thread.

### F.3. The next signer goes

A second signer does the same thing: clicks the Lab link the founder posts (or the updated XDR from step F.2), reviews the transaction, signs with their Freighter, posts the result back.

Once two signatures are on the transaction, the founder (or anyone with the final XDR) clicks "Submit" in Lab. Lab sends it to Stellar. A few seconds later it lands on chain. Done.

### F.4. What to refuse to sign

Sign nothing that:

- The founder did not explicitly request in the Slack thread.
- Differs from what the founder described. If the description says "change fee to 1.7%" but the transaction in Lab is moving 10,000 XLM somewhere, refuse and call the founder.
- Looks rushed or pressured ("we need this in five minutes, just sign it"). Real Boundless ops requests are never that urgent except for `pause()` during an active incident, and even then you should hear from the founder by voice, not just a message.
- Came in via email, DM, or any channel that is not the agreed-upon Slack channel. Phishers will try.

If anything feels off, **refuse and call the founder by phone or video.** Refusing to sign costs minutes; signing something bad can cost the company.

---

## Part G. Part 4: How to ask for a multi-sig operation (employee task)

This is for employees and operations folks who need something done on the contract: changing a fee, registering a new token, pausing for an incident.

You do not sign. You request. The founder coordinates the actual signing.

The process:

1. Write the request in `#ops-admin-requests` Slack channel. Include:
   - What operation you need (`set_fee_bps`, `register_supported_token`, etc.)
   - Why (one or two sentences of business context)
   - When you need it (most things are not urgent)
2. The founder reviews and acknowledges within the agreed SLA.
3. The founder builds the transaction, posts the Lab link, and tags two signers.
4. Two signers sign per Part F.
5. The founder submits.
6. The founder confirms in the thread that it landed, with the tx hash.
7. You verify the change is live (check the contract, check the UI, etc.).

If you skip step 1 and just text the founder directly, the request will probably get bumped to the channel anyway. The channel is the paper trail. Use it.

---

## Part H. The drill: practice once on testnet

Before the multi-sig is used for anything real on mainnet, the founder runs all three signers through a practice drill on testnet. The drill exists so that the first time you sign something for real, it is not actually your first time.

The drill has three short scenarios.

### H.1. The "2 of 3 routine op" scenario

The founder posts a tiny payment transaction (e.g., send 1 testnet XLM from the multi-sig to one of the signers' wallets). Two signers go through Part F. The payment lands. Everyone has seen the flow.

### H.2. The "3 of 3 big op" scenario

The founder posts a transaction that simulates `set_admin` (the most dangerous op). All three signers sign it. The transaction goes through. Everyone has now used the high-threshold path.

### H.3. The "1 of 3 should fail" scenario

The founder posts another transaction. Only one signer signs it. The submission fails with an error like `tx_bad_auth_extra` or `op_low_threshold`. This is supposed to fail. The point of the drill is to see the failure mode so you recognize it if you ever see it on mainnet.

After the drill, the founder logs it in the team's notes with the testnet transaction hashes and the date. The drill repeats every quarter so the process stays fresh.

---

## Part I. What this multi-sig is allowed to do

The Boundless contract limits even the admin. Knowing what the admin can and cannot do helps you sanity-check requests.

The admin **can:**

- Change the default fee rate (`set_fee_bps`).
- Change the fee account address (`set_fee_account`). This one needs all three signatures.
- Pause the contract in an emergency (`pause`).
- Resume after a fix (`unpause`).
- Propose a contract upgrade (`propose_upgrade`). The upgrade does not apply for at least one day after the proposal, so customers can react.
- Apply a proposed upgrade after the timelock (`apply_upgrade`).
- Cancel a queued upgrade (`cancel_pending_upgrade`).
- Run the post-upgrade migration (`migrate`).
- Add or remove tokens from the whitelist.
- Rotate the admin itself (`set_admin` and `accept_admin`). Needs all three signatures.

The admin **cannot:**

- Take funds out of escrow. Escrow is paid only by the contract's own rules (winners, milestones, refunds). The admin has no withdrawal authority.
- Override an event's winner. Winners are set by `select_winners` and are subject to the contract's checks.
- Reverse a settled transaction. Once a payment lands, it lands.

If anyone tells you to "let me sign one thing to drain the escrow," that request is impossible. The contract will reject it. Refuse anyway and call the founder.

---

## Part J. What to do if things go wrong

### J.1. You lost your laptop

Your laptop is encrypted, so the thief cannot trivially open Freighter. Still, do this within the day:

1. Tell the founder. By phone, not Slack.
2. The other two signers can still operate the multi-sig (2 of 3 is enough).
3. The founder and the surviving signers rotate the multi-sig to a new account that excludes your lost wallet. This follows the procedure in `docs/admin-custody-policy.md` §5.2.
4. You set up a brand new Freighter wallet on a new machine and the founder adds you to the new multi-sig.

### J.2. You lost your recovery phrase paper

If you still have access to your laptop and your Freighter is unlocked, you are not in immediate danger. But you have lost your safety net. Do this:

1. Tell the founder.
2. Generate a fresh Freighter wallet (new recovery phrase, new public address) on the same machine or a new one.
3. Back up the new recovery phrase the right way this time.
4. Send the new public address to the founder.
5. The founder rotates the multi-sig to swap your old signer address for the new one.

### J.3. Your Freighter passphrase no longer works

Freighter cannot recover a forgotten passphrase. If you cannot remember it:

1. Tell the founder.
2. If you have your recovery phrase paper, you can re-import the wallet into a new Freighter install on a new browser profile, set a new passphrase, and you keep the same address. No rotation needed.
3. If you do not have your recovery phrase paper either, treat this like J.1 (lost laptop). You are out. The other two signers carry on, the founder rotates you out and a new signer in.

### J.4. Something looks weird

"Weird" includes:

- Lab shows a transaction that does not match what the founder described.
- Freighter is asking for a signature you did not initiate.
- A new browser extension appeared in your Boundless Signer profile.
- Your laptop is acting strangely (apps installing themselves, browser redirecting weirdly).

Refuse to sign. Call the founder. Do not write off "weird" as a glitch.

### J.5. You think your wallet might be compromised

Assume it is. Immediately:

1. Tell the founder by phone.
2. The founder runs an emergency rotation per `docs/admin-custody-policy.md` §5.2 (pause first, then swap you out, then unpause).
3. Wipe the affected machine. Set up a new wallet on a new machine.

This is what 2-of-3 is for. Losing one signer is recoverable. Do not be embarrassed; just speak up fast.

---

## Part K. Mistakes to avoid (the anti-patterns list)

Things that defeat the multi-sig:

- Two signers using the same laptop "just for now." If one machine gets hacked, two signatures are owned. Multi-sig becomes single-sig.
- Saving the recovery phrase in 1Password, iCloud, Google Drive, or any cloud service. Cloud service gets hacked, recovery phrase leaks, attacker steals your wallet.
- Photographing the recovery phrase. Same problem.
- Sharing Freighter installs across two signers via screen-share or "let me borrow your laptop for a sec." Two signers must be two physical people on two physical machines.
- Skipping the testnet drill. The drill catches the mistakes that cost real money on mainnet.
- Signing while distracted, in a meeting, or while traveling on an airport wifi. Sit down at your own machine, in your own space, with the request fully reviewed.
- Using the Boundless Signer browser profile for personal browsing. The profile exists so it has no other extensions and no other tabs. Once you log into your personal Gmail in it, that protection is gone.
- Treating the hardware upgrade as optional. Per `admin-custody-policy.md` §10, when Boundless hits a TVL trigger, we move all three signers to hardware-isolated keys (Yubikeys). That is not "if we get around to it." That is a commitment that fires automatically.

---

## Part L. Glossary

| Term | Plain English |
|---|---|
| **Address (G-address)** | A public ID for a Stellar account. Starts with G. Safe to share. |
| **Admin** | The role the contract recognizes as allowed to change settings. The Boundless admin is the multi-sig account. |
| **Boundless** | The platform you work for. Runs bounties, hackathons, grants, and crowdfunding on Stellar. |
| **Contract** | The on-chain code that runs Boundless. There are two: `events` and `profile`. |
| **Drill** | A practice run of the multi-sig procedure on testnet. We do it once before mainnet and every quarter after. |
| **Escrow** | Money held by the contract on behalf of an event, waiting to be paid to winners or refunded to contributors. |
| **Founder** | Collins. The current operator of the multi-sig and the main contact for any signing question. |
| **Friendbot** | A free Stellar service that hands out testnet XLM. We use it to fund testnet wallets. |
| **High threshold** | The signature count needed for the most dangerous ops (`set_fee_account`, `set_admin`). Set to 2 by default for Boundless but the policy requires 3 of 3 in practice for those operations. |
| **Lab (Stellar Lab)** | A free website that lets us build and sign transactions. The tool we use for multi-sig signing. |
| **Low threshold** | The signature count for read-only ops. Set to 0 because no important op uses the low threshold. |
| **Medium threshold** | The signature count for normal ops (`set_fee_bps`, `pause`, `unpause`, upgrades). Set to 2. This is the workhorse setting. |
| **Master key** | The original signing key of an account. For our multi-sig the master key is disabled (weight 0). Disabling the master is the step that turns a normal Stellar account into a multi-sig. |
| **Multi-sig** | Short for "multi-signature." A Stellar account that needs more than one signature to act. |
| **Operation** | One action in a transaction. A transaction can have several operations bundled together. |
| **Quorum** | Enough signers to satisfy the threshold. For 2-of-3, two signers is a quorum. |
| **Recovery phrase** | The 12 words Freighter shows you once. The key to your wallet. The most important thing in this guide. |
| **Signer** | A person who holds one of the three multi-sig keys. |
| **Stellar** | The blockchain Boundless runs on. |
| **TVL** | "Total value locked." The dollar value of money currently sitting in Boundless escrow. The hardware-upgrade trigger fires when this passes the threshold in `admin-custody-policy.md` §10. |
| **Transaction (tx)** | A bundle of operations submitted to Stellar. Signed before it is submitted. |
| **Threshold** | The number of signatures an account requires before it will act. Boundless multi-sig is 2-of-3 by default. |
| **Wallet** | A piece of software that holds your keys. Freighter is a wallet. |
| **XDR** | The format Stellar uses to encode transactions. Looks like a long block of letters and numbers. You will see it in Stellar Lab. You do not need to understand it; the tools handle it. |
| **XLM** | The native token of the Stellar network. Used for transaction fees and (sometimes) escrow payments. |

---

## Where to look next

- `docs/admin-custody-policy.md`: the formal policy. Read it once.
- `docs/multisig-preflight.md`: the technical checklist for setting up a new multi-sig. Founder uses this for setup.
- `docs/mainnet-deploy-runbook.md`: the procedure for deploying Boundless contracts and pointing them at the multi-sig.
- `scripts/admin/verify-multisig.sh`: the script that confirms a multi-sig is correctly configured.

If anything in this guide is unclear, ask the founder to update it. The whole point of writing it down is so the next person does not have to figure it out from scratch.
