# Push payouts: feasibility and risk analysis

**Status:** proposal, not a decision
**Date:** 2026-08-08
**Question:** can we go back to pushing prize money at winner selection, instead of having winners claim it?

---

## 1. The question, and why it is being asked

Product wants the claim step gone. A winner should not have to press a button to
collect money they have already won. The money should simply arrive.

The obvious route is to change the contract: put the transfers back inside
`select_winners` so funds move at selection. Before touching the contract, this
document works out whether that is necessary, whether it is safe, and what it
would cost.

**The short answer:** the product outcome is achievable today with no contract
change at all. The contract change is a worthwhile optimisation afterwards, and
it is more feasible than the 1.3.0 decision suggests, but it is not the
prerequisite it appears to be.

---

## 2. What we actually decided in 1.3.0, and why

It is worth being precise here, because the reason we switched is not the reason
people remember.

The switch landed in `56ecff9`, 1.3.0, PR #61, on 2026-07-20. The commit states
the motivation directly:

> `select_winners` now records prizes; each winner claims in their own
> transaction via the new `claim_prize` entrypoint, **so the winner count per
> event is bounded by the winner_distribution instead of one transaction's
> resource budget.**

So the driver was **transaction resource limits**, not security and not
authorisation. Under the old push model, every transfer for every winner had to
fit inside a single `select_winners` transaction. An event with a long prize
ladder would hit the ceiling and become unpayable.

This matters because the reason widely repeated in the frontend and in product
conversation is different. The claim card in the platform explains that the
contract requires the winner's own authorisation so an organiser cannot collect
on their behalf. That is a true description of `claim_prize` as built, but it is
a **consequence** of the pull design, not the reason it was chosen. Nobody
decided that winners must authorise their own payouts as a safety property. We
should not defend a constraint we never intended.

### The part that is easy to miss

The same PR did two things at once:

1. Made `select_winners` **batchable at 50 winners per call**, with each position
   awardable exactly once and amounts anchored to the escrow baseline captured at
   the first batch.
2. Moved the transfer out into `claim_prize`.

**Batching alone solves the unbounded-winner problem.** Once selection is
batched, an event with 500 winners is 10 calls instead of 1, and no single
transaction has to carry them all. That is true whether or not the transfer
happens inside the batch. Adding the transfer back makes each winner cost more
resources, so the safe batch size falls, but it does not reintroduce the
unbounded case that motivated the change.

If that reasoning holds, then the constraint that pushed us to pull was
substantially retired by the other half of the same PR. This should be confirmed
by measurement (section 8) rather than taken on argument, but it reframes the
question from "can we undo a necessary safety decision" to "what is the right
batch size."

The commit also references "the constraints that sank PR #83" and answers them
with a no-persisted-struct-changes rule. I have not read #83 and it should be
reviewed before any design is finalised, since it is the record of an earlier
attempt that failed.

---

## 3. What changed in the protocol since

**CAP-73, Protocol 26 (Yardstick)** added a `trust` function to the Stellar Asset
Contract. It lets a contract create an asset's trustline for a G-address during a
contract invocation. The docs name our exact use case:

> The `trust` function is useful any time a contract distributes an asset to
> accounts that may not hold it yet: for example, an airdrop or payout contract
> calling `trust` before `mint` or `transfer`.

It is a no-op when the trustline already exists or when the address is a
contract, so it is safe to call unconditionally.

**But read the next paragraph of the same page carefully:**

> When a trustline is actually created, the SAC requires authorization from
> `addr`, preserving the opt-in nature of trustlines.

So CAP-73 does **not** let us push to a stranger who has never opted in. If the
recipient has no trustline, creating one still needs their signature, which is
exactly what we were trying to avoid needing. For a payout transaction submitted
by an organiser, a winner without a trustline remains unpayable.

This would be a serious problem, except for section 4.

---

## 4. The decisive fact: every payout address is ours

Boundless does not have a population of winners with arbitrary external wallets.

- The `Wallet` model stores `encryptedPrivateKey` and `encryptedDataKey`. Every
  wallet on the platform is custodial and platform-held under envelope
  encryption.
