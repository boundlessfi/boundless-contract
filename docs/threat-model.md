# Boundless Platform — STRIDE Threat Model

**Version:** 1.0
**Date:** June 2026
**Scope:** `boundless-events` + `boundless-profile` Soroban contracts and the off-chain orchestrator (`boundless-nestjs`)
**Prepared by:** Boundless Engineering
**Status:** Submitted for SDF Soroban Security Audit Program

---

## Section 1 — What Are We Working On?

### 1.1 System Description

Boundless is an on-chain coordination platform for hackathons, bounties, grants, and crowdfunding campaigns on the Stellar network. Its core value proposition is trust-minimised prize settlement: escrow locks at event creation, and releases atomically when winners are selected — sponsor default is structurally impossible.

The system consists of two Soroban smart contracts and an off-chain orchestrator:

| Component | Role |
|---|---|
| `boundless-events` | On-chain anchor for all event types. Manages escrow custody (deposit, lock, release, refund), event lifecycle, participant registration, submission anchoring, and winner selection for all four pillars (hackathon, bounty, grant, crowdfunding). |
| `boundless-profile` | Per-user credits and reputation scores. Called cross-contract by `boundless-events` to charge credits on application and award reputation on win. |
| `boundless-nestjs` | Off-chain orchestrator (NestJS). Holds rich content, user accounts, KYC state, and draft lifecycle. Builds and submits Stellar transactions via RPC. Houses the admin KYC workflow and the server-side admin co-signer. |
| Frontend (`boundless`) | Next.js web app. Runs in the user's browser. Calls the backend API. No direct RPC calls; all chain interaction is mediated by the backend. |
| Admin Portal | Staff-only Next.js app. Calls the backend admin API surface. Enforces step-up TOTP for sensitive mutations. |
| Didit KYC | External identity verification service. Receives user documents; sends HMAC-signed webhooks back to the backend. No PII stored on Boundless infrastructure. |
| Stellar RPC (Nodies) | Managed Soroban RPC endpoint. Backend submits signed transactions; event indexer polls for on-chain events. |
| Admin Multi-sig Account | 2-of-3 Stellar multi-sig G-address (`GCU64B3YTFL2M6I6MKPIIZWNYAIOKKOFVDJ6W2AIGNRZYO6OW5U32ZIT`). Controls all privileged contract operations (pause, fee changes, upgrade). No single team member can act unilaterally. |
| Organization Treasury Wallets | Stellar classic G-addresses with USDC trustlines. Held by organizer accounts; used to fund event escrow at publish time. |

---

### 1.2 Dataflow Diagram

```
 ═══════════════════ TRUST BOUNDARY: Internet ═══════════════════════════
 │                                                                        │
 │   [User Browser]          [Sponsor Browser]      [Admin Browser]      │
 │        │ REST/JWT               │ REST/JWT             │ REST/JWT     │
 ═════════════════════════════════════════════════════════════════════════
          │                        │                      │
          ▼                        ▼                      ▼
 ┌─────────────────────────────────────────────────────────────┐
 │               BACKEND API  (boundless-nestjs)               │
 │  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐    │
 │  │ Auth / KYC   │  │ Event/Escrow │  │  Admin v2      │    │
 │  │ Guard layer  │  │  Orchestrator│  │  (PolicyGuard +│    │
 │  │ (JWT, TOTP,  │  │  (build tx,  │  │  StepUpGuard)  │    │
 │  │  KycGuard)   │  │  admin cosign│  │                │    │
 │  └──────┬───────┘  └──────┬───────┘  └───────┬────────┘    │
 │         │                 │                   │             │
 │  ┌──────▼─────────────────▼───────────────────▼──────────┐ │
 │  │           PostgreSQL DB (PII-free)                     │ │
 │  │  User accounts, event drafts, escrow op log,           │ │
 │  │  KYC status (session_id + status only — no docs)       │ │
 │  └────────────────────────────────────────────────────────┘ │
 └──────────────────────────┬──────────────────────────────────┘
                            │
         ═══════════════════╪═══════════════════════════════════
         │  TRUST BOUNDARY: Off-chain / On-chain                │
         ═══════════════════╪═══════════════════════════════════
                            │ signed XDR
                            ▼
              ┌─────────────────────────┐
              │  Stellar RPC (Nodies)   │
              └────────────┬────────────┘
                           │ Soroban invocation
          ┌────────────────┴────────────────────┐
          ▼                                      ▼
 ┌──────────────────┐                ┌────────────────────┐
 │ boundless-events │──cross-contract│ boundless-profile  │
 │ (escrow, event   │───────────────▶│ (credits +         │
 │  lifecycle for   │                │  reputation)       │
 │  all 4 pillars)  │                └────────────────────┘
 └──────────────────┘
          │
          │ USDC token transfer (SAC)
          ▼
 ┌──────────────────────────────────┐
 │  Stellar Ledger                  │
 │  Winner wallets / fee account /  │
 │  org treasury wallet             │
 └──────────────────────────────────┘

 ═══════════════════ EXTERNAL SERVICE BOUNDARY ═══════════════
 │                                                             │
 │   [Didit KYC]                                              │
 │     │ HMAC-signed webhook (POST /api/didit/webhook)        │
 │     └──────────────────────────────────────────────────────┤
 │     User browser ──HTTPS──▶ Didit hosted flow              │
 │     (documents never touch Boundless servers)              │
 │                                                             │
 ═════════════════════════════════════════════════════════════
```

