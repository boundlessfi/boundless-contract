# Contract backlog

Source of truth for contract follow-ups. Update before opening a PR.

Legend: `[ ]` open, `[~]` in flight, `[x]` done.

---

## P0 (blocking mainnet)

- [ ] Third-party security audit (events + profile). Cost ~$30-80k for a Soroban-experienced firm. Schedule blocker for the mainnet date.
- [ ] Non-USDC token end-to-end smoke. Register testnet native XLM SAC + run `scripts/smoke/contract-non-usdc-token.ts` from boundless-nestjs.
- [ ] Mainnet admin multi-sig provisioned per `docs/admin-custody-policy.md` (3 signers, Yubikey-backed, 2-of-3).
- [ ] Mainnet deploy executed per `docs/mainnet-deploy-runbook.md`. Result committed to `deployments/mainnet.json`.

### From the 2026-06 Stellar-skill audit

See `docs/audit-2026-06-stellar-skill.md` for full findings.

- [x] **H1 + H2** — Admin/config moved to `instance()` storage; persistent reads of event-scoped data bump `extend_ttl`. Both contracts. (2026-06-04)
- [x] **H3 + H4** — Per-event `Vec<Address>` and `Map<Address, Submission>` storage replaced with per-element keys (`*Count` + `*At(idx)` + `*Slot(addr)` for O(1) membership). Submissions keyed per-(event, applicant). Caps lifted to 5,000 each after paged cancel landed. New errors `TooManyApplicants`, `TooManyContributors`. Paged read APIs `get_*_count` + `get_*_at` alongside the capped snapshot reads. (2026-06-04)
- [x] **Paged cancel_event** — Replaces single-tx cancel with `start_cancel` + `process_cancel_batch` + `finalize_cancel`. `start_cancel` snapshots the refund math and flips `EventStatus::Cancelling`; OwnerOnly branch settles inline. Batches refund up to `MAX_REFUNDS_PER_BATCH = 25` partners per tx. Finalize pays owner residual on the FullPartnerThenResidual branch and flips Cancelled. Lifts the contributor cap to 5,000. (2026-06-04)
- [x] **H5** — Two-step rotation for `set_events_contract` on profile contract. First-set stays single-step (deploy bootstrap); rotation is `propose_events_contract` + `accept_events_contract` with a ~1-day timelock and a 7-day expiry. `cancel_pending_events_contract` to prune. (2026-06-04)
- [x] **H6** — Timelocked upgrade flow (`propose_upgrade` + `apply_upgrade` + `cancel_pending_upgrade` with ~1-day timelock, 30-day expiry); on-chain `version()` view (initial `0.2.0`); `migrate()` admin one-shot per version guarded by `MigratedToVersion`. Both contracts. (2026-06-04)
- [ ] **Storage layout change shipped post-testnet-deploy.** The current testnet contract's storage layout is incompatible with the new code; redeploy required (not upgrade) until a `migrate()` entrypoint lands per H6. Recorded in `docs/mainnet-deploy-runbook.md`.

## P1 (post-launch)

- [ ] `select_winners` re-run semantics: today rejected; revisit if a real customer wants "append-only" behavior. Open question, not blocking.
- [ ] Grant committee multi-sig primitive (dedicated signer set + quorum at the contract level vs the current address-level multi-sig).
- [ ] Bounty Showdown participation badge on the boundless-profile contract for non-winning finalists.
- [ ] Multi-token support audit: verify the whitelist mechanism handles tokens with non-Stellar 7-decimal scales (currently assumed uniform).
- [x] **M1** — `select_winners` percent math now uses `remaining_escrow` snapshotted at select time so partner top-ups via `add_funds` flow to winners instead of getting trapped until cancel. (2026-06-04)
- [x] **M2** — Documented operational policy: contract-layer trustline checks aren't reliably possible in Soroban today (SAC `balance()` collapses "no trustline" and "0 balance"). Admin verifies off-chain before `set_fee_account` / `register_supported_token`; `FeeAccountUpdated` + `TokenRegistered` events are the off-chain monitoring hooks. (2026-06-04)
- [x] **M4** — Dropped `wins_count`, `submissions_count`, `applications_count`, `milestones_completed` from `Profile`. Off-chain indexer derives counters from emitted events. (2026-06-04)
- [x] **M5** — Crowdfunding `claim_milestone` requires `admin.require_auth()` in addition to `event.owner.require_auth()`. Grants stay single-auth (event owner is the grant org). (2026-06-04)
- [x] **L1** — Audit recommendation targeted the pre-H1 persistent layout. After H1 the Pending* keys live in instance storage which auto-extends and is the right home for them; the temporary-storage swap is no longer a net improvement. Decision recorded inline in storage.rs.
- [x] **L2** — `Error::NotInitialized` added to both error enums. (Shipped with H1+H2.)
- [x] **L3** — `panic_with_error!(NotInitialized)` replaces `.expect("admin not configured")`. (Shipped with H1+H2.)
- [x] **L4** — `MAX_FEE_BPS` tightened from 5000 (50%) to 1000 (10%). Per-event overrides still respect the cap. (2026-06-04)
- [x] **L5** — `EventCreated` event carries `title`. Indexers can populate listings without a follow-up `get_event` read. (2026-06-04)
- [x] **L6** — `__link_keep` removed; `evt::EVENTS_LINK_KEEP` constant gone. The events module is widely consumed (37+ `evt::` references across operation modules), so the keep-alive trick is no longer needed. (2026-06-04)
- [x] **L7** — Crowdfunding `validate_create` rejects nonzero `application_credit_cost` (reuses `Error::InvalidPillar` to stay inside the contracterror 50-variant cap). (2026-06-04)