- Wallet activation is **sponsored by the platform** and creates the configured
  auto-trustline assets in the same transaction. The platform pays the 1 XLM
  account reserve and 0.5 XLM per trustline, and records each sponsorship so the
  reserve can be recycled later.
- The escrow submission path validates that the applicant's payout address
  matches the caller's managed wallet, and rejects the call when it does not.

Three consequences, and they carry the whole analysis:

1. **The trustline risk is already mitigated at the source.** Winners have USDC
   trustlines because we created them, sponsored, at activation. CAP-73 is a
   useful belt-and-braces call but not a dependency for us.
2. **The base reserve requirement is already satisfied**, by us, for the same
   reason.
3. **We hold every winner's key.** Which means the platform can already sign
   `claim_prize` on a winner's behalf. The MANAGED signing path that does exactly
   this for other operations is built and in production.

Point 3 is the finding that changes the recommendation.

---

## 5. Three options

### Option A: auto-claim on the winner's behalf. No contract change.

When results publish, the backend claims each prize for each winner using the
custodial key it already holds, through the existing MANAGED signing path.

The winner does nothing and sees money in their wallet. The product outcome is
delivered in full.

**Cost:** one transaction per winner, fee paid by the platform. Orchestration and
retries on the backend, where we already have the escrow op infrastructure,
idempotency keys and failure handling.

**Risk:** low. No contract change means no upgrade, no timelock, no migration, no
audit delta, nothing that can strand escrow. A failure is a backend retry, not a
stuck event.

**The one thing to settle:** it is the platform moving value using a user's key
without a per-instance prompt. The direction of travel is benign, since funds can
only move toward the user and only to their own wallet, but it is a custody
posture that should be written down and reflected in the terms rather than
assumed. `admin-custody-policy.md` is the right home for it.

**Limitation:** it does not simplify the contract. The claim window, the
unclaimed-prize counter and the cancel sweep all stay. And if Boundless ever
supports genuinely external wallets, this option stops covering those users.

### Option B: push inside `select_winners`, batched

Transfers move back into selection, at a measured batch size.

**What it buys:** the contract becomes simpler in principle. No claim window, no
unclaimed counter, no expiry sweep, no second entrypoint. Money moves when the
organiser locks winners, which is what the original design and the Design spec
always said should happen.

**What it costs:** every risk in section 6.

### Option C: push with a pull fallback

Attempt the transfer inside selection. If it cannot complete for a given
recipient, record the award instead and let that winner claim later.

Soroban supports this: the codebase already uses `try_` invocations for the
profile calls in `claim_prize`, precisely so a broken profile contract cannot
block a payout. The same pattern applies to a transfer that cannot land.

This is the design that survives contact with reality. It gets the default path
right, keeps a bad recipient from blocking everyone else in the batch, and keeps
a working path for external wallets if we ever support them. It costs more WASM
than Option B because both paths must exist, which runs into the size ceiling.

---

## 6. Risk register

Risks are for Options B and C unless stated. Option A carries only the custody
posture item.

