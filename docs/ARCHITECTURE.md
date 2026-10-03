# boundless-contract: Architecture

## The fork in the road

There are two coherent ways to build a contract for a platform like Boundless:

**Custody-only on-chain.** Contract holds money. Everything else (event records, winners, reputation) lives off-chain in Postgres. The platform is the source of truth. Cheap to audit; useless for transparency; reintroduces all the trust we're trying to remove.

**Custody plus the records that matter on-chain.** Contract holds money, event existence, the terms that decide who is owed what, winner awards, and reputation. The platform is the source of truth for everything that benefits from iteration speed (descriptions, applications, KYC, role policies, AI) but cannot edit the things that demand transparency.

We took the second path. This document explains the boundary.

The boundary has moved twice, both times toward the platform. Through 1.0.0 the profile contract held per-user credit balances; 1.1.0 removed them, because credits are a pricing lever that product retunes often, which is exactly the iteration-speed test above. Through 1.7.0 the events contract also held applicant and submission records (`apply`, `apply_to_bounty`, `submit` and their registries); 2.0.0 removed them, because no payout ever read them: what the chain enforces is the award recorded at selection. Both now live in boundless-nestjs.

## What goes on-chain

| Family | Lives in | Why |
|--------|----------|-----|
| Event existence (id, pillar, owner, token, total budget, title, content URI) | events contract | Organizers cannot quietly delete events that went badly. |
| Terms that decide payouts (release kind, prize floors, fee override) | events contract | A published prize table is a guarantee: `select_winners` may pay above a position's floor but never below it. Mutating these off-chain would let the platform rewrite outcomes. The deadline is stored for reference and not enforced. |
| Winner awards, the grant roster, and payment state (recipient, position, amount, milestone, paid_at) | events contract | The promise of "you won this much" has to be enforced by the chain. |
| Contributors and their amounts | events contract | Refunds on cancellation are computed from them. |
| Escrow balance, owed total, and fee withholding | events contract | The platform cannot hold the keys to event funds. |
| Event manager delegation | events contract | Selection and cancellation authority must be provable. |
| Token whitelist | events contract | Admin-managed; on-chain so the whitelist is auditable. |
| Reputation scores | profile contract | The platform cannot inflate or deflate reputation. |
| Per-token earnings | profile contract | The promise of "you've earned this much" matches the chain's record of releases. |
| Idempotency markers | both contracts (temporary storage with TTL) | Replay-safe state transitions across the orchestrator's retry surface. |

## What stays off-chain (boundless-nestjs)

| Family | Why |
|--------|-----|
| Applications, submissions and entries | Moved off chain in 2.0.0. Judging works from them, but no payout reads them; the award recorded at selection is what the chain enforces. |
| Rich content (descriptions, banners, write-ups, comments) | Stored in S3, referenced by content_uri. Cheap to mutate; on-chain would burn storage. |
| Draft state | Drafts never hit the contract. Only published events do. |
| KYC tiers and verification details | PII. |
| Role policies and per-event reviewer assignments | Iterate rapidly without contract upgrades. |
| Credit balances and charges | Off-chain ledger since 1.1.0. Credits are pricing policy, not settlement. |
| Tier brackets and other rules that interpret on-chain scores | Lets product tune without an audit. |
| AI features (organizer assist, judging assist, similarity detection) | Out of scope for any contract. |
| Moderation queue, notifications, analytics, exports | None of these benefit from immutability. |
| Indexed event queries | Linear-scan getters in the contract burn cost; an off-chain mirror serves them cheaply. |

## Why two contracts and not one

Soroban's compiled WASM size limit is 64 KB. The events contract, with escrow, multi-token support, idempotency and every pillar's flows, already sits close to it. Adding reputation and earnings would push it over.

