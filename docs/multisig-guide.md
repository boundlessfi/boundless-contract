# The Boundless multi-sig guide (plain English)

This guide is for the three signers and anyone at Boundless who needs to understand how the admin multi-sig works. It assumes no prior Stellar or crypto knowledge.

If you read only one thing, read **Part A: The 60-second version** below.

Where this fits in:

- **You** if you are one of the three signers: read Parts A, B, C, D, E, J, K.
- **You** if you are a Boundless employee who is not a signer: read Parts A, B, F, G, J, K.
- **You** if you are the founder running setup: also read `docs/multisig-preflight.md`.

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

This part is for the founder. Signers do not need to do anything here, just wait for the founder to confirm the multi-sig is set up.

Estimated time: 20 minutes (plus 1 day of clock-watching if you want to be extra careful before going to mainnet).

### E.1. Collect addresses

The founder collects all three signer public addresses. Double-check each one against what each signer said in voice / video. A typo here is a real problem.

### E.2. Run the multi-sig setup script

The founder runs the setup with the three collected addresses. The procedure is in `docs/multisig-preflight.md`. The short version:

1. Create a fresh Stellar account that will become the multi-sig.
2. Fund it with the minimum reserve (testnet: friendbot is free; mainnet: from your treasury).
3. Add the three signer addresses with weight 1 each.
4. Set thresholds to 0 for low, 2 for medium, 2 for high.
5. Set the master key weight to 0 (this is the "lock the original key out" step).

### E.3. Run the verification script

```bash
./scripts/admin/verify-multisig.sh <MULTISIG_ADDRESS> testnet
```

The script must say `PASS: all 6 checks passed.` and print the three signer addresses. The founder eyeballs the printed addresses to confirm they match the ones the signers sent.

If any check fails, stop and fix it. Do not move on. Do not point the contract at a multi-sig that did not pass verification.

### E.4. Tell the signers it is ready

The founder shares the multi-sig public address with the three signers via the same trusted channel. Each signer adds it to their notes for the day-to-day signing flow in Part F.

---

## Part F. Part 3: How to sign a real transaction (day-to-day signer task)

This is the bit you will do over and over. It happens any time the founder or operations team needs to do something with the Boundless admin authority: change a fee rate, pause the app in an emergency, ship a contract upgrade.

The flow happens on [Stellar Lab](https://lab.stellar.org). Lab is a free website made by the Stellar Foundation. It is the easiest tool for multi-sig because it can connect directly to Freighter.

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
- Change the fee account address (`set_fee_account`) — but this one needs all three signatures.
- Pause the contract in an emergency (`pause`).
- Resume after a fix (`unpause`).
- Propose a contract upgrade (`propose_upgrade`). The upgrade does not apply for at least one day after the proposal, so customers can react.
- Apply a proposed upgrade after the timelock (`apply_upgrade`).
- Cancel a queued upgrade (`cancel_pending_upgrade`).
- Run the post-upgrade migration (`migrate`).
- Add or remove tokens from the whitelist.
- Rotate the admin itself (`set_admin` and `accept_admin`) — needs all three signatures.

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