**Trust Boundaries:**

| ID | Boundary | Description |
|---|---|---|
| TB1 | Internet / Backend | All user-originated input is untrusted. Enforced by JWT auth, KYC guard, input validation (class-validator + Zod). |
| TB2 | Backend / Stellar network | The network is decentralised; the backend cannot control what other actors submit. Backend verifies all on-chain state before acting. |
| TB3 | Backend / Didit | External service. Webhook authenticity enforced via HMAC. No PII crosses TB3 into Boundless systems. |
| TB4 | Admin browser / Admin Portal / Backend | Staff-only surface. Higher trust but still fully authenticated (OIDC) with step-up TOTP for Tier 2 actions. |
| TB5 | Off-chain / On-chain | The contract enforces its own auth rules independently of the backend. Compromise of the backend does not grant control over on-chain escrow. |

**Data Stores:**

| Store | Location | Contents | Sensitivity |
|---|---|---|---|
| PostgreSQL | Boundless servers | User accounts, event drafts, escrow op log, KYC status (no PII), audit log | Medium — no documents or keys |
| Soroban contract storage | Stellar ledger | Event records, escrow balances, winner assignments, credits, reputation | Public by design — pseudonymous addresses only |
| Didit systems | Didit infrastructure | Identity documents, biometrics, verification decisions | High — held exclusively by Didit |
| Admin multi-sig keys | Distributed (3 signers, separate machines) | Freighter wallet keys (software baseline) | Critical |
| Backend environment | Server environment vars | JWT secret, encryption key, Didit API key, orchestrator signing key | High |

---

## Section 2 — What Can Go Wrong?

### STRIDE Reminders

| Category | Definition | Key question |
|---|---|---|
| **Spoofing** | Impersonation of another user or system | Could someone perform an action as someone else? |
| **Tampering** | Unauthorized modification of data or code | Could data be altered unexpectedly? |
| **Repudiation** | Denying an action was taken | Can a party deny responsibility for an on-chain or off-chain action? |
| **Information Disclosure** | Unintended exposure of sensitive data | Is private data exposed where it should not be? |
| **Denial of Service** | Making the system unavailable | Can the system be overwhelmed or blocked? |
| **Elevation of Privilege** | Gaining access beyond what is authorized | Can a user obtain permissions they were not granted? |

---

### Threat Table

#### Spoofing

