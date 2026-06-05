# Boundless contract audit — June 2026

Audit run against the freshly-installed Stellar Development Foundation
`stellar-dev-skill` (https://github.com/stellar/stellar-dev-skill), specifically
the `soroban` sub-skill: Part 3 (Security), Part 4 (Advanced Patterns), and
Part 5 (Common Pitfalls).

Scope: `contracts/events` and `contracts/profile` source. Tests, deploy
scripts, and off-chain runbooks are out of scope here; they get their own pass.

## TL;DR

Boundless ships solid Soroban hygiene: typed errors, typed `DataKey` keys,
idempotent ops via `OpSeen` in temporary storage, two-step admin rotation,
pause everywhere, comprehensive events, `__constructor`, child-op-id
derivation so cross-contract calls never collide.

The two systemic risks both come from storage choices:

1. **Persistent everywhere.** Every config / admin / event / per-user value
   sits in `env.storage().persistent()`. The skill says global / shared state
   belongs in `instance()` so that contract invocation auto-extends its TTL,
   and persistent is for per-entity data that ships with its own TTL plan.
   We've inverted the rule: nothing extends, and a bricked Admin key takes
   down the whole contract.
2. **Hot lists in single entries.** `EventApplicants(id)`, `ContributorList(id)`,
   `EventWinners(id)`, and the `Map<Address, Submission>` per event are all
   single storage entries that grow with participants. A 1,500+ entry
   hackathon will hit the 64KB entry cap; cancel_event's pro-rata loop walks
   the contributor list and could exhaust ledger limits.

Neither is a live exploit, but both are gating items before mainnet at
hackathon scale. Everything else is moderate or below.

## Severity legend

- **HIGH** — pre-mainnet blocker, or active footgun if missed.
- **MEDIUM** — recoverable, but worth fixing this quarter.
- **LOW** — polish, documentation, semantic cleanup.
- **POSITIVE** — patterns the skill explicitly recommends and we got right.

# HIGH

## H1. Admin / config in `persistent` storage, not `instance`

Files: `contracts/events/src/storage.rs:14-129`,
`contracts/profile/src/storage.rs:13-75`.

Soroban skill (Part 5, "Wrong storage type"): use `instance()` for global
config and admin; use `persistent()` for per-entity data with explicit TTL
management.

Every admin/config key uses `persistent()`:

```rust
// events
env.storage().persistent().set(&DataKey::Admin, addr);
env.storage().persistent().set(&DataKey::FeeAccount, addr);
env.storage().persistent().set(&DataKey::FeeBps, &bps);
env.storage().persistent().set(&DataKey::Paused, &paused);
env.storage().persistent().set(&DataKey::ProfileContract, addr);
```

Effects:

- Each persistent key carries its own rent footprint and needs its own
  `extend_ttl` schedule. Instance storage is one bag, auto-extended on every
  contract invocation, with one rent line.
- If `DataKey::Admin` ever archives, `require_admin` permanently fails, the
  contract is bricked, and `upgrade()` can no longer be called to migrate.
- Same risk applies to `FeeAccount`, `FeeBps`, `Paused`, `ProfileContract`,
  `DeploymentSeq`.

Fix: move every admin/config key to `env.storage().instance()`. Storage
layout is part of the upgrade migration so this should ship with a version
bump and a one-shot migration entrypoint that reads from persistent and
writes to instance for any installed deployment. Per-event records and
per-user profiles can stay persistent (they're per-entity).

## H2. No `extend_ttl()` calls anywhere

Files: ALL `storage.rs` getters/setters, both contracts.

Soroban skill (Part 5, "Storage TTL not extended"): persistent storage must
be proactively bumped or the entry archives and is unreadable.

A grep over the repo turns up zero `extend_ttl` calls. Long-lived data at
risk:

- `Event(u64)` — escrow holdings for a multi-month hackathon
- `Profile(Address)` — every user's credits + reputation
- `EarningsByToken(user, token)` — per-user earnings record
- `MilestoneClaimed(event, recipient, milestone)` — replay protection on
  grant payouts
- `EventWinners(u64)` — anchor + payout audit trail
- `CrowdfundingMilestonesClaimed(u64)` — divisor for dynamic-split math

Archival of any of these silently breaks the corresponding flow. For
crowdfunding specifically, archived `CrowdfundingMilestonesClaimed` resets
the divisor and the next claim pays the entire remaining escrow.

Fix: on every read of long-lived persistent data, call
`extend_ttl` with thresholds matched to expected lifetime (sample numbers,
verify against current network parameters):

```rust
const EVENT_TTL_BUMP: u32 = 535_680;    // ~31 days
const EVENT_TTL_THRESHOLD: u32 = 17_280; // ~1 day

env.storage()
    .persistent()
    .extend_ttl(&DataKey::Event(id), EVENT_TTL_THRESHOLD, EVENT_TTL_BUMP);
```

The instance-storage migration (H1) covers admin/config TTL because
instance storage auto-extends on every invocation. Persistent reads still
need explicit bumps.

## H3. `EventApplicants(u64)`, `ContributorList(u64)`, `EventWinners(u64)` are single growable entries

File: `contracts/events/src/storage.rs:120-189`.

Each list is stored as one `Vec<Address>` or `Vec<Winner>` per event.
Soroban entries cap around 64KB; an Address is ~40 bytes, so ~1,500 entries
per list is the realistic ceiling. A community hackathon easily exceeds
that.

Two follow-on effects:

- `cancel_event` walks `ContributorList` doing per-entry storage reads of
  `ContributorAmount`. At 1,500 contributors that's 1,500 reads in a single
  invocation. Soroban resource limits will reject this even before the size
  cap hits.
- `select_winners` walks `EventWinners` to check the replay anchor, but
  caps at `MAX_WINNERS_PER_SELECT = 50` — that one's bounded.

Fix: split each list into per-element keys with an index counter.

```rust
// proposal — index + per-entry storage
DataKey::EventApplicantCount(u64)
DataKey::EventApplicant(u64, u32) -> Address
DataKey::EventApplicantIndex(u64, Address) -> u32  // for O(1) lookup
```

`cancel_event` then becomes a paged loop over `count`, but each iteration
is its own storage entry, so the resource budget scales linearly with
ledger-cost not entry-size. For a hackathon with > 5k contributors the
cancel itself needs to be paged by the caller (admin-controlled, multi-tx).

## H4. `EventSubmissions(u64)` is a single `Map<Address, Submission>` per event

File: `contracts/events/src/storage.rs:160-180`.

```rust
pub fn get_submission(env: &Env, id: u64, applicant: &Address) -> Option<Submission> {
    let map: Option<soroban_sdk::Map<Address, Submission>> = env
        .storage()
        .persistent()
        .get(&DataKey::EventSubmissions(id));
    map.and_then(|m| m.get(applicant.clone()))
}
```

Every read deserializes the entire map. Every `submit` deserializes and
reserializes the entire map. A 500-submission hackathon means each
submit/withdraw pays the gas of 500 entries.

Also runs into the 64KB entry cap for content_uri strings.

Fix: key per-submission, same shape as H3:

```rust
DataKey::Submission(u64, Address) -> Submission
DataKey::SubmissionCount(u64) -> u32  // for enumeration
```

## H5. `set_events_contract` on profile contract is single-step

File: `contracts/profile/src/admin.rs:73-81`.

```rust
pub fn set_events_contract(env: &Env, new_addr: Address) -> Result<(), Error> {
    require_admin(env)?;
    storage::set_events_contract(env, &new_addr);
    evt::EventsContractUpdated { new_addr: new_addr.clone() }.publish(env);
    Ok(())
}
```

The comment says "single-step; admin multisig is the protection." But
`require_events_contract` is what gates every credit, reputation, and
earnings mutation. Whoever holds the configured events-contract address can
mint credits, slash reputation, and rewrite earnings for any user.

A wrong or malicious address gets full control over the user-incentive
ledger. The skill (Part 3, vulnerability category 1) lists
authorization-via-trusted-caller as one of the top categories to harden.

Fix: rotate this through the same two-step pattern as `set_admin` — write
`PendingEventsContract`, require the new contract to call
`accept_events_contract` from its own admin (or hold for N ledgers before
auto-accepting). Keeps off-chain multisig as the first layer, on-chain
delay as the second.

## H6. Upgrade has no on-chain timelock or version tracking

Files: `contracts/events/src/admin.rs:upgrade`,
`contracts/profile/src/admin.rs:107-113`.

Skill Part 4 (Advanced Patterns, "Upgradeability") recommends:

- Explicit upgrade policy with timelock
- Version tracking (`version()` view + storage history)
- Migration entrypoint separate from upgrade

We have:

- `upgrade(new_wasm_hash)` requires admin auth and calls
  `env.deployer().update_current_contract_wasm(...)` immediately. No delay,
  no announcement window.
- `contractmeta!(key = "version", val = "0.1.0")` is baked into the wasm
  at compile time; it does not update on upgrade and is not callable.
- No migration entrypoint. Storage-layout changes need explicit migration
  code, run once post-upgrade. Today that's hand-coded each time.

Fix:

1. Add `Version` to `DataKey`, write it in `__constructor`, and bump it in
   `upgrade()`. Expose `version()` view.
2. Add `PendingUpgrade { wasm_hash, available_at_ledger }`. Add
   `propose_upgrade(hash)` admin-only, then `apply_upgrade()` after the
   delay (or anyone calls after the delay).
3. Add `migrate(from_version, to_version)` admin-only one-shot with a
   `MigrationApplied(from, to)` storage marker so the migration can't
   double-run.

Off-chain multisig (per `docs/admin-custody-policy.md`) covers the
exfiltration risk today. The timelock is what gives users a window to
react to a bad upgrade.

# MEDIUM

## M1. `select_winners` divides percent of `total_budget`, not `remaining_escrow`

File: `contracts/events/src/event_ops.rs:580-595`.

```rust
let percent = event.winner_distribution.get(spec.position).unwrap() as i128;
let amount = event.total_budget.saturating_mul(percent) / 100_i128;
```

For non-Crowdfunding pillars `add_funds` increases `remaining_escrow` past
`total_budget`. Winners are paid the percent of original budget, partners'
top-ups stay trapped until cancel.

Two coherent fixes; either works but you need to pick:

- (A) Pay against `remaining_escrow`. Top-ups boost winner payouts.
- (B) Grow `total_budget` inside `add_funds` so the percent math reflects
  the topped-up pool.

Today is neither — top-ups grow the escrow but don't change anything else.
This is a product decision more than a security one, but the contract
should match the product policy explicitly.

## M2. `set_fee_account` doesn't verify trustline

File: `contracts/events/src/admin.rs:set_fee_account`. Comment in
`token_whitelist.rs:6-8` documents that this is "admin's off-chain
responsibility."

If admin sets a fee account without a trustline for some whitelisted
token, the next `deposit_with_fee_at` for that token fails inside
`client.transfer(&contract, &fee_account, &fee)`. Whole event creation /
add_funds tx reverts.

Self-DoS, not loss-of-funds. Admin recovers by rotating to a properly
trustlined account.

Fix options:

- Strictest: trustline check per token whitelist entry whenever
  `set_fee_account` or `register` token is called.
- Cheaper: emit an event-only "FeeAccountSetWarning" with a per-token
  trustline check the admin can call out-of-band.

Adding a trustline-existence read on token register is the cleanest spot
(we already mutate token state there, and the admin is the caller).

## M3. No on-chain version function

See H6. Worth listing separately because the `version()` view is the lowest
effort piece — it doesn't need timelock infrastructure.

## M4. Profile-side counters are dead fields

File: `contracts/profile/src/types.rs:9-17`.

```rust
pub struct Profile {
    pub bootstrapped_at: u64,
    pub credits: u32,
    pub reputation: u64,
    pub wins_count: u32,
    pub submissions_count: u32,
    pub applications_count: u32,
    pub milestones_completed: u32,
}
```

`wins_count`, `submissions_count`, `applications_count`,
`milestones_completed` are written in `Profile::new(...)` as 0 and never
incremented anywhere. Either we removed the increment paths intentionally
or they were never wired. Cost: extra serialization bytes on every profile
read/write, plus reader confusion.

Fix: either wire increments inside the corresponding profile-side
mutations (`spend` → `applications_count += 1`, etc.) and bump version, or
drop the fields and migrate the existing rows on the same upgrade.

## M5. `crowdfunding::claim_milestone` has no on-chain milestone validation

File: `contracts/events/src/grant.rs:64-211`. Comment says the off-chain
layer admin-co-signs the builder's claim, but the contract requires only
`event.owner.require_auth()` — and for crowdfunding the owner is the
builder.

Today this is fine because abstracted wallets hold the signing key on the
platform side. If the wallet model ever surfaces builder-controlled keys
(self-custody mode, hardware-wallet integration, anything), the contract
has zero defense: a builder can immediately drain their own escrow on
milestone 1 before community validation happens.

Fix: require both the builder and an admin-designated validator address
(`require_auth` on both). The admin can configure the validator per event
at create time or per-pillar globally.

Decision is product-level; flagging now while abstracted wallets are still
the default and the change is cheap.

# LOW

## L1. `PendingAdmin` should be `temporary` storage

Files: `contracts/events/src/storage.rs` (events PendingAdmin),
`contracts/profile/src/storage.rs:25-37`.

PendingAdmin carries `expires_at_ledger`; once expired, the entry is dead
weight. `temporary()` storage auto-expires; persistent storage doesn't.
Cheaper, cleaner.

## L2. `storage::get_admin` returns `Error::AlreadyInitialized` on missing

Files: `contracts/events/src/storage.rs:get_admin`,
`contracts/profile/src/storage.rs:14-19`.

```rust
pub fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(Error::AlreadyInitialized)
}
```

Reversed semantic. Should be `Error::NotInitialized` (which doesn't exist
yet — add to enum). Misleads operators reading logs.

## L3. `admin::get_admin` uses `.expect("admin not configured")`

Files: `contracts/events/src/admin.rs:get_admin`,
`contracts/profile/src/admin.rs:118-120`.

Skill rule: typed errors via `contracterror`. `.expect` produces a string
panic that off-chain clients can't structurally interpret.

Fix:

```rust
pub fn get_admin(env: &Env) -> Address {
    storage::get_admin(env)
        .unwrap_or_else(|_| panic_with_error!(env, Error::NotInitialized))
}
```

Or have `storage::get_admin` already do the `panic_with_error!`.

## L4. `MAX_FEE_BPS = 5_000` is permissive

File: `contracts/events/src/admin.rs:MAX_FEE_BPS`.

50% is a soft cap. Operationally we'd never run there; sales-side tier is
single-digit percent. Tighten to 1_000 (10%) so a config typo can't ship
half-fees. The override path still respects the cap, so this propagates.

## L5. `EventCreated` event omits `title`

File: `contracts/events/src/events.rs:14-21`. Indexer needs title for
listings; today it has to fetch the event record separately to populate
a card. Add `title: String` to the event struct.

## L6. `__link_keep` is publicly callable

File: `contracts/events/src/lib.rs`.

It's a no-op linker keep-alive. No state changes. Pure DoS surface (waste
caller gas, that's it). Worth at minimum gating it behind a `#[cfg(test)]`
or making it a private constant if the linker permits.

## L7. Crowdfunding validate_create ignores `application_credit_cost`

File: `contracts/events/src/crowdfunding.rs:30-60`. Crowdfunding has no
apply flow; any nonzero `application_credit_cost` is silently irrelevant.
Reject nonzero values explicitly so wallet UIs and indexers don't get
confused.

# POSITIVE

These match the skill's recommendations cleanly. Listed so we don't lose
them in a refactor.

- **P1.** Protocol 22+ `__constructor` pattern in both contracts.
- **P2.** Typed `DataKey` enum prevents storage key collisions
  (skill Part 3, "Storage Security").
- **P3.** `OpSeen(BytesN<32>)` idempotency markers stored in `temporary()`
  with auto-TTL — correct storage type per skill Part 5.
- **P4.** Two-step admin rotation with TTL on `PendingAdmin`, including
  the `expires_at_ledger` check inside `accept_admin` (skill Part 3,
  vulnerability category 1).
- **P5.** Reinitialization protection via `has(&DataKey::Admin)` check in
  every `initialize` / `__constructor` path (skill Part 3, vulnerability
  category 6).
- **P6.** Comprehensive contract-event emission on every state change
  (skill Part 3, "Auditability").
- **P7.** Pause / unpause / `require_not_paused` consistently invoked at
  the top of every state mutator on both contracts (skill Part 3, category
  7).
- **P8.** Token whitelist with admin-gated `register` / `deregister`,
  plus per-event `require_supported` check (skill Part 3, "Cross-Contract
  Calls" — restricting external token surface).
- **P9.** Child op_id derivation (`derive_child` / `derive_child_indexed`)
  gives every cross-contract call its own idempotency marker so the
  profile-side `OpSeen` set never collides with the events-side parent op.
- **P10.** Per-pillar `validate_create` functions keep lifecycle invariants
  (release kind, deadline-required, distribution shape) co-located with the
  pillar logic. Refactor-friendly.
- **P11.** `select_winners` does a two-pass total-owed check before any
  transfers — `InsufficientEscrow` fires before money moves.
- **P12.** `claim_milestone` last-milestone sweep math
  (`already_claimed + 1 == total_milestones ⇒ pay remainder`) avoids
  stranded dust on per-recipient grant payouts.
- **P13.** All public functions return typed `Result<_, Error>`, never
  raw panics from user-facing paths (skill Part 3, "Error handling").
- **P14.** `ProfileClient` defined via `#[contractclient]` typed trait —
  no string-named cross-contract calls (skill Part 3, "Cross-Contract
  Calls").
- **P15.** `saturating_*` arithmetic throughout. Skill Part 3 category 4
  (integer overflow) gets shut down structurally.
- **P16.** Profile contract gates ALL credit / reputation / earnings
  mutations on `require_events_contract` — single trusted-caller surface,
  documented (skill Part 3, "Authorization").
- **P17.** Partners-first refund priority on `cancel_event` with explicit
  case A / case B math, no rounding gives the owner contributor money.
- **P18.** Per-event `fee_bps_override` snapshotted at create time and
  reused by `add_funds` — fee model stays stable even if the contract
  default changes mid-flight.
- **P19.** `withdraw_application` blocks when a submission already exists,
  closing the "withdraw, refund credits, resubmit free" loop.
- **P20.** Hackathon `validate_create` requires a deadline (open-ended
  hackathons would break the submission window).

# Recommended fix order

Pre-mainnet blockers:

1. H1 + H2 together. Storage-type migration is one change set, includes
   the TTL-extension plumbing on persistent reads. Single upgrade tx
   path. **Ship before mainnet.**
2. H5. Two-step rotation for `set_events_contract`. Small, isolated,
   high-leverage on the profile contract's auth model. **Ship before
   mainnet.**
3. H3 + H4. Storage layout change for the per-event lists and
   submissions. Heavier — needs paged read/write helpers and a
   migration. **Ship before any hackathon > 500 participants.**

Q3 polish:

4. H6 timelock + version() + migrate() infrastructure. Cuts time-to-react
   on bad upgrades from "next admin tx" to "delay window."
5. M1 product decision: total_budget vs remaining_escrow basis for
   `select_winners`.
6. M5 builder-claim double-auth before any non-abstracted-wallet mode
   ships.
7. M4 dead counters: pick wire-up or remove, then version-bump.

Backlog:

8. M2 trustline check on `set_fee_account` / token register.
9. L1 PendingAdmin storage type change (gets folded into the H1 migration).
10. L2 / L3 error semantics + `NotInitialized` enum variant.
11. L4 tighten MAX_FEE_BPS to 1000.
12. L5 add title to EventCreated.
13. L6 hide `__link_keep`.
14. L7 reject nonzero `application_credit_cost` for Crowdfunding.

# Methodology

- Read every source file in `contracts/events/src` and
  `contracts/profile/src`. No grep-only checks.
- Cross-referenced findings against the soroban sub-skill Part 3, 4, 5.
- Severity calibrated for a contract that holds production escrow at
  hackathon scale.
- No off-chain claims taken at face value (e.g., "admin signs on behalf of
  the builder") — every authorization invariant verified at the contract
  layer first.

Out of scope: tests under `contracts/*/src/tests/`, deploy scripts,
off-chain backend, admin runbooks. Each gets its own audit pass.