| # | Risk | Severity | Notes and mitigation |
| --- | --- | --- | --- |
| R1 | **One bad recipient reverts the whole batch.** A transfer to an address that cannot receive panics, and a panic aborts the entire invocation. Under pull, a broken recipient hurts only themselves. Under push, they block every other winner in the batch and stop the organiser closing the event. | **High** | This is the single biggest argument against naive push. Mitigations, in order of strength: Option C's `try_` fallback; an off-chain preflight that checks every winner before building the batch; a payout-capability gate at submission time so the failure surfaces weeks earlier. |
| R2 | **In-flight events must not break.** Events already have recorded-but-unclaimed awards. `claim_prize` cannot simply be deleted. | **High** | Dual path until the last pre-upgrade claim window expires, which is up to 90 days from the last pre-upgrade selection. Plan the cutover as a window, not an event. |
| R3 | **Audit timing.** A third-party audit is an open P0 mainnet blocker, budgeted at $30-80k and unscheduled. The money path is the highest-value target in the contract. | **High** (schedule) | Either land this before the audit so it is covered, or accept paying for a re-review of the delta. Changing the payout path *after* an audit is the worst of the three orderings. |
| R4 | **WASM size ceiling.** 64 KB hard limit. The build measured 51,276 bytes at 1.3.0, leaving roughly 12.7 KB. We are now at 1.6.0, which removed per-event caps and added pagination, so the current headroom is unknown. Option C needs both paths resident. | **Medium-high** | Measure the 1.6.0 baseline before designing. If headroom is thin, Option C may not fit and the choice collapses to B or A. |
| R5 | **Error enum is at the 50-case XDR cap.** Any new error variant requires consolidating an existing one first. | **Medium** | Already tracked in the backlog. Budget the consolidation as part of the work, not as a surprise. |
| R6 | **Payment event timing moves.** `WinnerPaid` currently fires when money actually moves, at claim time. Under push it fires at selection. Four escrow subscribers and the indexer key off these events. | **Medium** | Audit every subscriber for timing assumptions. The upside is that the event becomes simpler to reason about, since selection and payment stop being separable. |
| R7 | **Fee incidence shifts.** Winners currently pay their own claim fee. Under push the organiser or platform pays for everyone, in one fat transaction that competes for contract resources under surge pricing. | **Medium** | Absolute cost is small, but it should be modelled rather than assumed, and it is a real cost transfer onto organisers. Note Option A shifts the same cost onto the platform. |
| R8 | **Reentrancy and call ordering.** Transfers inside a state-transition op mean external calls mid-transition. | **Medium** | Effects before interactions, the discipline `claim_prize` already follows and comments. The SAC is trusted so real exposure is low, but auditors will look here first. |
| R9 | **Partial batch failure and retry.** A batch that fails midway must not double-pay on retry. | **Medium** | The per-position award key is already the replay lock, and a revert unwinds the write and the transfer together, so on-chain retry is safe. The orchestrator still needs clean semantics for "batch 3 of 5 failed". |
| R10 | **Liveness machinery becomes vestigial.** The 90-day window, unclaimed counter, cancel gate and expiry sweep exist to guarantee escrow can never strand. With push there is nothing to strand. | **Low-medium** | Do not delete while R2 is live. Deleting later reclaims WASM space, which R4 may need. |
| R11 | **The claim window is a module constant.** Making it per-event requires a storage migration. | **Low** | Only relevant if we keep a fallback path. Already in the backlog for the next real migration window. |

---

## 7. Hard constraints to design within

- **WASM:** 64 KB ceiling. Last measured 51,276 bytes at 1.3.0.
- **Errors:** enum at the 50-case XDR cap.
- **Upgrade path:** `propose_upgrade`, roughly one day of timelock, `apply_upgrade`,
  then `migrate()`, all under a 2-of-3 admin multisig. This is not a same-day
  change, and it needs signer coordination.
- **Ledger limits:** roughly 200 disk-read entries and 200 write entries per
  transaction on Protocol 27, with the caveat from the docs that these are
  validator-settable network settings and must be verified against mainnet rather
  than quoted from documentation.
- **No persisted-struct changes**, the rule adopted in 1.3.0 after PR #83. New
  state goes under appended data keys, and there is a regression test that turns
  CI red if `Winner` gains a field.

---

## 8. What to measure before committing

Nothing below is a design decision. All of it is arithmetic we have not done, and
the answers determine which option is even available.

1. **Batch headroom with transfers included.** Simulate `select_winners` on
   testnet with the transfer in the loop and find the real per-winner cost in
   instructions, entry reads and writes, and IO bytes. This is already an open
   backlog item for the selection-only case, where the 50-per-call cap is
   explicitly described as conservative and never measured. Do both at once. The
   number we care about: does push drop the safe batch from 50 to roughly 25, or
   to 5? The first is a non-issue. The second changes the answer.
2. **Current WASM headroom at 1.6.0.** Determines whether Option C fits.
3. **Payout-address population.** Confirm from production data that every winner
   address is a platform-held wallet with a live USDC trustline, and that there is
   no legacy or edge-case path that produced anything else. The entire trustline
   argument rests on this.
