# boundless-contract: Architecture

## The fork in the road

There are two coherent ways to build a contract for a platform like Boundless:

**Custody-only on-chain.** Contract holds money. Everything else (event records, submissions, winners, credits, reputation) lives off-chain in Postgres. The platform is the source of truth. Cheap to audit; useless for transparency; reintroduces all the trust we're trying to remove.

**Custody plus the records that matter on-chain.** Contract holds money, event existence, key terms, submission anchors, winner records, credits, and reputation. The platform is the source of truth for everything that benefits from iteration speed (descriptions, KYC, role policies, AI) but cannot edit the things that demand transparency.

We took the second path. This document explains the boundary.

## What goes on-chain

| Family | Lives in | Why |
|--------|----------|-----|
| Event existence (id, owner, pillar, token, total_budget) | events contract | Organizers cannot quietly delete events that went badly. |
| Event key terms (release_kind, deadline, winner_distribution, application_credit_cost) | events contract | These define what is owed to whom; mutating them off-chain would let the platform rewrite outcomes. |
| Submission anchors (applicant, content_uri, timestamp) | events contract | Tamper-proof participation history. |
| Winner records (recipient, position, amount, milestone, paid_at) | events contract | The promise of "you won this much" has to be enforced by the chain. |
| Escrow balance and fee withholding | events contract | The platform cannot hold the keys to event funds. |
| Token whitelist | events contract | Admin-managed; on-chain so the whitelist is auditable. |
| Credit balances | profile contract | The platform cannot mint or freeze credits. |
| Reputation scores | profile contract | The platform cannot inflate or deflate reputation. |
| Per-token earnings | profile contract | The promise of "you've earned this much" matches the chain's record of releases. |
| Idempotency markers | both contracts (temporary storage with TTL) | Replay-safe state transitions across the orchestrator's retry surface. |

## What stays off-chain (boundless-nestjs)

| Family | Why |
|--------|-----|
| Rich content (descriptions, banners, write-ups, comments) | Stored in S3, referenced by content_uri. Cheap to mutate; on-chain would burn storage. |
| Draft state | Drafts never hit the contract. Only published events do. |
| KYC tiers and verification details | PII. |
| Role policies and per-event reviewer assignments | Iterate rapidly without contract upgrades. |
| Tier brackets and credit-discount rules | Off-chain policy that interprets on-chain scores; lets product tune without audit. |
| AI features (organizer assist, judging assist, similarity detection) | Out of scope for any contract. |
| Moderation queue, notifications, analytics, exports | None of these benefit from immutability. |
| Indexed event queries | Linear-scan getters in the contract burn cost; off-chain mirror serves them cheaply. |

## Why two contracts and not one

Soroban's compiled WASM size limit is 64 KB. The platform contract that holds events + escrow + multi-token + idempotency lands around 50 KB once all operation bodies are implemented. Adding credits and reputation operations would push it over.

The natural split line: events are per-event; profile is per-user. Cross-family operations (apply charges credits; select_winners bumps reputation) happen at the events-to-profile boundary, where the events contract calls the profile contract with explicit auth.

Three contracts would have been over-engineering. Two contracts split is the smallest cut that keeps both under the ceiling while keeping cross-contract calls bounded.

## The cross-contract dance

```
applicant.require_auth() in events contract
  events contract holds the binding for the profile contract address
  events contract calls profile.spend_credits(applicant, cost, "apply", op_id)
  profile contract verifies events_contract.require_auth()
  profile contract checks balance, debits, emits CreditsSpent
```

Users cannot call profile mutations directly. The profile contract verifies the caller is the registered events contract address (or admin for direct grants and slashes). The `events_contract` binding in the profile contract is two-step rotatable so the events contract can be upgraded or replaced without re-deploying profile.

## Multichain readiness without multichain code

v1 ships Stellar-only. The Chain Abstraction ADR (`boundless-chain-abstraction-adr.md`) defines what the off-chain orchestrator does to keep the doors open: `chain_id` everywhere in nestjs schema, `ChainAdapter` interface with `StellarAdapter` as the only implementation, per-chain wallets and fee accounts.

The contracts themselves do nothing for multichain. The only hook is the `profile_contract` binding on the events contract: when profile relocates to a cross-chain-aware deployment (someday), the events contract swaps the address. No event-contract code change.

## Deployment-epoch IDs

Borrowed from Stallion (`stallionsassemble/stallion-contract`). Upper 32 bits of every event id encode the ledger sequence at deployment. Cross-deployment ID stability is built in: ids from a redeployment never collide with ids from a prior deployment.

```rust
fn id_base(env: &Env) -> u64 {
    let seq = storage::get_deployment_seq(env);
    (seq as u64) << 32
}
```

The off-chain orchestrator's `EventReference` type includes the contract address so a single u64 alone is never enough to point at an event globally. The chain prefix from the ADR completes the global namespacing.

## Idempotency

Every state-mutating op accepts an `op_id: BytesN<32>` derived deterministically off-chain. The contract refuses to replay. The orchestrator interprets the replay error as success-from-prior-attempt. `OpSeen` markers use temporary storage, which auto-expires (Soroban TTL). The orchestrator caps reconciliation at 14 days to stay inside that window.

## Token whitelist and multi-token

The events contract supports any SAC-compatible token, but only tokens admin-registered with `register_supported_token`. The whitelist is on-chain so the registry is auditable. The pre-flight check on registration verifies the fee account has a trustline for the new token by attempting a zero-amount self-transfer; missing trustline reverts.

Launch whitelist: USDC. Phase 2: XLM. Adding a token after launch is documented in the platform contract PRD Section 8.4.

## Authority model

Three roles, no overlap:

- **Admin.** Platform deployment multisig. Can rotate admin, change fee bps, change fee account, register and deregister tokens, change profile contract binding, pause and unpause, upgrade. All admin ops require `admin.require_auth()`.
- **Event owner.** The organizer, set at create_event. Can cancel their event, select winners, claim milestones. `owner.require_auth()` on every owner op.
- **Applicant.** Per user. Can apply, submit, withdraw application, withdraw submission for themselves. `applicant.require_auth()` on every applicant op.

Admin rotations are two-step (set_admin then accept_admin within 7 days at testnet cadence). Same pattern for profile-contract binding rotation. Prevents accidental key changes.

## Pause

Admin can pause the entire events contract or the entire profile contract. Pause blocks every state-mutating op except admin ops. Reads continue. Designed for emergency response, not routine operation.

## Upgrade

Standard Soroban `update_current_contract_wasm`. Storage layout migrations are the caller's responsibility. Soroban does not move data automatically; we plan storage shapes to be additive across upgrades.

## The 64 KB discipline

Every PR touches the workspace's CI size check. WASM size is reported in `make size`. We accept the discipline:

- No linear-scan getters in the contract. Off-chain indexes them.
- Short error variants; no verbose strings in contract code.
- Per-event collections in separate storage keys, not inlined into `EventRecord`.
- Event names under 25 characters (the `contractevent` macro behaves predictably in that range).
- Functions kept under Soroban's 10-parameter limit; packed structs for create_event.

When the limit is approached, the first thing we cut is more getters. When that runs out, we re-evaluate the split.
