# Contract backlog

Source of truth for contract follow-ups. Update before opening a PR.

Legend: `[ ]` open, `[~]` in flight, `[x]` done.

---

## P0

- [ ] Third-party security audit (events + profile). Cost ~$30-80k for a Soroban-experienced firm. Scope it to the 2.0.0 / 1.2.1 tree.
- [ ] **Upgrade mainnet to events 2.0.0 + profile 1.2.1** (`docs/upgrade-runbook.md`, Section 8). Carries every threat-model fix (`docs/threat-model.md` v1.1, Section 4.1.2), restores the 17,280-ledger upgrade timelock (EoP.10: the deployed 1.7.0 / 1.2.0 builds have none), and finishes the 1.7.0 migration that never ran: events still reports a 1.6.0 marker, and events 1 and 2 (`…609`, `…610`, both Completed with zero escrow) are still in the old layout and fail to decode. Rehearsed on testnet 2026-10-02 (events `CBEODVJG…` at 2.0.0, 185 events through `migrate_events`, `migrate` stamped). Backend first: read pages of 90, refund cranks of 15, hackathon selections split at 20 per call, `forfeit_milestone` on milestone rejection, `claim_refund` for held refunds, and no calls to the removed participation entry points. Then profile propose / apply / migrate, and events pause / propose / apply / one `migrate_events` page / `migrate` / unpause, all under the deployed zero timelock. `deploy_mainnet.sh` now refuses to unpause events until `migrate` has stamped, the gate the August upgrade lacked.

## P1

- [ ] Per-event prize claim window: `PRIZE_CLAIM_WINDOW_SECS` (90 days) is a module constant because adding a field to `EventRecord` requires a storage migration. Move it onto `EventRecord` at the next real migration window, per the per-event-config rule in `CLAUDE.md`.
- [ ] `Error` enum headroom: events uses 42 of the 50-case `contracterror` cap after 2.0.0, and retired codes are never reused. Plan a consolidation before the cap, not when the next variant fails to compile.
- [ ] Grant committee multi-sig primitive (dedicated signer set + quorum at the contract level vs the current address-level multi-sig).
- [ ] Multi-token support audit: verify the whitelist mechanism handles tokens with non-Stellar 7-decimal scales (currently assumed uniform).
- [ ] Decide whether to appoint a crowdfunding release validator on mainnet once 2.0.0 is live, and who holds the key (`docs/admin-custody-policy.md` §4.1). Until one accepts, the admin multisig co-signs every crowdfunding release.

## P2 (roadmap)

- [ ] EVM adapter (per B14 phase 3).
- [ ] Solana adapter (per B14 phase 4).
- [ ] Richer `boundless-profile` read API for the cross-pillar builder profile UI.

---

## Done

- [x] 2026-10-02: measured batch headroom on the test host, which enforces mainnet limits (100 footprint entries, 50 writes): single-release selection 19 + 4 per award, grant selection 15 + 2 per award, refund crank 11 + 5 per refund, read page 3 + 1 per row. Caps are now 20 single-release awards per call, 40 per grant, 15 refunds per crank and 90-row pages, each tested at its maximum (`grant_scenarios::limits`).
- [x] 2026-10-02: upgrade timelock restored in source (events 2.0.0, profile 1.2.1): 17,280 ledgers on mainnet builds, 0 with `--features testnet`; tests assert `UpgradeTimelockNotElapsed`.
- [x] 2026-09-28: on-chain participation removed in events 2.0.0 (`apply`, `apply_to_bounty`, `remove_applicant`, `submit` and their registries); entries live in boundless-nestjs.
- [x] 2026-07-30: mainnet events 1.1.0 to 1.6.0 (propose, apply, migrate).
- [x] 2026-07-24: SDK 23 to SDK 27 storage compatibility suite (`docs/history/mainnet-sdk27-upgrade-2026-07.md`); now the storage compatibility suite for 2.0.0 (`docs/storage-compatibility.md`).
- [x] 2026-07-18: `select_winners` batchable for single-release events (1.3.0, #61). Each position is awardable once (the `EventPrizeAward` key is the replay lock) and winners collect with `claim_prize`.
- [x] 2026-07-17: cancellation liveness. `add_funds` keeps an O(1) per-event non-owner contribution total, `start_cancel` no longer scans contributors, and `process_cancel_batch` / `finalize_cancel` are permissionless once cancellation starts. A missing total with existing contributors fails closed.
- [x] 2026-07-03: on-chain credits removed (events and profile 1.0.0 to 1.1.0 on mainnet). Credits are an off-chain ledger in boundless-nestjs.
- [x] 2026-06-27: mainnet deployed (events `CCFVEGOQ…`, profile `CD3KH4OE…`). Admin of both is the 2-of-3 multisig `GCVK72I6…` (thresholds 0/2/2, master weight 0), checked by `scripts/admin/verify-multisig.sh`.
- [x] 2026-06-05: testnet redeploy and smoke battery against the audited surface (paged cancel, H6 migrate replay, token enumeration), plus a non-USDC end-to-end smoke with the testnet native XLM SAC.
- [x] 2026-06-04: June 2026 Stellar-skill audit fixed: H1 + H2 (instance storage, TTL bumps), H3 + H4 (per-element storage for per-event lists), paged cancel, H5 (two-step events-contract rotation on profile), H6 (timelocked upgrade, `version()`, `migrate()`), M1, M2 (trustline check is operational, not on chain), M4, L1 to L6. Report: `docs/history/audit-2026-06-stellar-skill.md`.
- [x] 2026-06-03: `fee_bps_override` per event, `WinnersAlreadySelected` replay lock, grant last-milestone sweep (G4).

---

## How to use this file

Same convention as `boundless-nestjs/BACKLOG.md`. External coordination (audit firm, formal verification, Stellar SDF) goes in GitHub Issues.