4. **Real mainnet ledger limits**, read from network settings rather than docs.
5. **Fee modelling.** Cost per winner under Option A (one transaction each)
   against Option B or C (batched), including a surge-pricing allowance.

---

## 9. Recommendation

**Do Option A now. Decide on B or C after the measurements.**

The reasoning is that Option A delivers the entire product outcome, this month,
with no contract risk. Winners stop claiming. The button disappears. Money
arrives on its own. Every user is covered, because we hold every key and we
created every trustline.

Against that, Option B and C deliver no additional user-visible benefit. Their
value is engineering: a simpler contract, fewer moving parts, no vestigial
liveness machinery, and payment semantics that match what the Design spec always
described. Those are real and worth having, but they are worth having at a time
of our choosing, not on the critical path of a product change.

Sequenced:

**Phase 0, now.** Run the five measurements in section 8. Read PR #83. Cheap, and
it settles most of the argument.

**Phase 1, next.** Ship auto-claim on behalf. Update the custody policy to
describe the platform signing payout claims for users, and reflect it in the
terms. Keep manual claim reachable as a fallback so a backend failure never
leaves a winner with no route to their money.

**Phase 2, decide.** With numbers in hand, choose:

- If batch headroom is comfortable and WASM allows, do **Option C**, hybrid push
  with a pull fallback, and land it before the audit. This is the design that
  matches the original intent and survives external wallets later.
- If WASM is tight, do **Option B** with a hard off-chain preflight, accepting
  R1 as an operational control rather than a contract-level one.
- If both are tight, stay on Option A indefinitely. It works, and the cost is
  carrying machinery we no longer strictly need.

The one thing not to do is treat the contract change as a blocker for the product
change. It is not one.

---

## 10. Open questions

1. **Custody posture.** Is the platform signing a value-moving transaction on a
   user's behalf, without a per-instance prompt, something we are comfortable
   stating publicly in the terms? Option A depends on yes.
2. **External wallets.** Is supporting genuinely external payout addresses on the
   roadmap? If yes, Option C becomes materially more attractive and Option A
   becomes a stopgap. If no, Option A is close to a permanent answer.
3. **Audit sequencing.** Do we hold the audit until the payout path is final, or
   audit now and pay for a delta review later?
4. **Organiser fee incidence.** Under push, organisers pay to distribute. Is that
   acceptable, or does the platform absorb it? Note that under Option A the
   platform absorbs it by default.
5. **Failure visibility.** When an automatic payout fails for one winner, who is
   told, how fast, and what does the winner see in the meantime? This needs an
   answer in all three options, and it is a design question as much as an
   engineering one.

---

## Sources

Internal:

- `56ecff9` and PR #61, the 1.3.0 pull-model commit and its stated rationale.
- `contracts/events/src/event_ops.rs`, `select_winners` and `claim_prize`,
  including the pre-1.3.0 push-model compatibility branch.
- `BACKLOG.md`: unmeasured batch headroom, the 50-case error cap, the claim window
  constant, and M2 on contract-layer trustline checks not being reliably possible.
- `docs/ARCHITECTURE.md` for the 64 KB WASM ceiling.
- `boundless-nestjs`: the `Wallet` model, `wallet-sponsor.service.ts` sponsored
  activation with auto-trustlines, and the MANAGED signing path.

External:

- [Stellar Asset Contract: creating trustlines from a contract](https://developers.stellar.org/docs/tokens/stellar-asset-contract#creating-trustlines-from-a-contract) (CAP-73, Protocol 26)
- [Stellar Asset Contract: contract errors](https://developers.stellar.org/docs/tokens/stellar-asset-contract#contract-errors) (`TrustlineMissingError = 13`)
- [Storage strategies in production contracts](https://developers.stellar.org/docs/build/guides/storage/storage-strategies) (per-transaction entry caps, Protocol 27)
- [Fees, resource limits and metering](https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering) (multidimensional resource model, surge pricing on contract transactions)
- [Analyzing smart contract cost and efficiency](https://developers.stellar.org/docs/build/guides/fees/analyzing-smart-contract-cost) (batch transfer patterns)
