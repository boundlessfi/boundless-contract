# Boundless — STRIDE Threat Model

**Version:** 1.0  
**Date:** September 2026  
**Prepared for:** Stellar Development Foundation — Soroban Security Audit Bank  
**Prepared by:** Boundless Engineering  
**Contracts:** `boundless-events` 1.7.0, `boundless-profile` 1.2.0  
**Repository:** github.com/boundlessfi/boundless-contract  

---

## Summary

This document is the STRIDE threat model for the two Soroban contracts that anchor the Boundless platform: `boundless-events`, which holds prize escrow and drives the lifecycle of hackathons, bounties, grants, and crowdfunding campaigns, and `boundless-profile`, which records builder reputation and earnings. It follows the four-question structure recommended by the Stellar Development Foundation: what we are working on, what can go wrong, what we are doing about it, and whether we did a good job.

The model covers 72 threats across the six STRIDE categories, each tied to a step in the on-chain flow and to a trust boundary in the data-flow diagram, and each rated for likelihood and impact. Ten are Priority 1 — the controls whose failure would be catastrophic — and all ten are mitigated and tested; every open item is Priority 2 or lower. Forty-nine are mitigated by controls in the contracts or in operating procedure; every contract-level control is exercised by a test that asserts its failure mode. Thirteen are accepted residual risks with a written rationale. Ten are open: three are design questions on which auditor input is requested (recovery when a delegated manager's key is lost, cancellation of a grant that already carries awards, and the treatment of a refund recipient that can no longer receive the token), and seven are hardening changes scheduled before audit fieldwork, the most important of which is restoring the upgrade timelock from its current value of zero.

## 1. What are we working on?

### 1.1 Overview

Boundless is an on-chain coordination platform for hackathons, bounties, grants, and crowdfunding campaigns on Stellar. Its design goal is trust-minimized prize settlement: the budget for a program is locked in a Soroban contract before any builder does work, and it can leave only to winners, to milestone recipients, or back to the accounts that funded it. Neither the program owner nor the platform admin can redirect escrow.

An event is an **open pool with committed prize floors**. The owner deposits a budget and publishes a per-position minimum (`prize_floors`). Winner selection names the actual award per position, which may exceed a floor but never fall below it. Every award is reserved against the pool so that a later selection cannot promise funds an earlier winner is still owed.

**Roles and terms used in this document.**

- **Owner** — the account that creates and funds an event (the organizer). Custody of the pool always belongs to the owner and to contributors.
- **Manager** — an optional delegate who, once accepted, holds selection and cancellation authority for one event. Where no manager is delegated, the owner is the manager.
- **Contributor** — any account that tops up an event's pool.
- **Builder** — an account that applies to or submits work for an event; a **winner** is a builder named in a selection.
- **Admin** — the platform's 2-of-3 multi-signature account. The only privileged role in either contract.
- **Event types** — hackathon, bounty, grant, and crowdfunding. Hackathons and bounties are **single-release**: each winner pulls their prize with `claim_prize` inside a 90-day window. Grants and crowdfunding campaigns are **multi-release**: funds leave per milestone through `claim_milestone`.
- **Permissionless crank** — an entry point that requires no signature, so that anyone can advance a process (used for cancellation refunds).
- **Managed and external wallets** — a user's Stellar wallet is either *managed* (a keypair the platform generates, sponsors, holds under envelope encryption, and signs with on the user's behalf) or *external* (a wallet such as Freighter whose key never leaves the user's device). Most builders and organizers use managed wallets; organizers may connect an external wallet.

### 1.2 Scope and assets

**In scope.** The `boundless-events` and `boundless-profile` contracts at the versions above, the cross-contract interface between them, the administrative operations that govern them, and the custody procedures for the admin account. The backend orchestrator, web application, and admin portal are covered only at the points where they touch the contracts or hold material that could affect them.

**Out of scope.** Backend and web-application internals, the Didit service, the Stellar network and Soroban host (assumed correct; see 1.7), and the issuer of the supported token.

| Asset | Where it lives | Why it matters |
|---|---|---|
| Escrow funds | Token balance held by `boundless-events` (one balance for all events; 1,030 USDC across three active bounties at the time of writing) | Critical. The purpose of the system; any path that moves or locks it is the highest-value target. |
| Per-event accounting | `remaining_escrow`, `EventOwedTotal`, winner records, prize awards, contributor amounts | Critical. Drives every release and refund; an inconsistency here becomes a fund movement. |
| Admin signer keys | Three signers on separate machines; 2-of-3 threshold | Critical. The quorum can pause, change fees, re-point bindings, and replace contract code. |
| Fee account key | Same custody as the admin | High. Receives all platform revenue. |
| Prize floors and event terms | `EventRecord` | High. The published guarantee builders rely on. |
| Managed wallet private keys | PostgreSQL, envelope-encrypted: a per-wallet data key wrapped by a master encryption key held only in the backend environment; key material is excluded from ORM reads except on the signing path | Critical. Together with the master key they control every managed owner, manager, builder, and winner wallet. |
| External wallet keys | User devices | High per event. Key loss or compromise affects that party's funds or authority. |
| Reputation and earnings | `boundless-profile` | Medium. Informational; carries no on-chain privilege, but the platform's trust signal. |
| Backend secrets | Server environment | Critical for the wallet master encryption key (see above); High for the rest. No server-held secret can act as the admin. |
| Identity data | Didit only | High; never enters Boundless systems. |
| Availability of off-chain services | Backend, RPC provider, Didit | Medium. Outages delay operations but cannot move funds; every contract entry point remains callable directly. |

### 1.3 Components

| Component | Role |
|---|---|
| `boundless-events` | On-chain anchor for all four event types. Escrow custody (deposit, reserve, release, refund), event lifecycle, applicant, contributor, and submission registries, winner awards, milestone claims, paged cancellation, per-event manager delegation, timelocked upgrades with paged data migration. Mainnet: `CCFVEGOQJEM47LRAJU2LHEK4KTL5VYN7AOGZ2HH2GNHAMXTILNMMJGQZ`. |
| `boundless-profile` | Per-user reputation score and per-token lifetime earnings. Mutated only by the events contract through cross-contract calls; users may bootstrap their own record. Mainnet: `CD3KH4OE7HDHHHUYFX3U4L7NLIILMXAY6HM5FEH2UH6UBOKX4HDNE3PC`. |
| Backend orchestrator | NestJS service. Holds rich content, user accounts, KYC state, draft lifecycle, and the off-chain credit ledger. Builds Stellar transactions, submits them through RPC, indexes contract events, and assembles crowdfunding milestone-release transactions for admin co-authorization. Runs the custodial wallet service: generates keypairs for managed wallets, sponsors their reserves, holds the private keys under envelope encryption, and signs for them. It does not hold the admin quorum. |
| Web application | Next.js front end in the user's browser. Calls the backend API; wallets sign transactions locally. |
| Admin portal | Staff-only application against the backend admin API, with step-up TOTP for sensitive mutations. |
| Didit KYC | External identity verification. Receives documents directly from the user and returns HMAC-signed webhooks. No identity data is stored on Boundless infrastructure. |
| Stellar RPC | Managed Soroban RPC endpoint used for submission and event polling. |
| Admin | 2-of-3 Stellar multi-signature account `GCVK72I6TVJVDTTY4UKU6MQT4QJ2T2AAG3NULNEUDM46L3UOQYDSO4O2` (three signers of weight 1, medium and high thresholds of 2, master key weight 0). Sole holder of every privileged contract operation: pause, fee configuration, supported-token list, upgrades, migration, admin rotation, and the co-authorization of crowdfunding milestone releases. |
| Fee account | `GADSIP2HPINWTMEUNMVYA5NJRL372OV52HDEJTRTLDZXMHADIPQNYH55`. Receives platform fees; same custody as the admin. |
| Owner, contributor, and builder wallets | Stellar accounts holding a trustline to a supported token (USDC on mainnet); managed wallets are created and sponsored by the platform, external wallets are user-held. |

### 1.4 On-chain flow

The threat tables in Section 2 reference these steps.

1. **Deploy and configure.** The admin deploys `boundless-profile`, then `boundless-events` (admin, fee account, fee rate, profile address); binds the two with `set_events_contract` (the first binding is single-step, later rotation is a timelocked two-step); and registers supported tokens.
2. **Create.** The owner calls `create_event`. For hackathons, bounties, and grants the full budget plus the platform fee is pulled from the owner in the same transaction; the fee is forwarded to the fee account and the budget is held by the contract. Crowdfunding events start at zero and are funded by contributors. Prize floors must be positive and sum to at most the budget. A manager may be proposed.
3. **Delegate (optional).** The proposed manager calls `accept_manager` within 17,280 ledgers (about one day). From then on the manager, not the owner, holds selection and cancellation authority. Custody of the pool is unchanged.
4. **Top up.** Contributors call `add_funds` (minimum 10 tokens). Non-crowdfunding top-ups pay the fee at deposit; crowdfunding deposits are fee-free and the fee is taken at release. A per-event running total of non-owner contributions is maintained for refunds.
5. **Apply (bounties).** Builders call `apply_to_bounty`; the events contract bootstraps their profile.
6. **Submit.** Builders call `submit(event_id, applicant, slot, content_uri)` to anchor an off-chain submission. Bounties require a prior application; hackathons do not. A wallet may hold several slots; resubmitting to a slot updates it in place.
7. **Select.** The manager calls `select_winners` with up to 50 `WinnerSpec{recipient, position, amount, reputation_bump}` entries per call. Single-release events may select in as many calls as needed; multi-release events accept exactly one selection. Each position is awardable once; each amount must meet the position's floor; the batch total plus everything already awarded but unclaimed (`EventOwedTotal`) must fit in `remaining_escrow`. Single-release events open, or extend, a 90-day claim window.
8. **Claim a prize.** Each winner calls `claim_prize(event_id, position)`. The contract records the payment, then transfers the token, then bootstraps the profile, bumps reputation, and registers earnings on a best-effort basis.
9. **Claim a milestone.** For grants the owner calls `claim_milestone(event_id, recipient, milestone, reputation_bump)`; the recipient must hold a winner record, each (recipient, milestone) pair is claimable once, and the final milestone sweeps the recipient's remainder. Crowdfunding additionally requires the admin's co-authorization, pays the owner `remaining_escrow / milestones_left`, and takes the fee at release.
10. **Cancel.** The manager calls `start_cancel`. It is refused while single-release prizes remain inside their claim window. Events with no outside contributors settle inline; otherwise the event enters `Cancelling`, and anyone may crank `process_cancel_batch` (up to 25 refunds per call) and `finalize_cancel`, which returns the owner's residual.
11. **Govern.** The admin may pause and unpause, change the fee (capped at 10%) and fee account, manage the supported-token list, rotate the admin in two steps, and upgrade: `propose_upgrade`, `apply_upgrade`, `migrate_events` (paged, up to 8 events per call), and `migrate` (one-shot version stamp).

### 1.5 Data-flow diagrams

![Figure 1 — System data-flow diagram with trust boundaries](figures/threat-model-figure-1-system-dfd.svg)

*Figure 1 — System data-flow diagram. Element types are identified in the legend; dashed amber regions are trust boundaries.*

![Figure 2 — Escrow lifecycle and authorization](figures/threat-model-figure-2-escrow-lifecycle.svg)

*Figure 2 — Escrow lifecycle inside `boundless-events`. Each step shows which key must sign it and how the pool balance moves; the lower band shows how the pool is accounted for and where funds leave to.*

### 1.6 Trust boundaries

| ID | Boundary | Description |
|---|---|---|
| TB1 | Internet / Backend | All user-originated input is untrusted. Enforced by JWT authentication, the KYC guard, and schema validation. Nothing the backend accepts is trusted by the contracts (see TB5). |
| TB2 | Backend / Stellar network | The network is permissionless; anyone can call the contracts directly and bypass the backend entirely. The backend reads chain state before acting and never assumes it is the only writer. |
| TB3 | Backend / Didit | External service. Webhook authenticity is enforced with HMAC. No identity data crosses into Boundless systems. |
| TB4 | Admin browser / Admin portal / Backend | Staff-only surface with OIDC sign-in and step-up TOTP for sensitive actions. |
| TB5 | Off-chain / On-chain | The contracts enforce every rule themselves: each state change needs the Soroban authorization of the owner, manager, winner, contributor, or admin quorum. The backend can never exceed the authority of the wallets it can sign for. For external wallets that is nothing; for managed wallets it is that wallet's own authority, so a backend compromise that also yields the master encryption key can act as any managed owner, manager, builder, or winner (Spoof.10), but never as the admin quorum, which is held by three external signers. |
| TB6 | events ↔ profile | Cross-contract calls. The profile contract accepts mutations only from the configured events contract address; the events contract treats profile failures on the prize-claim path as non-fatal. |
| TB7 | Owner ↔ Manager | Within one event. The manager holds operational authority (select, cancel, rotate) but never custody: refunds route to the owner and contributors by construction, and `claim_milestone` remains owner-authorized. |

### 1.7 Threat actors and assumptions

| Actor | Capability | Motivation |
|---|---|---|
| Anonymous network participant | Can call any public entry point directly, spam registries, replay captured transactions; cannot forge signatures. | Theft, disruption, griefing. |
| Builder or contributor | Legitimate participant who may act in bad faith: rewrite a submission, remove a trustline, spam applications. | Advantage in a program; griefing an owner. |
| Owner or manager | Per-event privilege: creates, funds, selects, cancels. May try to reclaim or redirect a pool or favour themselves. | Financial gain at builders' expense. |
| Compromised backend | Can build and submit arbitrary transactions, corrupt off-chain state, and leak secrets. With the wallet master encryption key it can sign as any managed wallet; it can never sign as the admin. | Theft from events whose owner, manager, or winners are managed wallets; bounded by TB5. |
| Compromised single admin signer | One of three signers; cannot meet the threshold alone. | Stepping stone to the quorum. |
| Compromised or coerced admin quorum | Full administrative control: pause, fees, bindings, upgrade (instant while the timelock is zero). | Theft by replacing contract logic. |
| Token issuer | Can freeze or claw back balances under the asset's policy. | Compliance action, not malice; treated as an availability event. |
| Infrastructure providers (RPC, Didit) | Outages, stale or wrong data, forged inbound messages. | Failure or compromise of a third party. |

**Assumptions.** The model relies on the following; a threat that requires one of them to fail is out of scope unless listed.

- The Soroban host behaves as specified: `require_auth` verifies a matching authorization entry, re-entrant contract invocation is refused, execution is deterministic, and resource limits are enforced.
- Stellar consensus provides finality; there are no reorganizations.
- Ed25519 signatures cannot be forged, and users control their own keys. Key loss is the user's risk except where the design makes it everyone's (DoS.9).
- Supported tokens are standard Stellar asset contracts with conventional transfer semantics; the supported-token list is curated by the admin.
- The admin signers' machines are not compromised simultaneously (custody policy).
- Users who choose a managed wallet accept platform custody of that wallet's key; the platform's obligation is the key-handling controls in Spoof.10.
- Judging is an off-chain process. The contracts guarantee custody and the published floors, not the fairness of a selection.

### 1.8 Data stores

| Store | Location | Contents | Sensitivity |
|---|---|---|---|
| Events contract — instance storage | Stellar ledger | Admin, pending admin, fee account, fee rate, paused flag, deployment sequence, profile address, version, pending upgrade, migration cursor, supported-token index | Public. Admin-only writes. TTL is extended on every admin write and on event reads. |
| Events contract — persistent storage | Stellar ledger | Event records (including prize floors), manager and pending manager, applicants, contributors, and winners as per-element keys, submissions keyed by (event, address, slot), prize awards keyed by (event, position), owed total, unclaimed-prize count, claim-window expiry, contributor amounts, non-owner contribution total, milestone-claimed flags, cancellation state | Public by design; pseudonymous addresses only. Each event read extends the record's TTL. |
| Events contract — temporary storage | Stellar ledger | Idempotency markers keyed by (authorizing address, op_id) | Public. Lives for at least the network's minimum temporary TTL (17,280 ledgers, about one day on mainnet). |
| Profile contract | Stellar ledger | `Profile{bootstrapped_at, reputation}`, per-token earnings, admin and binding configuration, idempotency markers keyed by (domain, op_id) where the domain is the events contract or the user | Public; pseudonymous. |
| Contract token balance | Stellar ledger (Stellar Asset Contract) | All escrow for all events in one balance; per-event accounting lives in `remaining_escrow` and `EventOwedTotal` | Critical. Released only by contract logic. |
| PostgreSQL | Boundless servers | Accounts, drafts, escrow operation log, credit ledger, KYC status (session identifier and status only), audit log, managed-wallet private keys (envelope-encrypted) | Critical for the encrypted key material; medium otherwise. No identity documents. |
| Didit | Didit infrastructure | Identity documents, biometrics, decisions | High; held exclusively by Didit. |
| Admin signer keys | Three signers on separate machines | Wallet keys (hardware-key upgrade scheduled by the custody policy) | Critical. |
| Backend environment | Server environment variables | Wallet master encryption key, JWT secret, Didit key, transaction-building configuration | Critical (master key). |

### 1.9 Key parameters

| Parameter | Value |
|---|---|
| Maximum fee | 1,000 bps (10%) |
| Upgrade timelock | 0 ledgers at present (EoP.10); target 17,280 (about one day) |
| Upgrade proposal expiry | 518,400 ledgers (about 30 days) |
| Admin rotation expiry | 120,960 ledgers (about 7 days) |
| Manager proposal expiry | 17,280 ledgers (about one day) |
| Profile events-binding rotation | 17,280-ledger timelock, 120,960-ledger expiry |
| Prize claim window | 90 days from the most recent selection |
| Winners per `select_winners` call | 50 |
| Refunds per `process_cancel_batch` call | 25 |
| Read page size | 100 entries |
| Minimum contribution | 10 tokens (100,000,000 stroops at 7 decimals) |
| Title length | 120 bytes |
| Submission content URI length | 256 bytes (the event-level content URI set at creation is not capped) |
| Events per `migrate_events` call | 8 |
| Applicant, contributor, submission caps | None; each entry is its own ledger entry paid for by the writer |

---

## 2. What can go wrong?

### 2.1 STRIDE reminders

| Threat | Definition | Question |
| --- | --- | --- |
| **S**poofing | The ability to impersonate another user or system component to gain unauthorized access. | Is the user who they say they are? |
| **T**ampering | Unauthorized alteration of data or code. | Has the data or code been modified in some way? |
| **R**epudiation | The ability for a system or user to deny having taken a certain action. | Is there enough data to prove the user took the action if they were to deny it? |
| **I**nformation Disclosure | The over-sharing of data expected to be kept private. | Is there anywhere where excessive data is being shared, or controls are not properly in place to protect private information? |
| **D**enial of Service | The ability for an attacker to negatively affect the availability of a system. | Can someone, without authorization, impact the availability of the service or business? |
| **E**levation of Privilege | The ability for an attacker to gain additional privileges and roles beyond what they were initially granted. | Are there ways for a user, without proper authentication and authorization, to gain access to additional privileges? |

### 2.2 Threat table

Step numbers refer to the on-chain flow in Section 1.4; TB numbers to Section 1.6. Each threat is rated for **likelihood** (L: how easily the threat can be attempted, given who must act and what access they need) and **impact** (I: what is lost if it succeeds), assessed for the design *before* the treatment in Section 3 is applied; the Status column in Section 3 records the residual position. Scales: L = Low / Medium / High; I = Low (cosmetic or easily reversed), Medium (integrity of non-financial data, availability of one flow, or privacy of off-chain data), High (loss or lock of funds for one event or one party), Critical (loss or lock of escrow across events, or control of a contract). Priority combines the two: **P1** critical impact with medium or high likelihood, or high impact with high likelihood; **P2** critical impact with low likelihood, high with medium, or medium with high; **P3** high with low, medium with medium, or low with high; **P4** everything lower.

#### Spoofing

| ID | Threat | Affected component | L | I | Priority |
|---|---|---|---|---|---|
| Spoof.1 | (Steps 2, 4, 5, 6) A caller passes another user's address as `owner`, `from`, or `applicant` in `create_event`, `add_funds`, `apply_to_bounty`, or `submit` to act, spend, or anchor content on their behalf. | `boundless-events` | H | C | P1 |
| Spoof.2 | (Step 9) Key material that satisfies the crowdfunding co-authorization is obtained by an attacker, for example through a backend compromise, and used to release milestones early. | `claim_milestone`, admin custody | L | H | P3 |
| Spoof.3 | A forged Didit webhook approves a user's KYC. | Backend webhook endpoint | M | M | P3 |
| Spoof.4 | A stolen staff session token is replayed against the admin portal without step-up credentials. | Admin portal, backend admin API | M | M | P3 |
| Spoof.5 | (Step 11) A single signer submits an admin transaction claiming to be the multi-signature account. | Admin account, contract admin functions | L | C | P2 |
| Spoof.6 | (Step 8) An attacker calls `claim_prize` for a position awarded to someone else, or a manager "claims on behalf of" a winner into a different wallet. | `claim_prize` | H | H | P1 |
| Spoof.7 | (Step 3) Management of an event is assigned to an address that never consented, making it appear that a known organization endorses or operates the event. | Manager delegation | M | L | P4 |
| Spoof.8 | (Steps 5, 8, 9) An arbitrary account or contract calls `bootstrap`, `bump_reputation`, `slash_reputation`, or `register_earnings` on the profile contract pretending to be the events contract. | `boundless-profile` (TB6) | H | M | P2 |
| Spoof.9 | (Step 11) An admin signer's software-wallet key is exfiltrated from that signer's machine and combined with a second compromised or coerced signer to meet the 2-of-3 threshold. | Admin custody | L | C | P2 |
| Spoof.10 | (Steps 2–10) **Managed-wallet custody.** An attacker who compromises the backend and obtains the wallet master encryption key can decrypt every managed wallet's private key and act as any managed owner, manager, builder, or winner: select attacker-controlled winners, cancel an event and withdraw the refund from the managed owner wallet, or claim prizes. | Custodial wallet service (TB5) | L | C | P2 |

#### Tampering

| ID | Threat | Affected component | L | I | Priority |
|---|---|---|---|---|---|
| Tamp.1 | (Steps 2, 7) An owner lowers the advertised prizes after builders have committed work. | `boundless-events` | M | H | P2 |
| Tamp.2 | (Step 7) A `select_winners` call whose amounts exceed what the pool holds, drawing on escrow that belongs to other events (all events share one token balance). | Escrow accounting | M | C | P1 |
| Tamp.3 | (Step 6) A builder overwrites another builder's submission, or a third party withdraws it. | Submissions | H | M | P2 |
| Tamp.4 | A compromised RPC node returns forged event data to the indexer. | Backend indexer | L | M | P4 |
| Tamp.5 | (Step 11) The WASM hash is swapped between `propose_upgrade` and `apply_upgrade`, or an unreviewed hash is applied. | Upgrade flow | L | C | P2 |
| Tamp.6 | (Step 11) A partially signed multi-signature transaction is altered in transit. | Signing workflow | L | C | P2 |
| Tamp.7 | Integer overflow or underflow in index and count bookkeeping corrupts a registry or an award anchor. | Storage helpers, `select_winners` | L | H | P3 |
| Tamp.8 | (Step 7) **Over-promising across batches.** `remaining_escrow` only drops when a winner claims. Without a reservation, a second `select_winners` call would see funds an earlier winner is still owed and promise them again; the last claimant would find an empty pool. | `select_winners` | M | H | P2 |
| Tamp.9 | (Step 7) A manager awards a position below its published floor, so the published prize table is no longer a guaranteed minimum. | `select_winners` | M | M | P3 |
| Tamp.10 | (Step 11) **Storage-layout migration.** Event records written under the earlier percentage-based distribution cannot be decoded by the current record type. A migration that stops part-way, stamps the version early, or miscomputes owed totals for already-awarded winners could strand or double-count funds. | `migrate_events`, `migrate` | M | C | P1 |
| Tamp.11 | (Steps 8, 9) Re-entrancy through the token contract during a release, double-paying a claim. | Escrow release paths | L | C | P2 |
| Tamp.12 | (Step 7) A duplicate position inside one batch, or a position re-awarded in a later batch, pays the same prize twice. | `select_winners` | M | H | P2 |
| Tamp.13 | (Steps 2–10) A backend retry, or an attacker replaying a captured request, executes the same logical operation twice: two deposits or two applications for one intent. | Idempotency layer | H | H | P1 |
| Tamp.14 | (Steps 5, 8, 9) **Cross-user child op_id collision.** The child op_ids the events contract derives for profile calls are keyed by the parent op_id and the profile address but not by the originating user, and all events-originated markers share one domain in the profile contract. An attacker who learns a victim's pending parent op_id can pre-mark its bootstrap child through `apply_to_bounty`, so that the victim's `claim_milestone` reverts or the victim's `claim_prize` pays out without bootstrapping the profile (reputation lost). | Idempotency layer (TB6) | L | M | P4 |
| Tamp.15 | (Steps 6, 10) The manager cancels a hackathon or bounty after builders have submitted work but before any selection, reclaiming the full budget. | `start_cancel` | M | M | P3 |
| Tamp.16 | (Step 7) A multi-release selection names the same recipient at two positions. Only positions are de-duplicated; `claim_milestone` follows the last anchor for that recipient, so the first award's amount stays reserved with no claim path. | `select_winners`, `claim_milestone` | L | M | P4 |
| Tamp.17 | (Steps 6, 7) A builder overwrites the content of an already-judged submission slot after winners are selected; the stored record keeps the original submission time, so the rewrite is indistinguishable from the original in storage. | `submit` | M | M | P3 |

#### Repudiation

| ID | Threat | Affected component | L | I | Priority |
|---|---|---|---|---|---|
| Repud.1 | (Step 7) A manager disputes having selected the winners on record. | `select_winners` | M | M | P3 |
| Repud.2 | (Step 8) A winner claims they never received their prize. | Stellar ledger | M | M | P3 |
| Repud.3 | A staff member disputes a KYC override. | Backend audit log | L | M | P4 |
| Repud.4 | (Step 10) A contributor claims they were never refunded, or a manager denies having started a cancellation. | Cancellation flow | M | M | P3 |
| Repud.5 | (Step 9) The platform denies having co-authorized a crowdfunding milestone release. | Transaction envelope | L | M | P4 |
| Repud.6 | (Step 3) A manager denies having accepted delegation, or an owner denies having delegated, after a contested selection. | Manager delegation | L | M | P4 |
| Repud.7 | (Step 6) A builder disputes what they submitted, or an owner claims a submission pointed at different content than what was judged. | Submission anchors | M | M | P3 |

#### Information Disclosure

| ID | Threat | Affected component | L | I | Priority |
|---|---|---|---|---|---|
| Info.1 | All contract storage is publicly readable: participant addresses, prize floors, awards, earnings, reputation. | Stellar ledger | H | L | P3 |
| Info.2 | A Boundless database breach exposes identity documents. | Backend database | L | H | P3 |
| Info.3 | Signing key material or transaction-building secrets are exposed through logs, environment dumps, or configuration serialization. | Backend environment | L | M | P4 |
| Info.4 | Staff session tokens or TOTP secrets are exposed from a compromised admin workstation. | Admin portal | L | M | P4 |
| Info.5 | A Didit webhook payload is logged in plain text. | Backend logging | M | M | P3 |
| Info.6 | The fee account key is exposed, allowing fee diversion. | Fee account custody | L | H | P3 |
| Info.7 | (Step 7) Winner selection is visible on-chain the moment it lands, before the owner's announcement. | `boundless-events` | H | L | P3 |

#### Denial of Service

| ID | Threat | Affected component | L | I | Priority |
|---|---|---|---|---|---|
| DoS.1 | (Steps 5, 6) Bot accounts flood applications or submissions to a popular event to lock out legitimate builders or bloat state. | Registries | H | M | P2 |
| DoS.2 | (Step 10) A cancellation with thousands of contributors exceeds per-transaction ledger limits and can never settle. | Cancellation flow | M | C | P1 |
| DoS.3 | Unauthenticated request floods exhaust backend rate limits. | Backend API | H | M | P2 |
| DoS.4 | The RPC provider becomes unavailable. | RPC dependency | M | M | P3 |
| DoS.5 | Fee-market spam delays legitimate transactions. | Stellar network | M | L | P4 |
| DoS.6 | Webhook floods exhaust KYC processing. | Backend webhook handler | M | L | P4 |
| DoS.7 | (Steps 5, 9) **Profile coupling.** `apply_to_bounty` and `claim_milestone` call the profile contract without `try_`; if the profile contract is paused or mis-bound, bounty applications and every grant or crowdfunding milestone claim revert. `claim_prize` is decoupled. | `boundless-events` (TB6) | L | H | P3 |
| DoS.8 | (Steps 7, 8, 10) Winners never claim, or a winner's wallet is lost; the pool cannot be cancelled while a prize is inside its window, so funds are stuck. | `claim_prize`, `start_cancel` | M | M | P3 |
| DoS.9 | (Steps 3, 7, 10) **Manager key loss.** Once a manager has accepted, `select_winners`, `start_cancel`, `propose_manager`, and `cancel_pending_manager` all require the current manager's signature. If that key is lost, the owner cannot select winners, cancel, or reassign, and there is no admin override. | Manager delegation (TB7) | M | H | P2 |
| DoS.10 | (Step 11) A global pause halts every user-facing state-changing path, including `claim_prize`, `process_cancel_batch`, and `finalize_cancel`, freezing funds in flight; time-based windows keep running while paused. | Admin controls | L | H | P3 |
| DoS.11 | Ledger state archival: a dormant event's persistent entries expire, making the event unreadable until restored. | Persistent storage TTL | M | L | P4 |
| DoS.12 | (Step 11) A storage migration on a deployment with real history cannot fit in one transaction; attempted one-shot, it aborts and leaves every event undecodable. | `migrate_events` | M | C | P1 |
| DoS.13 | (Step 6) Grant builders cannot submit: `submit` requires a prior application for grant events, but the application entry point accepts only bounties, so every grant submission fails with `ApplicantNotApplied`. | `submit`, `apply_to_bounty` | H | L | P3 |
| DoS.14 | (Step 10) **Refund recipient that cannot receive the token.** `process_cancel_batch` transfers to each contributor in index order without a skip path. A contributor who has removed their trustline, or whose trustline the issuer has frozen, makes every batch containing their index revert, so `finalize_cancel` can never run and every other contributor's refund and the owner's residual stay locked. | Cancellation flow | M | H | P2 |
| DoS.15 | (Steps 2, 4, 9) The fee account lacks, or loses, a trustline to a supported token; every deposit and every crowdfunding release in that token then reverts until the admin rotates the fee account. | Fee account configuration | L | H | P3 |

#### Elevation of Privilege

| ID | Threat | Affected component | L | I | Priority |
|---|---|---|---|---|---|
| EoP.1 | (Step 7) A builder calls `select_winners` directly, naming themselves. | `select_winners` | H | C | P1 |
| EoP.2 | A read-only staff member invokes a KYC override. | Backend authorization guards | L | M | P4 |
| EoP.3 | (Step 9) An owner claims a crowdfunding milestone without the admin co-authorization. | `claim_milestone` | M | H | P2 |
| EoP.4 | A user session token is presented on admin routes. | Backend staff guard | M | M | P3 |
| EoP.5 | An attacker calls `bootstrap_self` with another user's address to squat a profile or its op_id space. | `boundless-profile` | H | L | P3 |
| EoP.6 | (Step 11) An event owner calls `set_admin`, `propose_upgrade`, `register_supported_token`, or `set_fee_account`. | Admin functions | H | C | P1 |
| EoP.7 | (Step 9) A crowdfunding milestone is co-authorized for a claim the owner did not initiate. | `claim_milestone` | L | M | P4 |
| EoP.8 | (Steps 3, 7, 10) A manager escalates to owner powers: redirects refunds to themselves, changes the owner, claims milestones, or drains the pool by awarding themselves. | Manager delegation (TB7) | M | H | P2 |
| EoP.9 | (Step 9) The owner passes an arbitrary `recipient` or an inflated amount to `claim_milestone` to pay a non-winner or overpay. | `claim_milestone` | M | H | P2 |
| EoP.10 | (Step 11) **Zero upgrade timelock.** The upgrade timelock is configured at 0 ledgers on both contracts. A compromised or coerced 2-of-3 quorum could propose and apply a WASM upgrade in a single step, leaving `cancel_pending_upgrade` and off-chain monitors no window to react. | Upgrade flow, both contracts | L | C | P2 |
| EoP.11 | A non-admin calls `admin_slash_reputation`, or the events contract's slash path is reached from outside the events contract. | `boundless-profile` | M | M | P3 |
| EoP.12 | (Step 11) A non-admin calls `migrate_events` or `migrate` to rewrite event records or stamp a version. | Migration functions | H | C | P1 |
| EoP.13 | (Step 1) The profile contract's events binding is rotated to an attacker-controlled contract that can then mint reputation and earnings at will. | `boundless-profile`, binding rotation | L | H | P3 |
| EoP.14 | (Steps 7, 10) **Cancellation of an awarded grant.** The cancellation guard protects only single-release prizes inside their window. A manager can cancel a grant after grantees have been selected and before milestones are paid; the reservation is released and the awarded funds return to the owner and contributors. | `start_cancel` (TB7) | M | H | P2 |
| EoP.15 | (Steps 7, 9) **Unbounded reputation minting.** `reputation_bump` is chosen by the manager or owner and forwarded to the profile contract unchecked. Any wallet can create a minimal bounty, award itself with `reputation_bump = u32::MAX`, claim, and hold the maximum reputation score. | `select_winners`, `claim_milestone`, `boundless-profile` | H | M | P2 |
| EoP.16 | (Step 1) The admin re-points the events contract's profile binding in a single step (`set_profile_contract` has no timelock), instantly changing every derived child op_id and, through DoS.7, halting bounty applications and milestone claims if the new binding is broken. | `set_profile_contract` (TB6) | L | M | P4 |

---

### 2.3 Priority summary

**Priority 1 (10)** — the controls whose failure would be catastrophic; each is mitigated and tested, and these are the controls the audit should verify first: Spoof.1 (acting under another user's address); Spoof.6 (claiming another winner's prize); Tamp.2 (spending more than the pool holds); Tamp.10 (storage migration correctness); Tamp.13 (replayed operations); DoS.2 (large cancellations that cannot settle); DoS.12 (migration that cannot fit in one transaction); EoP.1 (non-manager selects winners); EoP.6 (non-admin calls admin functions); EoP.12 (non-admin rewrites records through migration).

**Priority 2 (21)** — Spoof.5 (single signer as the multi-signature account); Spoof.8 (forged cross-contract calls into the profile); Spoof.9 (admin signer key exfiltration); Spoof.10 (managed-wallet custody); Tamp.1 (lowering advertised prizes); Tamp.3 (overwriting another builder's submission); Tamp.5 (unreviewed or swapped upgrade hash); Tamp.6 (multi-signature envelope altered in transit); Tamp.8 (over-promising the pool across batches); Tamp.11 (re-entrancy during release); Tamp.12 (paying one position twice); DoS.1 (registry spam); DoS.3 (backend request floods); DoS.9 (manager key loss (open)); DoS.14 (refund recipient blocks cancellation (open)); EoP.3 (crowdfunding claim without admin co-authorization); EoP.8 (manager escalation to owner powers); EoP.9 (milestone paid to a non-winner or overpaid); EoP.10 (zero upgrade timelock (open)); EoP.14 (cancellation of an awarded grant (open)); EoP.15 (unbounded reputation minting (open)).

Priority 3 and 4 threats are listed in the tables above. Every open item (Section 4.1) is Priority 2 or lower; no Priority 1 threat is open.

## 3. What are we going to do about it?

Status values: **Mitigated** — a control in the contracts or in operating procedure addresses the threat. **Accepted** — the residual risk is documented and carried. **Open** — a change is scheduled, or auditor input is requested, and the item is disclosed for the audit.

| Threat | Treatment | Status |
|---|---|---|
| **Spoof.1** | Every state-changing entry point calls `require_auth()` on the address it acts for: `params.owner`, `from`, `applicant`. The host verifies the matching authorization entry at the point `require_auth()` is invoked, and every entry point invokes it before any persistent state is written. Inside the events contract, idempotency markers are keyed by the authorizing address, so a permissionless caller cannot pre-mark an op_id and block a privileged one (the profile-side exception is Tamp.14). | Mitigated |
| **Spoof.2** | The contract has no separate co-signer role: `claim_milestone` for crowdfunding calls `require_auth()` on the admin address, which on mainnet is a 2-of-3 account. A single key cannot satisfy it, and no server holds the quorum, so a backend compromise does not yield the co-authorization. The owner's signature is required in the same transaction and the recipient is the owner (crowdfunding's only winner record), so a compromised quorum changes timing, not destination. Planned hardening: introduce a dedicated validator role for crowdfunding co-authorization so that the admin quorum is not exercised in a routine flow. | Accepted |
| **Spoof.3** | HMAC-SHA256 over the raw body with the shared webhook secret; requests with a missing or invalid signature are rejected before deserialization. | Mitigated |
| **Spoof.4** | Short-lived staff sessions; sensitive routes require a fresh TOTP code; sessions can be invalidated centrally. | Mitigated |
| **Spoof.5** | Threshold weights are enforced by the Stellar protocol on the admin account; a single-signer envelope never reaches the contract. Signer configuration is verified by script before every governance operation. | Mitigated |
| **Spoof.6** | `claim_prize` takes no recipient parameter. The recipient is read from the prize award written at selection, cross-checked against the anchored winner record (recipient and position must both match), and must `require_auth()`. The transfer goes to that stored address only. | Mitigated |
| **Spoof.7** | Delegation is two-step: `propose_manager` by the current authority, then `accept_manager`, which calls `require_auth()` on the proposed address. Proposals expire after 17,280 ledgers; `cancel_pending_manager` withdraws them. `ManagerProposed` and `ManagerChanged` events make the timeline public. Planned hardening: `propose_manager` will emit a cancellation event when it replaces an existing proposal. | Mitigated |
| **Spoof.8** | The four mutation entry points call `require_auth()` on the events contract address stored in instance storage. For a contract address this succeeds only when that contract is the direct invoker, so no account or third contract can satisfy it. | Mitigated |
| **Spoof.10** | Managed keys are envelope-encrypted: a random data key per wallet, wrapped by a master key that exists only in the backend environment; the ciphertext is excluded from ORM reads globally and decrypted only on the signing path; master-key rotation is supported. Network fees are paid by a separate sponsor account, so managed wallets hold no XLM to drain. Every action a managed wallet takes is an on-chain event attributable to that wallet, and organizers may connect an external wallet so that high-value programs never depend on custody. Planned hardening: hold the master key in a KMS/HSM, require external-wallet signing above a per-event value threshold, and alert on anomalous managed-wallet activity. The residual custodial risk is carried and disclosed. | Accepted |
| **Spoof.9** | Signers hold keys on separate machines under a written custody policy; the medium and high thresholds require two signers; every envelope is simulated and inspected by each signer before signing. The policy schedules a hardware-key upgrade at defined value and incident triggers. Until then, the residual risk of two simultaneous signer compromises is carried. | Accepted |
| **Tamp.1** | Prize floors are written once at `create_event` and never mutated; there is no setter. The full budget plus fee is pulled in the create transaction, so the pool exists before any builder sees the event. Selection can pay at or above a floor, never below (Tamp.9). | Mitigated |
| **Tamp.2** | `select_winners` sums the batch with checked arithmetic, adds the already-reserved `EventOwedTotal`, and rejects with `InsufficientEscrow` if the committed total exceeds `remaining_escrow`. `claim_prize` and `claim_milestone` re-check `amount <= remaining_escrow` at release. Because per-event accounting is enforced on every release path, one event cannot spend another's share of the shared token balance. Covered by the escrow fee-math, prize-claim, and grant test suites, which assert recipient and fee-account balance deltas. | Mitigated |
| **Tamp.3** | Submissions are keyed by (event, applicant, slot); `submit` and `withdraw_submission` both `require_auth()` on the applicant. The applicant recorded inside the stored submission is the authenticated address. | Mitigated |
| **Tamp.4** | The indexer filters on the known contract identifiers and reconciles against `get_event` reads before acting; the RPC endpoint is a dedicated managed node. | Mitigated |
| **Tamp.5** | `propose_upgrade` stores the hash on-chain; `apply_upgrade` reads only the stored hash and is gated by admin authorization, a 30-day expiry, and the timelock. The upgrade runbook requires reproducing the WASM hash from source with a locked build and simulating every admin envelope before signing. The on-chain review window this control relies on is absent while the timelock is zero (EoP.10). | Mitigated |
| **Tamp.6** | Envelopes are built offline, simulated, and inspected by each signer before signing; signers hold keys on separate machines (Spoof.9). | Mitigated |
| **Tamp.7** | The applicant index uses `checked_sub` with typed errors; the submission counters, unclaimed-prize count, and supported-token index use saturating arithmetic that clamps at zero; the workspace builds with `overflow-checks = true` in release, so any remaining bare arithmetic traps rather than wraps. One bare addition remains in the anchor-index computation of `select_winners`; it is unreachable in practice (it would require 2³² winner records) and traps under the release profile. Planned hardening: convert it to saturating arithmetic. | Mitigated |
| **Tamp.8** | The contract maintains `EventOwedTotal`: `select_winners` reserves the batch total plus the amount already owed and rejects if that exceeds `remaining_escrow`; `claim_prize` and `claim_milestone` decrement the owed and remaining balances together with checked arithmetic, failing closed if the reservation has drifted. `start_cancel` releases the reservation only after confirming that no single-release prize is still inside its claim window; multi-release awards carry no such guard (EoP.14). | Mitigated |
| **Tamp.9** | `create_event` requires every floor to be positive and their sum to be at most the budget. `select_winners` rejects any amount below its position's floor with `InvalidDistribution`. Positions without a floor are payable at any positive amount, by design. | Mitigated |
| **Tamp.10** | `migrate_events` walks a stored cursor from the deployment's first event to the latest, up to 8 events per call, admin-only; `migrate` refuses to stamp the version while any event remains unconverted. Records in the earlier layout are decoded with a frozen copy of their original type, and floors are derived as `total_budget × percent / 100`, which reproduces exactly what each position would have been paid. Owed totals are reconstructed for awards recorded in the earlier layout, which carried no reservation. The procedure has been rehearsed on testnet (45 events across 6 calls). Pending operational action: two mainnet event records remain in the earlier layout and cannot be decoded until `migrate_events` runs; both are completed events holding zero escrow, so no funds are affected. The conversion, followed by `migrate` on both contracts, is scheduled as part of the next administrative operation. | Mitigated |
| **Tamp.11** | The Soroban host forbids re-entrant contract invocation, so a token cannot call back into the events contract mid-release; tokens are also admin-approved. `claim_prize` additionally writes all state before the transfer. `claim_milestone` transfers before it marks the milestone claimed, which is safe under the host's re-entrancy prohibition. Planned hardening: reorder `claim_milestone` to record the claim before transferring, matching `claim_prize`. | Mitigated |
| **Tamp.12** | Duplicate positions within a batch are rejected. Across batches the per-position prize award record is the replay lock for single-release events; multi-release events accept exactly one `select_winners` call for the life of the event, so un-awarded headroom in a grant pool can leave only through cancellation. `claim_prize` rejects already-paid awards with `PrizeAlreadyClaimed`. | Mitigated |
| **Tamp.13** | Every fund-moving and registry-mutating call carries a 32-byte `op_id`; a marker keyed by (authorizing address, op_id) in temporary storage rejects repeats with `OpAlreadySeen`. Manager rotation and admin functions rely on Soroban authorization nonces alone. On mainnet a temporary entry lives at least 17,280 ledgers (about one day), covering the backend's retry horizon; beyond that, the original signed authorization has expired, so a replay needs a fresh signature from the user, which is a new intent by definition. Planned hardening: `process_cancel_batch` will reject `max_refunds = 0` so that a no-op call cannot consume a contract-domain op_id. | Mitigated |
| **Tamp.14** | Child op_ids are derived with SHA-256 over the parent, a tag, an index, and the profile contract identifier, which makes them collision-resistant across tags and indices for one parent but does not bind them to the originating user. Exploitation requires learning a victim's pending parent op_id (generated randomly by the backend) within the one-day marker lifetime, and the impact is a reverted claim or a lost reputation update, never a loss of funds. Planned remediation: include the events-side authorizing address in the child preimage. | Open |
| **Tamp.15** | Cancellation before selection is a deliberate owner right: the pool is the owner's money until it is awarded, and there is no on-chain notion of a submission deadline. The platform's terms and the public `EventCancelled` record are the controls; builders can see on-chain that a pool was withdrawn. | Accepted |
| **Tamp.16** | Manager-inflicted and recoverable through cancellation, but a validation gap. Planned remediation: reject duplicate recipients within a multi-release selection. | Open |
| **Tamp.17** | The ordered `Submitted` events show every rewrite, so the change is provable from the ledger, but the stored record does not carry it. Planned remediation: refuse `submit` to a slot once a selection has been recorded for the event, and record the last update time on the submission. | Open |
| **Repud.1** | `WinnersSelected` plus one winner record and one prize award per position are written in the manager-signed transaction; the manager's authorization entry is in the envelope on the public ledger. Planned hardening: emit one award event per position so that the per-winner linkage does not depend on reading storage in the same ledger. | Mitigated |
| **Repud.2** | `WinnerPaid{event_id, recipient, position, amount}` is emitted alongside the token transfer; both are permanently on-chain, and the payment timestamp is readable through `get_winner_at`. | Mitigated |
| **Repud.3** | Append-only audit log with staff identity, action, target, timestamp, and reason, written in the same database transaction as the change. | Mitigated |
| **Repud.4** | `ContributorRefunded` and `OwnerResidualRefunded` are each backed by a token transfer; `EventCancelled` marks the final state. The manager's authorization entry for `start_cancel` is in the envelope. Planned hardening: emit a `CancellationStarted` event when an event enters `Cancelling`, and emit a zero-amount refund event for contributors whose pro-rata share rounds to zero. | Mitigated |
| **Repud.5** | The admin authorization entries are part of the signed transaction envelope and attributable on any explorer. | Mitigated |
| **Repud.6** | `ManagerProposed`, `ManagerChanged`, and `PendingManagerCancelled` events are emitted; acceptance requires the proposed manager's own signature, so a denial is refutable from the ledger. | Mitigated |
| **Repud.7** | `Submitted{event_id, applicant, slot, content_uri}` and `SubmissionWithdrawn` events are emitted. The anchor binds the content, rather than only a location, only when `content_uri` is content-addressed; the contract stores whatever URI the builder signs. Planned control: the backend will pin submission content and require a content identifier or an embedded hash in the URI. | Accepted |
| **Info.1** | By design for a trustless settlement layer. Addresses are pseudonymous; all rich content stays off-chain behind `content_uri`. Users are told that addresses and amounts are public. | Accepted |
| **Info.2** | The database holds only Didit's opaque session identifier and a status value. Documents and biometrics never reach Boundless. | Mitigated |
| **Info.3** | Secrets are environment variables only, read at transaction-build time, never logged or serialized; rotation is a redeploy (master-key rotation re-wraps the per-wallet data keys). No server-held secret can act as the admin; the master encryption key is the one secret whose exposure matters for escrow (Spoof.10). | Mitigated |
| **Info.4** | Short-lived sessions, fresh TOTP for sensitive actions, dedicated browser profiles; hardware-backed staff keys planned. | Mitigated |
| **Info.5** | The handler logs the session identifier and status only; production log levels exclude request bodies. | Mitigated |
| **Info.6** | The fee account is under the same 2-of-3 custody as the admin; `set_fee_account` is admin-only and emits `FeeAccountUpdated` for monitoring. | Mitigated |
| **Info.7** | Inherent to public ledgers. The backend batches selection and announcement so the gap is minutes; the owner controls when the selection transaction is submitted. | Accepted |
| **DoS.1** | Applicants, contributors, and submissions are per-element persistent entries whose write and rent are paid by the submitter's own transaction, so spam funds its own storage and cannot fill a shared entry. No state-changing path iterates a participant set in one transaction: refunds are cranked in batches of 25 and selection takes an explicit list of at most 50. Winner records are scanned in full by `claim_milestone`, the multi-release branch of `select_winners`, and `migrate_events`, but those records are written only by the manager. Reads page at 100 entries. At the API layer, applying and submitting require a KYC-approved session and, where configured, an off-chain credit cost. | Mitigated |
| **DoS.2** | `start_cancel` is O(1): it reads the running non-owner contribution total instead of scanning contributors, snapshots the refund arithmetic, and flips the event to `Cancelling`. `process_cancel_batch` and `finalize_cancel` are permissionless cranks with contract-domain idempotency keys, so a missing manager cannot strand an in-progress refund. Tested at 220 contributors with a constant start footprint. Current mainnet limits (200 read and 200 write entries per transaction) leave headroom. | Mitigated |
| **DoS.3** | Global per-IP throttling plus infrastructure-level DDoS protection. | Mitigated |
| **DoS.4** | Queue-buffered submissions with exponential backoff and distributed locks limit the impact of an outage; a failover RPC endpoint is planned. The single-provider dependency is carried until then. | Accepted |
| **DoS.5** | Soroban resource fees price out spam; the backend simulates before submitting and refuses operations beyond configured limits. | Mitigated |
| **DoS.6** | Separate rate limit, HMAC rejection before any write, IP allow-listing available. | Mitigated |
| **DoS.7** | The profile contract can be paused only by the admin quorum, its instance TTL is extended on every state-changing call and its persistent entries on every read, and `set_profile_contract` on the events side can re-point the binding. `claim_prize`, the release path with the most claimants, already uses `try_` calls so a profile fault cannot block a payout. Planned remediation: convert `apply_to_bounty` and `claim_milestone` to `try_` calls so that profile availability never gates a fund movement. Auditor input is requested on whether best-effort reputation updates are the appropriate trade-off. | Open |
| **DoS.8** | Prizes are claimable for 90 days from the most recent selection; later batches extend the window. During the window `start_cancel` is refused, so a manager cannot withdraw funds while a winner's claim is still valid; afterwards `start_cancel` releases the reservation and unclaimed amounts flow back through the normal refund branches (contributors first, then owner). Funds remain reachable provided a manager exists (DoS.9) and every refund recipient can still receive the token (DoS.14). | Mitigated |
| **DoS.9** | Controls in place: delegation is opt-in and two-step, proposals expire, the owner keeps `claim_milestone` authority for grants, and the cancellation cranks are permissionless once cancellation has started. There is no recovery when an accepted manager's key is lost before selection or cancellation. Auditor input is requested on three candidate designs: (a) the owner may `propose_manager` after a period of manager inactivity; (b) the owner may always revoke a manager while no selection has occurred; (c) admin-mediated reassignment with a timelock and event emission. Until one ships, owners are advised to delegate only to keys they can recover. | Open |
| **DoS.10** | Intended emergency brake; admin-only under 2-of-3; `Paused` and `Unpaused` events; the operations runbook requires a written incident record. Pausing never moves escrow, and admin entry points remain callable so the pause can be lifted. Time-based windows (the 90-day claim window, manager and admin proposal expiries) are not extended by a pause; a pause longer than a window would let it lapse. Planned hardening: extend the prize-claim expiry by the pause duration on `unpause`. | Accepted |
| **DoS.11** | Every `get_event` read extends the record's TTL when it drops below 86,400 ledgers (to about 90 days); mainnet's minimum persistent TTL for new entries is about 120 days. Instance storage is extended on every admin write. A truly dormant event can be restored by anyone with a `RestoreFootprint` operation; the backend indexer touches the events it tracks. | Accepted |
| **DoS.12** | `migrate_events` is paged over a persisted cursor and bounds the number of event records per call to 8; each event additionally costs one read per winner record (and one write per multi-release anchor), which is adequate while no event holds more winner records than the per-transaction entry limits allow. `migrate` blocks until the cursor reaches the latest event. | Mitigated |
| **DoS.13** | Fail-closed: no funds or state are affected, and grants proceed through owner-driven selection and milestone claims without on-chain submissions. Planned remediation: either add a grant application entry point or make `submit` reject grant events with a dedicated error. | Open |
| **DoS.14** | A contributor is refunded to the address they contributed from, and the token is admin-approved; the failure requires a contributor to act against their own refund, or an issuer freeze. Auditor input is requested on the remediation design: record a failed refund as claimable and continue the batch, or provide a permissionless skip that leaves the contributor's amount claimable later. | Open |
| **DoS.15** | Admin-inflicted and reversible with `set_fee_account`. The operations runbook requires verifying that the fee account holds a trustline for every supported token before registering the token or rotating the account; `FeeAccountUpdated` and `TokenRegistered` events are the monitoring hooks. | Accepted |
| **EoP.1** | `select_winners` calls `require_auth()` on the resolved manager: the accepted manager or, absent one, the owner. No other address can satisfy it. | Mitigated |
| **EoP.2** | Route-declared permissions are checked by a policy guard; sensitive routes additionally require a fresh TOTP code through an independent step-up guard. | Mitigated |
| **EoP.3** | `claim_milestone` calls `require_auth()` on the event owner and, for crowdfunding, on the admin; both must be present in the same transaction. The crowdfunding test suite asserts that the admin's authorization is demanded. | Mitigated |
| **EoP.4** | Separate staff token issuer and secret; user tokens fail the staff guard. | Mitigated |
| **EoP.5** | `bootstrap_self` calls `require_auth()` on the user and uses the user's own address as the op_id domain, so it cannot squat events-domain keys. Bootstrap is idempotent and creates an empty profile, so there is nothing to gain from bootstrapping someone else. | Mitigated |
| **EoP.6** | Every admin entry point begins with `require_auth()` on the admin address in instance storage; admin rotation is two-step with a 7-day expiry. The admin test suite asserts that authorization is genuinely required, not mocked. | Mitigated |
| **EoP.7** | The owner's signature is required in the same transaction as the admin co-authorization, the recipient is the owner, and the amount is derived from the pool. The backend assembles the transaction only for an authenticated session that owns the event. | Mitigated |
| **EoP.8** | The manager can select, cancel, and rotate. It cannot change the owner (there is no setter), receive refunds (routed to the stored owner and contributor addresses), claim milestones (owner-authorized), or alter floors. A manager, or an owner acting as manager, can award any address including itself: judging is off-chain by design, and the owner chooses whom to delegate to. `ManagerChanged` lands on-chain before any selection, so the delegation is auditable. | Accepted |
| **EoP.9** | The recipient must already hold a winner record with no milestone; the amount is computed from that award (grants) or from `remaining_escrow / milestones_left` (crowdfunding), never from a caller-supplied value; each (recipient, milestone) pair is claimable once; the milestone index must be within range. | Mitigated |
| **EoP.10** | The timelock is configured at zero on both contracts while mainnet holds live escrow. Restoring it to 17,280 ledgers on both contracts is the next administrative operation and will precede audit fieldwork. Compensating controls meanwhile: the 2-of-3 threshold and the runbook's simulate-and-hash-verify step, which protect against mistakes rather than against a compromised quorum. | Open |
| **EoP.11** | `admin_slash_reputation` requires admin authorization and a non-empty reason, and emits `AdminReputationSlashed`; `slash_reputation` is reachable only through the events-contract domain (Spoof.8). | Mitigated |
| **EoP.12** | `migrate_events` and `migrate` both require admin authorization; the cursor only advances; `migrate` is one-shot per version. | Mitigated |
| **EoP.13** | The first binding is single-step at deployment; rotation is `propose_events_contract`, a 17,280-ledger timelock, then `accept_events_contract` within 7 days, admin-only, with events emitted for monitoring. The timelock is only as strong as the profile contract's upgrade timelock, so this control is fully effective once EoP.10 closes. | Mitigated |
| **EoP.14** | Grantees are named on-chain (`WinnersSelected`, winner records) and cancellation is public (`EventCancelled`), so an un-awarding is visible and attributable, and the funds return to the accounts that supplied them rather than to the manager. Auditor input is requested on extending the cancellation guard to multi-release awards with unpaid milestones, and on whether such a guard should have a time bound. | Open |
| **EoP.15** | Reputation is informational and carries no on-chain privilege; earnings are recorded from actual transfers and cannot be inflated the same way. Planned remediation: cap `reputation_bump` per award in the events contract, and cap `delta` per call in the profile contract. | Open |
| **EoP.16** | Admin-only, emitted for monitoring, and reversible in one step. The profile-side binding is timelocked (EoP.13) because that side can mint reputation; the events-side binding cannot move funds. The asymmetry is carried as a residual risk; a two-step rotation on the events side is a candidate hardening. | Accepted |

---

## 4. Did we do a good job?

**Has the data-flow diagram been referenced since it was created?**
Yes. Drawing the diagram made two boundaries explicit that the code alone did not: the boundary between owner and manager inside a single event (TB7), and the boundary between the two contracts (TB6). Walking TB7 produced Spoof.7, Repud.6, DoS.9, EoP.8, and EoP.14; walking TB6 produced Spoof.8, Tamp.14, DoS.7, EoP.13, and EoP.16. Figure 2 was drawn after the threat tables and immediately exposed that the cancellation guard is evaluated for only one of the two release kinds.

**Did the STRIDE model uncover any new design issues or concerns that had not been previously addressed or thought of?**
Yes. The following were not written down anywhere before this exercise:

1. **Manager key loss locks the pool (DoS.9).** Delegation deliberately removes the owner's ability to act once a manager accepts, so that an owner cannot override a neutral judge. The cost is that nothing can recover an event whose manager disappears. Three candidate designs are listed for audit input.
2. **A refund recipient that cannot receive the token blocks every other refund (DoS.14).** Batched refunds have no skip path, so one removed or frozen trustline can leave an event in `Cancelling` indefinitely. Two remediation designs are listed for audit input.
3. **Awarded grants can be cancelled (EoP.14).** The cancellation guard was written for pull-model prizes and does not cover multi-release awards.
4. **Reputation can be minted without bound (EoP.15).** The reputation bump is caller-chosen and uncapped. Planned remediation: per-award and per-call caps.
5. **Child op_ids are not bound to the user (Tamp.14).** Planned remediation: include the authorizing address in the derivation.
6. **Submissions can be rewritten after judging (Tamp.17)** and **a multi-release selection can strand a reservation by naming one recipient twice (Tamp.16).** Planned remediation for both.
7. **Grant submissions are unreachable (DoS.13).** Fail-closed, so not a security issue, but a dead path. Planned remediation: a grant application entry point or an explicit rejection.
8. **Profile coupling on fund-moving paths (DoS.7).** `claim_prize` treats profile updates as best-effort; `apply_to_bounty` and `claim_milestone` do not. Planned remediation: align them.
9. **Release ordering (Tamp.11).** `claim_milestone` transfers before it records the claim; `claim_prize` does the reverse. Not exploitable under the host's re-entrancy prohibition. Planned remediation: align the ordering.
10. **Event schema gaps (Repud.1, Repud.4).** Selection emits only a count, entering `Cancelling` emits nothing, and zero-value pro-rata refunds leave no trace. Planned remediation: per-award, cancellation-started, and zero-refund events.
11. **Custodial wallets narrow TB5 (Spoof.10).** The assumption that a backend compromise cannot move escrow holds only for external wallets. For managed wallets the backend holds the key, so the escrow of any event whose owner, manager, or winners are managed is only as safe as the master encryption key. Planned hardening: KMS/HSM custody of the master key and external-wallet signing above a value threshold.
12. **Co-authorization uses the admin key (Spoof.2).** The crowdfunding milestone co-authorization is the same admin address that governs upgrades. Planned hardening: a dedicated validator role.
13. **Configuration state (EoP.10, Tamp.10).** Comparing the live mainnet state against the model showed two event records still in the earlier storage layout and live escrow held under a zero upgrade timelock. Both are scheduled for the next administrative operation, and the upgrade runbook will gain a gate that refuses to sign `apply_upgrade` unless the migration steps are queued in the same session.

**Did the treatments identified in Section 3 adequately address the issues identified?**
For 49 of the 72 threats, yes; every contract-level control has a test that asserts its failure mode (312 passing: 243 for `boundless-events`, 66 for `boundless-profile`, 3 storage-compatibility fixtures), and the backend and process controls are covered by the backend's own test suite and operating procedures. Two of the contract-level controls (Tamp.5, EoP.13) are fully effective only once EoP.10 closes. Thirteen threats are accepted residual risks with a written rationale. Ten are open: EoP.10, DoS.9, DoS.14, and EoP.14 are the items on which auditor input is most valuable; the remaining six (Tamp.14, Tamp.16, Tamp.17, DoS.7, DoS.13, EoP.15) have a planned remediation scheduled before audit fieldwork.

**Have additional issues been found after the threat model?**
This is the first iteration of the model. It will be revised after the audit report is received, and whenever the architecture changes materially: a new event type, a new authorization flow, a new external service, or a contract upgrade that touches storage layout.

**Any additional thoughts or insights on the threat modeling process that could help improve it next time?**
Two. First, modeling a role boundary (owner versus manager) with the same rigor as a system boundary proved valuable; the same treatment is planned for the grant-committee multi-signature primitive before it is implemented. Second, "what is the recovery path if this key is lost?" and "what happens if this transfer cannot complete?" should be standing questions for every `require_auth` and every token transfer in the contract, not only the admin's; asking them late is what produced DoS.9 and DoS.14.

### 4.1 Open items

| ID | Item | Treatment |
|---|---|---|
| EoP.10 | Upgrade timelock is 0 on both contracts while escrow is live | Restore to 17,280 ledgers as the next administrative operation, before audit fieldwork. |
| DoS.9 | Manager key loss strands the pool | Auditor input requested on the three candidate designs in the DoS.9 treatment, Section 3. |
| DoS.14 | A refund recipient that cannot receive the token blocks cancellation | Auditor input requested on the two remediation designs in the DoS.14 treatment, Section 3. |
| EoP.14 | An awarded grant can be cancelled before milestones are paid | Auditor input requested on extending the cancellation guard to multi-release awards. |
| DoS.7 | Profile availability gates bounty applications and milestone claims | Convert to `try_` calls before fieldwork. |
| Tamp.14 | Child op_ids not bound to the originating user | Include the authorizing address in the derivation before fieldwork. |
| Tamp.16 | Duplicate recipient in a multi-release selection strands a reservation | Reject duplicate recipients before fieldwork. |
| Tamp.17 | Submission rewrite after selection | Freeze submissions after selection; record last update time. |
| DoS.13 | Grant submissions unreachable | Grant application entry point or explicit rejection. |
| EoP.15 | Unbounded reputation bump | Cap per award and per call. |

### 4.2 Pending operational action

| ID | Item | Treatment |
|---|---|---|
| Tamp.10 | Two mainnet event records remain in the earlier storage layout | Run `migrate_events` and `migrate` on both contracts with the next administrative operation; no funds affected. |

### 4.3 Accepted residual risks

| ID | Risk | Rationale |
|---|---|---|
| Spoof.2 | Crowdfunding co-authorization uses the admin quorum | No single key can satisfy it; owner signature also required; dedicated validator role planned. |
| Spoof.9 | Software-held admin signer keys | Two signers required; separate machines; hardware-key upgrade scheduled by policy. |
| Spoof.10 | Managed-wallet custody | Envelope encryption, sponsor separation, on-chain attribution, external-wallet option; KMS/HSM and value-threshold external signing planned. |
| Tamp.15 | Pre-selection cancellation | Owner's money until awarded; visible on-chain; platform terms govern. |
| Repud.7 | `content_uri` not content-addressed by the contract | Backend to pin and hash submissions. |
| Info.1, Info.7 | Public ledger transparency | Pseudonymous; announcement batching. |
| DoS.4 | Single RPC provider | Queue buffering; failover planned. |
| DoS.10 | Global pause freezes claims and does not stop time-based windows | Emergency control under 2-of-3; expiry extension on unpause planned. |
| DoS.11 | State archival of dormant events | TTL extended on read; restorable by anyone. |
| DoS.15 | Fee-account trustline misconfiguration | Admin-inflicted, reversible; runbook check and monitoring events. |
| EoP.8 | Manager may award itself | Judging is off-chain; delegation is the owner's explicit, on-chain-visible choice. |
| EoP.16 | Single-step events-side profile binding | Cannot move funds; reversible; two-step rotation is a candidate hardening. |

---

## 5. Verification and review plan

| Layer | What is in place | Cadence |
|---|---|---|
| Contract tests | 312 tests: every payout split asserts recipient and fee-account deltas; admin authorization asserted as genuinely required; idempotency and op_id namespacing; paged cancellation at 220 contributors; storage migration replayed against synthetic mainnet-shaped state. | Every commit (CI, release profile). |
| Live-network tests | 23 testnet smoke scripts drive the deployed contracts through the backend orchestrator across every lifecycle path, including cancellation, grant sweep and replay, crowdfunding co-authorization, and a non-USDC token. | Before every upgrade. |
| Build integrity | Reproducible locked WASM build with a size gate; the upgrade runbook requires reproducing the on-chain hash from source and simulating every admin envelope before any signer signs. | Every upgrade. |
| Static analysis | Scout (CoinFabrik) against the source under review; findings triaged into the repository with a false-positive registry. | Every release. |
| On-chain monitoring | Both contracts emit an event for every state change, including every administrative one (`PendingUpgradeProposed`, `UpgradeApplied`, `AdminUpdated`, `PendingAdminSet`, `FeeAccountUpdated`, `FeeBpsUpdated`, `ProfileContractUpdated`, `TokenRegistered`, `Paused`, `Unpaused`, `EventsContractUpdated`). The backend indexer consumes them; a public analytics model reconciles escrow inflows and outflows. Planned before fieldwork: alerting on every administrative event, a reconciliation check that the contract's token balance is at least the sum of `remaining_escrow`, and stuck-state detection (events in `Cancelling` beyond a threshold, prizes approaching their claim expiry). | Continuous. |
| Incident response | `pause` on both contracts is the first action for any suspected compromise; the custody policy defines emergency signer rotation (pause, provision a new signer, rotate the admin in two steps, unpause), lost-signer scenarios, and a written escalation list. Every incident produces a written record. | As needed; rotation drilled on testnet. |
| External audit | This submission. Findings will be fixed, re-tested, and the fixes rehearsed on testnet before a mainnet upgrade; the threat model will be updated to reference the report. | Once, then on material change. |
| Threat-model review | This document is revised with every release that changes the contract surface, authorization, or storage layout, and reviewed in full at least annually. | Per release; annual. |

---

## Appendix A — Contract entry points and authorization

| Function | Contract | Authorization | Notes |
|---|---|---|---|
| `__constructor` | events | none (deploy time) | fee at most 10% |
| `create_event` | events | `params.owner` | supported token; floors positive, sum at most budget; deposit plus fee pulled in-transaction (non-crowdfunding) |
| `propose_manager`, `cancel_pending_manager` | events | current manager (owner if none) | proposal expires in 17,280 ledgers; no op_id |
| `accept_manager` | events | proposed manager | no op_id |
| `add_funds` | events | `from` | at least 10 tokens; event `Active` |
| `apply_to_bounty`, `withdraw_application` | events | `applicant` | bounty events only; withdrawal blocked once a submission exists |
| `submit`, `withdraw_submission` | events | `applicant` | bounties and grants require a prior application (grants have no application entry point, DoS.13); crowdfunding rejected; slot-keyed |
| `select_winners` | events | current manager | at most 50 per call; single-release: per-position once, repeatable; multi-release: exactly one call; at or above floor; owed-total reservation |
| `claim_prize` | events | stored award recipient | single-release only; window enforced at cancellation |
| `claim_milestone` | events | event owner, plus admin for crowdfunding | recipient must hold a winner record |
| `start_cancel` | events | current manager | refused while single-release prizes remain inside the 90-day window; multi-release awards carry no guard (EoP.14) |
| `process_cancel_batch`, `finalize_cancel` | events | none (permissionless crank) | only while `Cancelling` |
| `set_admin` | events, profile | admin | two-step, 7-day expiry |
| `accept_admin` | events, profile | proposed admin | |
| `set_fee_bps`, `set_fee_account`, `set_profile_contract` | events | admin | fee at most 10%; single-step |
| `pause`, `unpause` | events, profile | admin | admin entry points remain callable while paused |
| `register_supported_token`, `deregister_supported_token` | events | admin | |
| `propose_upgrade`, `apply_upgrade`, `cancel_pending_upgrade` | events, profile | admin | timelock currently 0 (EoP.10); 30-day expiry |
| `migrate_events` | events | admin | paged, at most 8 events per call |
| `migrate` | events, profile | admin | one-shot per version; events refuses while records remain unconverted |
| `set_events_contract` (first binding) | profile | admin | single-step only when unset |
| `propose_events_contract`, `accept_events_contract`, `cancel_pending_events_contract` | profile | admin | 17,280-ledger timelock, 7-day expiry |
| `bootstrap`, `bump_reputation`, `slash_reputation`, `register_earnings` | profile | events contract (contract-invoker authorization) | op_id domain is the events contract |
| `bootstrap_self` | profile | `user` | op_id domain is the user |
| `admin_slash_reputation` | profile | admin | non-empty reason required |
| All read functions | both | none | list reads page at 100 entries |

## Appendix B — Audit readiness summary

| Item | Status |
|---|---|
| Unit and integration tests | 312 passing (243 `boundless-events`, 66 `boundless-profile`, 3 storage-compatibility fixtures). Every payout split asserts both the recipient and the fee-account balance deltas; admin authorization is asserted as genuinely required. |
| Live-network tests | 23 testnet smoke scripts drive the deployed contracts through the backend orchestrator: publish, contribute, submit, select, claim, single- and multi-contributor cancellation, grant sweep and replay, crowdfunding co-authorization, non-USDC token, fee deduction. |
| Continuous integration | Reproducible locked WASM build, WASM size gate, formatting check, mainnet upgrade-guard script, storage-compatibility fixtures, full test suite in release mode. |
| Static analysis | Scout (CoinFabrik) is run against the source under review. The single outstanding item is the arithmetic change described under Tamp.7. |
| Upgrade path | Timelocked propose and apply, on-chain `version()`, paged migration, testnet rehearsal, mainnet runbook with simulation before signing. |
| Admin custody | Documented custody policy, multi-signature pre-flight checklist, and an on-chain signer verification script. |
| Disclosed items | Open: Section 4.1. Pending operational action: Section 4.2. Accepted: Section 4.3. |