| ID | Threat | Affected Component |
|---|---|---|
| Spoof.1 | A caller passes another user's address in `create_event`, `apply`, or `add_funds` to act on their behalf. | `boundless-events` contract |
| Spoof.2 | The backend orchestrator co-signing key is leaked; an attacker uses it to forge admin co-authorization entries on crowdfunding `claim_milestone` calls. | Backend orchestrator, `boundless-events` |
| Spoof.3 | A malicious third party sends a forged Didit webhook to the backend to fraudulently approve a user's KYC status. | Backend `/api/didit/webhook` endpoint |
| Spoof.4 | An attacker reuses a stolen staff JWT to access the admin portal without valid step-up credentials. | Admin portal, backend admin API |
| Spoof.5 | A malicious actor attempts to claim they are the admin multi-sig by submitting a transaction signed by a single key, bypassing the 2-of-3 threshold. | Admin multi-sig account, contract admin functions |

#### Tampering

| ID | Threat | Affected Component |
|---|---|---|
| Tamp.1 | An organizer attempts to modify the prize amount or winner allocation after `create_event`, to redirect funds to themselves. | `boundless-events` contract |
| Tamp.2 | An attacker submits a `select_winners` call with a crafted `amounts` vector that sums to more than `remaining_escrow`, draining the contract. | `boundless-events` contract, `escrow.rs` |
| Tamp.3 | A builder attempts to overwrite another builder's submission `content_uri` to substitute their evidence. | `boundless-events` contract |
| Tamp.4 | A malicious or compromised Stellar RPC node returns forged event data to the backend indexer, causing the platform to display incorrect state. | Backend event indexer, Nodies RPC |
| Tamp.5 | The WASM hash in a `propose_upgrade` call is quietly replaced between proposal and application to install malicious contract logic. | Contract upgrade flow (`admin.rs`) |
| Tamp.6 | An attacker intercepts and modifies a partially-signed multi-sig transaction in transit to change the operation parameters (e.g., new admin address). | Admin multi-sig signing workflow |

#### Repudiation

| ID | Threat | Affected Component |
|---|---|---|
| Repud.1 | An organizer claims they never called `select_winners` or disputes the winner list they submitted. | `boundless-events` contract |
| Repud.2 | A winning builder claims they never received their payout. | Stellar ledger, USDC token transfers |
| Repud.3 | A staff member claims they never overrode a KYC decision, or disputes the reason/timestamp. | Backend admin KYC service, audit log |
| Repud.4 | A contributor to an event cancellation claims they never received their refund. | `boundless-events` contract, paged cancel flow |
| Repud.5 | The platform claims an admin co-sign was not performed on a crowdfunding milestone release, to deny a builder's payout claim. | Soroban transaction XDR, backend orchestrator |

#### Information Disclosure

| ID | Threat | Affected Component |
|---|---|---|
| Info.1 | All Soroban contract storage is publicly readable; event participants' wallet addresses, prize amounts, and winner assignments are visible on-chain. | `boundless-events` contract, Stellar ledger |
| Info.2 | A data breach of the Boundless PostgreSQL database exposes user PII including identity documents. | Backend database |
| Info.3 | The backend orchestrator co-signing key is exposed via misconfigured environment variables, logging, or a code path that serializes config. | Backend environment, orchestrator service |
| Info.4 | Staff admin JWT tokens or TOTP secrets are exposed via a compromised admin workstation or session fixation attack. | Admin portal, staff auth flow |
| Info.5 | A Didit webhook payload containing partial identity data (e.g., name, status) is logged in plaintext by the backend. | Backend webhook handler, logging |
| Info.6 | The fee account private key (used to receive platform fees) is exposed, allowing fee diversion. | Fee account management, backend config |

#### Denial of Service

| ID | Threat | Affected Component |
|---|---|---|
| DoS.1 | An attacker floods the `apply` endpoint for a popular hackathon to exhaust the applicant cap (5,000) with bot accounts, locking out legitimate builders. | `boundless-events` contract, backend API |
| DoS.2 | A large hackathon with thousands of contributors triggers a single-transaction cancel that exhausts Stellar ledger resource limits. | `boundless-events` contract, paged cancel flow |
| DoS.3 | An attacker floods the backend API with unauthenticated requests, causing rate limit exhaustion for legitimate users. | Backend API, `GlobalThrottlerGuard` |
| DoS.4 | The Soroban RPC endpoint (Nodies) becomes unavailable, preventing the backend from submitting or reading any transactions. | Nodies RPC dependency |
| DoS.5 | An attacker submits many failing transactions with high compute units to delay legitimate transaction processing. | Stellar network, Soroban fee market |
| DoS.6 | A malicious webhook sender floods the `/api/didit/webhook` endpoint, exhausting backend processing capacity for KYC events. | Backend webhook handler |