## P2 (roadmap)

- [ ] EVM adapter (per B14 phase 3).
- [ ] Solana adapter (per B14 phase 4).
- [ ] Richer `boundless-profile` read API for the cross-pillar builder profile UI.

---

## Done

- [x] 2026-06-03 — `fee_bps_override` per-event field + `effective_fee_bps` resolver.
- [x] 2026-06-03 — `WinnersAlreadySelected` replay lock on `select_winners`.
- [x] 2026-06-03 — Grant last-milestone sweep (G4).
- [x] 2026-06-03 — Contract upgrade primitive verified end to end on testnet (upgrade tx 50a6ab65).
- [x] 2026-06-04 — Stellar-skill audit landed (`docs/audit-2026-06-stellar-skill.md`).
- [x] 2026-06-04 — H1+H2 storage migration: admin/config → `instance()`, persistent reads bump TTL, both contracts. Tests green (events 64/64, profile 2/2). Adds `Error::NotInitialized` to both error enums; replaces `.expect()` with `panic_with_error!` on the admin getter (L2/L3).
- [x] 2026-06-04 — H5 two-step events-contract rotation on the profile contract: `propose` + `accept` with a ~1-day timelock and a 7-day expiry; `cancel_pending_events_contract` prunes expired or unwanted proposals. First-set unchanged. Tests green (profile 9/9).
- [x] 2026-06-04 — H3+H4 per-element storage layout for applicants, contributors, winners; per-(event, addr) submission entries. Soft caps at 100 per list to keep cancel_event single-tx. New paged read APIs (`get_*_count` + `get_*_at`). Tests green (events 66/66, profile 9/9).
- [x] 2026-06-04 — Paged cancel: `start_cancel` + `process_cancel_batch` + `finalize_cancel`. `EventStatus::Cancelling` intermediate state. `CancellationState` snapshots refund math (non_owner_total, remaining_at_start, count_at_start, next_idx, branch). OwnerOnly branch settles inline. Lifts contributor + applicant caps to 5,000. Tests green (events 67/67, profile 9/9).
- [x] 2026-06-04 — H6 timelocked upgrade + on-chain version + migrate. `propose_upgrade(wasm_hash, new_version)` + `apply_upgrade()` (after ~1 day, before 30 days) + `cancel_pending_upgrade()` + `migrate()` (one-shot per version). `version()` view returns the on-chain semver string (initial `0.2.0`). Both contracts. Tests green (events 74/74, profile 13/13). Closes audit recommendation; the prior immediate-upgrade path is gone, callers must rotate through the timelock.
- [x] 2026-06-04 — M1 + M2 + M4 + M5. `select_winners` Single math switched to `remaining_escrow` (partner top-ups flow to winners). Operational trustline policy documented in `token_whitelist.rs` + `admin.rs` (M2). `Profile` counter fields dropped (M4). Crowdfunding `claim_milestone` requires admin auth on top of builder auth (M5). Tests green (events 76/76, profile 13/13).
- [x] 2026-06-04 — L4 + L5 + L6 + L7. `MAX_FEE_BPS` tightened to 10% (L4). `EventCreated` event carries `title` for indexer convenience (L5). `__link_keep` + `EVENTS_LINK_KEEP` removed (L6). Crowdfunding rejects nonzero `application_credit_cost` (L7). L1 skipped per inline note: post-H1 the Pending* keys are already in instance storage, so the temporary swap is no longer a net improvement. Tests green (events 76/76, profile 13/13).
- [x] 2026-06-04 — Off-chain catch-up. `deploy_and_upgrade.sh` rewritten for the timelocked upgrade flow (`propose-upgrade` + `apply-upgrade` + `cancel-pending-upgrade` + `migrate` + `status`); the old `upgrade` action errors out with migration instructions. boundless-nestjs: `EscrowContractClient` exposes `buildStartCancel` / `buildProcessCancelBatch` / `buildFinalizeCancel` + the H6 upgrade builders; orchestrator gains `beginStartCancel` + `beginProcessCancelBatch` + `beginFinalizeCancel` + `beginProposeUpgrade` + `beginApplyUpgrade` + `beginCancelPendingUpgrade` + `beginMigrate`. Paged read APIs (`get_applicant_count` / `get_applicant_at` / `get_winner_count` / `get_winner_at` / `get_contributor_count` / `get_contributor_at`). Smoke scripts updated. New `EscrowOpKind` values (START_CANCEL, PROCESS_CANCEL_BATCH, FINALIZE_CANCEL, PROPOSE_UPGRADE, APPLY_UPGRADE, CANCEL_PENDING_UPGRADE, MIGRATE) shipped in Prisma migration `20260604100000_paged_cancel_upgrade_op_kinds`. Profile.ts loses the four counter fields (M4). EventStatus gains `Cancelling`. `tsc --noEmit` clean; escrow-contract jest 125/125 green.

---

## How to use this file

Same convention as `boundless-nestjs/BACKLOG.md`. External coordination (audit firm, formal verification, Stellar SDF) goes in GitHub Issues.