The natural split line: events are per-event; profile is per-user. Cross-family operations (a payout bumps the winner's reputation and registers the earnings) happen at the events-to-profile boundary, where the events contract calls the profile contract with explicit auth.

Three contracts would have been over-engineering. Two contracts split is the smallest cut that keeps both under the ceiling while keeping cross-contract calls bounded.

## The cross-contract dance

```
winner signs claim_prize (or the owner signs claim_milestone) in the events contract
  events contract pays the recipient from escrow
  events contract calls profile.bootstrap(recipient, child_op_id)
  events contract calls profile.bump_reputation(recipient, delta, "win", child_op_id)
  events contract calls profile.register_earnings(recipient, token, amount, child_op_id)
  profile contract verifies events_contract.require_auth()
  profile contract mutates reputation / earnings, emits events
```

The profile calls are best-effort: the payment is final, so a profile failure never reverts it. Users cannot call the profile mutations directly; the profile contract only accepts them from its bound events contract, and the admin can only lower a score (`admin_slash_reputation`, with a reason). A user may create their own empty profile with `bootstrap_self`. The binding is set once at deploy and later rotated through a timelocked two-step (`propose_events_contract`, `accept_events_contract`), so the events contract can be replaced without redeploying profile.

## Multichain readiness without multichain code

v1 ships Stellar-only. The off-chain orchestrator keeps the doors open: chain ids throughout the backend schema, a chain adapter interface with Stellar as its only implementation, and per-chain wallets and fee accounts.

The contracts themselves do nothing for multichain. The only hook is the `profile_contract` binding on the events contract: if profile ever relocates to a cross-chain-aware deployment, the events contract swaps the address. No events-contract code change.

## Deployment-epoch IDs

Borrowed from Stallion (`stallionsassemble/stallion-contract`). Upper 32 bits of every event id encode the ledger sequence at deployment. Cross-deployment ID stability is built in: ids from a redeployment never collide with ids from a prior deployment.

```rust
fn id_base(env: &Env) -> u64 {
    let seq = storage::get_deployment_seq(env);
    (seq as u64) << 32
}
```

The off-chain orchestrator's `EventReference` type includes the contract address so a single u64 alone is never enough to point at an event globally.

## Idempotency

Every state-mutating op accepts an `op_id: BytesN<32>` derived deterministically off-chain. The contract refuses to replay. The orchestrator interprets the replay error as success-from-prior-attempt. `OpSeen` markers are keyed by the authorizing caller and the `op_id`, so a permissionless call cannot squat a privileged caller's id, and live in temporary storage, which expires on its own (Soroban TTL). The orchestrator caps reconciliation at 14 days to stay inside that window.

## Token whitelist and multi-token

The events contract supports any SAC-compatible token, but only tokens the admin registers with `register_supported_token`. The whitelist is on-chain so the registry is auditable.

The contract does not check that the fee account can receive a token: a Stellar Asset Contract's `balance()` cannot tell a missing trustline from a zero balance. A missing trustline makes every deposit in that token revert, so the operator checks it off chain before registering a token or rotating the fee account (`scripts/admin/verify-fee-trustline.sh`, which `register_token.sh` and `deploy_mainnet.sh register-token` run). Mainnet whitelists USDC and native XLM.

## Authority model

| Role | Who | Can |
|------|-----|-----|
| Admin | The platform's 2-of-3 multisig | Rotate the admin (two-step), set the fee rate and fee account, register and deregister tokens, change the profile binding, pause and unpause, upgrade through the timelock, appoint or clear the release validator. Cannot move escrow. |
| Event owner | The organizer, set at `create_event` | Fund the event, release grant and crowdfunding milestones (`claim_milestone`), forfeit a grant milestone it rejects, take management back before the first award (`reclaim_management`). Manages the event itself until it delegates. |
| Event manager | Optional, per event | Whoever currently manages (the owner at first) proposes a successor with `propose_manager`, which takes over once it accepts within 17,280 ledgers. Selects winners, starts a cancellation, delegates onward. Never holds custody: refunds route to the owner and contributors by construction. |
| Release validator | Optional, platform-wide | Co-signs every crowdfunding milestone release with the campaign owner. The admin proposes it and the key itself accepts within 120,960 ledgers; the admin can clear it at once, and co-signs itself while none is appointed. Decides when a release happens, never where the money goes or how much. |
| Winner | A recorded award's recipient | Collect its own prize with `claim_prize`. |
| Contributor | Anyone who adds funds | `add_funds`, and `claim_refund` for a refund the contract had to hold back. |
| Anyone | No signature | Drive a started cancellation forward (`process_cancel_batch`, `finalize_cancel`), so an absent owner or manager cannot strand it. |

Every role-gated call uses `require_auth()` on that role's address. Admin rotations are two-step (set_admin then accept_admin within 120,960 ledgers), the same pattern as the profile binding and the release validator, so a typo or a key nobody holds can never take over.

## Pause

The admin can pause the events contract or the profile contract. Pause blocks every event-lifecycle call; reads and admin calls continue, so an upgrade can run while paused. Designed for emergency response and upgrade windows, not routine operation.

## Upgrade

Upgrades are timelocked. The admin proposes a wasm hash and version (`propose_upgrade`), and `apply_upgrade` swaps the code no earlier than 17,280 ledgers (about a day) later on mainnet builds; testnet builds compile the wait to zero. The proposal is public for the whole window and can be cancelled. Storage shapes are planned to be additive; when a layout has to change, as it did for prize floors in 1.7.0, `migrate_events` converts old records in pages and `migrate` refuses to stamp the new version until none remain. The procedure is `docs/upgrade-runbook.md`.

## The 64 KB discipline

Every PR touches the workspace's CI size check. WASM size is reported in `make size`. We accept the discipline:

- No linear-scan getters in the contract. Off-chain indexes them.
- Short error variants; no verbose strings in contract code.
- Per-event collections in separate storage keys, not inlined into `EventRecord`.
- Event names under 25 characters (the `contractevent` macro behaves predictably in that range).
- Functions kept under Soroban's 10-parameter limit; packed structs for create_event.

When the limit is approached, the first thing we cut is more getters. When that runs out, we re-evaluate the split.