#### Elevation of Privilege

| ID | Threat | Affected Component |
|---|---|---|
| EoP.1 | A builder calls `select_winners` directly (bypassing the backend), attempting to pick themselves as winner on an event they did not create. | `boundless-events` contract |
| EoP.2 | A staff member with `kyc:read` (Tier 0) permission calls the `kyc:override` endpoint (Tier 2) to approve their own or a collaborator's KYC. | Backend admin KYC controller, `PolicyGuard` |
| EoP.3 | An organizer calls `claim_milestone` on a crowdfunding event without the required admin co-authorization, attempting to claim funds early. | `boundless-events` contract, admin.require_auth() |
| EoP.4 | A regular backend API user (builder/organizer) sends requests to the `/admin/v2/*` routes by crafting bearer tokens with elevated claims. | Backend admin API, `StaffAuthGuard` |
| EoP.5 | An attacker calls `bootstrap_self` on `boundless-profile` with another user's address to create a profile and claim their credits. | `boundless-profile` contract |
| EoP.6 | A malicious event owner calls `set_admin` or `propose_upgrade` directly, attempting to rotate the contract admin to an address they control. | `boundless-events` contract, `admin.rs` |
| EoP.7 | The backend orchestrator key is used to approve a `claim_milestone` on behalf of an event where the organizer did not initiate the call, redirecting funds. | Backend orchestrator, `boundless-events` |

---

## Section 3 — What Are We Going to Do About It?

### Mitigation Table

| Threat ID | Mitigation |
|---|---|
| **Spoof.1** | All state-changing contract functions enforce `addr.require_auth()` via the Soroban auth framework. The Soroban runtime validates the caller's cryptographic signature against the address before executing. No caller can submit a valid auth entry for an address they do not control without its private key. |
| **Spoof.2** | The orchestrator co-signing key is scoped exclusively to `admin.require_auth()` within `claim_milestone`. It does NOT control the admin multi-sig account and cannot call `select_winners`, `set_admin`, or any other admin function. A compromise of the orchestrator key does not grant escrow control — the attacker would also need the event organizer's key (separate party). This is accepted residual risk with limited blast radius. Mitigation: rotate the orchestrator key immediately on suspected compromise; the admin multi-sig is not affected. |
| **Spoof.3** | All incoming Didit webhook requests are verified with HMAC-SHA256 using the `DIDIT_WEBHOOK_SECRET` before any payload is processed. Requests with missing or invalid signatures are rejected with HTTP 401 before deserialization. |
| **Spoof.4** | Staff JWTs are short-lived. Tier 2 actions (KYC override) require a valid TOTP code via `StepUpGuard` — a stolen JWT alone is insufficient. Session invalidation is supported via the `BetterAuth` session table. |
| **Spoof.5** | The admin multi-sig account is a Stellar G-address configured with 2-of-3 signers and threshold weights enforced at the Stellar protocol level. A single-signer transaction is rejected by the network before it reaches the contract. The `verify-multisig.sh` script verifies on-chain signer configuration before any governance operation. |
| **Tamp.1** | Prize amounts are locked in `remaining_escrow` at `create_event`. The `select_winners` function computes payouts from `remaining_escrow` at selection time, not from organizer-supplied values at release time. An organizer cannot change the total available after funding. |
| **Tamp.2** | The `select_winners` fee math divides `remaining_escrow` proportionally using basis points. The contract enforces that all percentages sum to 10,000 bps (100%) and releases only from what is held in escrow. An overflowing sum is rejected with `InvalidAllocation`. |
| **Tamp.3** | Submissions are keyed per `(event_id, applicant_address)`. A builder can update their own submission; another address's submission key is inaccessible without their auth. The submitting address is stored at commit time and is immutable thereafter. |
| **Tamp.4** | The backend verifies event existence against the contract before acting (read-then-act). The Nodies RPC endpoint is managed and dedicated; the backend does not follow `_links` navigation. Soroban event payloads include the emitting contract ID; the indexer filters on the known contract address. |
| **Tamp.5** | `propose_upgrade(wasm_hash, new_version)` stores the WASM hash on-chain. `apply_upgrade()` can only be called after the timelock elapses (~1 day) and re-verifies against the stored hash. The proposal is public on-chain; off-chain monitors can detect unexpected upgrade proposals. The admin multi-sig must authorize both steps. |
| **Tamp.6** | Multi-sig transactions are built offline, inspected before signing, and signed independently by each signer. The `verify-multisig.sh` script validates the on-chain signer configuration. Future target state: hardware keys (Yubikey/Ledger) eliminate in-memory key exposure. |
| **Repud.1** | `select_winners` emits a `WinnersSelected` Soroban event containing the event ID, winner addresses, and amounts. This event is immutable on the Stellar ledger and publicly auditable on Stellar Expert. |
| **Repud.2** | Every USDC release is an on-chain SAC `transfer` call. The transaction hash, sender, recipient, and amount are permanently recorded on the Stellar ledger. |
| **Repud.3** | Every admin KYC action (sync, retrigger, override) is recorded by `AdminAuditService` inside a database transaction with the staff member's ID, action type, target user ID, timestamp, and decision reason. The audit log is append-only. |
| **Repud.4** | The paged cancel flow emits on-chain events at each step (`CancelStarted`, `CancelBatchProcessed`, `CancelFinalized`). Each refund transfer is an on-chain SAC `transfer`. Stellar Expert shows the full refund history per account. |
| **Repud.5** | The admin co-sign is a `SorobanAuthorizationEntry` embedded in the signed XDR of the transaction. It is visible in the transaction envelope on Stellar Expert — both the organizer's auth and the admin's auth are present and attributable to their respective addresses. |
| **Info.1** | Wallet addresses are pseudonymous. Rich personal data (submission content, project descriptions) is stored off-chain via `content_uri` (IPFS or backend storage), not in contract storage. This transparency is by design for a trustless settlement platform. Users are informed that their address and prize amounts are publicly visible. |
| **Info.2** | The Boundless PostgreSQL database stores only the Didit `session_id` (opaque token) and a normalized status string (`Approved / Declined / In Review`). Identity documents, biometrics, and personal details are never received or stored by Boundless. A full database breach does not expose KYC documents. |
| **Info.3** | The orchestrator signing key is stored as an environment variable, never logged, and never serialized in API responses. The server-side signing path reads the key only at transaction build time. Key rotation procedure: update the env var and redeploy; the key does not appear in source code or git history. |
| **Info.4** | Staff sessions use short-lived JWTs. Tier 2 actions require a fresh TOTP token. Admin workstations are on separate browser profiles from personal use. Future: hardware-backed staff keys. |
| **Info.5** | The Didit webhook handler logs only the `session_id` and normalized status, never the full payload. Log levels are set to `error,warn,log` in production; raw request bodies are not logged. |
| **Info.6** | The fee account is a Stellar G-address whose key is held by the admin multi-sig (same custody as the contract admin). Fee diversion requires compromising the multi-sig. |
| **DoS.1** | Per-event applicant cap is 5,000 entries. The `apply` endpoint requires a valid JWT and KYC-approved status, raising the cost of bot registration significantly. Credit costs on application (`application_credit_cost`) are configurable per event, making spam economically prohibitive for events with non-zero credit costs. |
| **DoS.2** | The paged cancel design (`start_cancel` / `process_cancel_batch(max_refunds=25)` / `finalize_cancel`) caps per-transaction refund work at 25 entries, well within Stellar ledger resource limits even at 5,000 contributors. |
| **DoS.3** | `GlobalThrottlerGuard` applies per-IP rate limits globally. The backend runs behind a load balancer with DDoS protection at the infrastructure layer. |
| **DoS.4** | Nodies is a managed, SLA-backed RPC provider. BullMQ queues buffer all async contract operations; if RPC is temporarily unavailable, jobs retry with exponential backoff and are not lost. The `DistributedLockService` prevents duplicate submissions during retry storms. |
| **DoS.5** | Soroban's fee market (resource fees) prices out spam. The backend uses `simulateTransaction` before submission to estimate fees; operations that would exceed configured limits are rejected before they reach the network. |
| **DoS.6** | The webhook endpoint is rate-limited separately. Didit sends webhooks from known IP ranges; IP allowlisting can be added as a hardening measure. Invalid HMAC requests are rejected before any DB write. |
| **EoP.1** | `select_winners` enforces `resolve_manager(env, event_id, &event.owner).require_auth()`. The manager is always the event owner address (set at `create_event`). No other address can produce a valid auth entry for the event owner. |
| **EoP.2** | `PolicyGuard` checks the staff member's permission set against the required permission declared on the route (`@RequirePermission`). `kyc:override` maps to Tier 2. `StepUpGuard` independently verifies a fresh TOTP token for all Tier 2 routes. These are orthogonal checks — bypassing one does not bypass the other. |
| **EoP.3** | `claim_milestone` in `crowdfunding.rs` calls both `event.owner.require_auth()` AND `admin.require_auth()`. Both must be satisfied in the same transaction. Without the platform's server-side admin co-sign, the transaction is rejected by the contract at the auth validation step. |
| **EoP.4** | The `/admin/v2/*` routes are gated by `StaffAuthGuard`, which validates a separate staff JWT issued by the admin authentication system (BetterAuth with a separate secret). A user JWT from the main app is not valid on the admin surface. |
| **EoP.5** | `bootstrap_self` calls `caller.require_auth()` where `caller` is `env.invoker()`. The invoker is the transaction source — it is cryptographically bound to the account that signed the transaction. A third party cannot produce a valid auth for another user's address. |
| **EoP.6** | `set_admin` and `propose_upgrade` each enforce `admin.require_auth()` at the contract level. The admin is the 2-of-3 multi-sig account. No event owner address is the admin; the check is against the configured admin key stored in instance storage. |
| **EoP.7** | The orchestrator only co-signs `claim_milestone` calls that originate from a backend-authenticated organizer session who is the verified owner of the event in the DB. The co-sign path checks the event ownership record before building the transaction. If the organizer's session is not the event owner, the build is refused before a signing key is accessed. |

---

## Section 4 — Did We Do a Good Job?

### Dataflow Diagram Utility

The dataflow diagram directly informed our threat analysis. Mapping the trust boundary between the off-chain backend and the on-chain contracts (TB5) was the most productive part of the exercise — it clarified that compromise of the backend orchestrator does not automatically translate to loss of on-chain escrow, because the contracts enforce their own auth rules independently. This shaped our decision to scope the orchestrator co-signing key narrowly (crowdfunding admin co-auth only) rather than giving it broader permissions.

### STRIDE Coverage Assessment

All six STRIDE categories produced actionable threats. The most significant findings:

- **Spoofing:** `require_auth()` coverage across all contract functions is thorough, but the orchestrator key's scope needed explicit documentation. The key's limited blast radius (covered under Spoof.2) was identified during this exercise and has been documented in the operational runbooks.
- **Tampering:** The `select_winners` math (Tamp.2) was scrutinized in the June 2026 Stellar-skill audit (finding M1). The fix — computing winner payouts from `remaining_escrow` at selection time — was already applied, but the threat model gave us additional confidence that the math is correct.
- **Information Disclosure:** Info.2 (database breach does not expose KYC documents) is the strongest privacy property of the design, enabled by the architectural decision to keep PII exclusively with Didit. This would not have been called out as explicitly without the Information Disclosure category forcing the question.
- **Denial of Service:** DoS.2 (paged cancel) was the design driver for the `start_cancel / process_cancel_batch / finalize_cancel` architecture. The threat was identified pre-audit and the fix was implemented and audited before this threat model was written.
- **Elevation of Privilege:** EoP.3 (crowdfunding claim requires admin co-auth) is the most operationally sensitive control. The threat model confirmed that the dual-auth design is correctly implemented and the blast radius of an orchestrator key leak is bounded.

### Unresolved / Residual Risks

| Risk | Status | Acceptance Rationale |
|---|---|---|
| Orchestrator co-signing key compromise (Spoof.2, EoP.7) | Accepted, monitored | Blast radius is limited to crowdfunding claim timing; attacker also needs organizer's key. Key rotation procedure is documented. |
| Nodies RPC single point of failure (DoS.4) | Accepted, mitigated | Queue buffering handles transient outages; failover RPC endpoint planned for mainnet. |
| Software multi-sig (vs. hardware) for admin keys | Accepted, time-bounded | Hardware upgrade is triggered at $250K TVL or first signer-machine incident per the admin custody policy. |
| All contract storage public on-chain | Accepted by design | Pseudonymous addresses; PII off-chain via `content_uri`. Informed user consent. |

### Post-Modeling Changes

Two issues surfaced during the modelling process that were not previously documented:

1. The orchestrator key's scope was not formally written down anywhere. This threat model prompted us to document the key's exact permissions and rotation procedure in the admin runbook.
2. The audit log for KYC overrides did not originally include the `reason` field in the initial implementation. Reviewing the Repudiation category prompted us to confirm this was added and tested — it is present in `AdminKycActionsService.override()`.

### Living Document Commitment

This threat model will be revisited before mainnet launch (Tranche 3) and any time the architecture changes materially (new pillar, new auth flow, new external service, contract upgrade).

---

## Appendix A — Contract Entry Points and Auth Requirements

| Function | Contract | Caller Auth Required |
|---|---|---|
| `__constructor` | events | None (deploy-time only) |
| `create_event` | events | `params.owner.require_auth()` |
| `apply` | events | `applicant.require_auth()` |
| `withdraw_application` | events | `applicant.require_auth()` |
| `submit` | events | `event.owner.require_auth()` (manager) |
| `add_funds` | events | `from.require_auth()` |
| `select_winners` | events | `event.owner.require_auth()` (manager) |
| `claim_milestone` | events | `event.owner.require_auth()` + `admin.require_auth()` |
| `start_cancel` | events | `admin.require_auth()` |
| `process_cancel_batch` | events | `admin.require_auth()` |
| `finalize_cancel` | events | `admin.require_auth()` |
| `set_admin` | events | `admin.require_auth()` (current admin) |
| `accept_admin` | events | `pending.target.require_auth()` |
| `pause` / `unpause` | events | `admin.require_auth()` |
| `propose_upgrade` | events | `admin.require_auth()` |
| `apply_upgrade` | events | `admin.require_auth()` |
| `migrate` | events | `admin.require_auth()` |
| `register_supported_token` | events | `admin.require_auth()` |
| `set_events_contract` (first) | profile | `admin.require_auth()` |
| `propose_events_contract` | profile | `admin.require_auth()` |
| `accept_events_contract` | profile | `admin.require_auth()` |
| `bootstrap_self` | profile | `caller.require_auth()` |

---

## Appendix B — Audit Checklist Cross-Reference

| Checklist Item | Status | Evidence |
|---|---|---|
| SCF funded | Yes | SCF grant recipient; Tranche 2 submission |
| Repo hygiene | Yes | `README.md`, `CONTRIBUTING.md`, `BACKLOG.md`, `docs/`, clean CI |
| Integration tests | Yes | 15+ smoke scripts against live testnet; `cargo test` 97/97 passing |
| Threat model | Yes | This document |
| Dataflow diagram | Yes | Section 1.2 above |
| Tooling scan | Done | Scout (CoinFabrik) run June 2026; report at `docs/scout-audit-report.md` |
| Remediation plan | Done | June 2026 Stellar-skill audit (all HIGH/MEDIUM closed) + Scout findings remediated; see `docs/audit-2026-06-stellar-skill.md`, `docs/scout-audit-report.md`, `BACKLOG.md` |
