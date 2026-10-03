# Boundless Contracts — Scout Security Scan Report

**Tool:** [Scout by CoinFabrik](https://github.com/CoinFabrik/scout) (`cargo-scout-audit`)
**Version:** 0.3.x (nightly-2025-08-07 toolchain)
**Date:** June 2026 (re-run after remediation)
**Scope:** `contracts/events` + `contracts/profile`
**Command:** `cargo scout-audit` (run from workspace root)

> **Historical note (post-scan):** this report describes the contracts as
> scanned, at version 1.0.0, when per-user credit balances were still
> on-chain (`profile/src/credits.rs`). The 1.0.0 → 1.1.0 upgrade (2026-06)
> removed on-chain credits — they are now an off-chain ledger in
> boundless-nestjs — so `credits.rs` and its findings (e.g. C-1 / C-6) no
> longer have an on-chain counterpart. The 1.7.0 → 2.0.0 upgrade removed
> the on-chain participation records in the same way, so the findings that
> touch `apply()`, `remove_applicant()`, `applicant_at()`,
> `applicant_count()` and `applicants_snapshot()` no longer have one
> either. References below are preserved unedited as the record of the
> scan.

---

## Summary

**Initial scan (before remediation):**

| Severity | Findings | Real | False Positive |
|---|---|---|---|
| CRITICAL | 11 | 5 | 6 |
| MEDIUM | 21 | 0 | 21 |
| ENHANCEMENT | 50 | 0 (advisory) | — |

**Post-remediation scan:**

| Severity | Remaining | Real | False Positive |
|---|---|---|---|
| CRITICAL | 5 | 0 | 5 |
| MEDIUM | 21 | 0 | 21 |
| ENHANCEMENT | 50 | 0 (advisory) | — |

All real CRITICAL findings have been remediated. The 5 remaining CRITICAL and all 21 MEDIUM are documented false positives. See sections below.

---

## CRITICAL Findings

### C-1 — Subtraction underflow: `profile/src/credits.rs:81`

```rust
profile.credits -= amount;
```

**Assessment: False positive.** Lines 78–80 guard with:
```rust
if profile.credits < amount {
    return Err(Error::InsufficientCredits);
}
```
Scout does not trace the guard above the operation. Underflow is impossible at this callsite. No change required.

---

### C-2 — `update_current_contract_wasm` without access control: `profile/src/admin.rs:262`

**Assessment: False positive.** This call is inside `apply_upgrade()`, which begins with `admin.require_auth()`. Scout does not trace access control through the calling function. The upgrade is fully admin-gated and also requires the timelocked proposal to have elapsed.

---

### C-3 — Addition overflow: `idempotency.rs:35,36`

```rust
let id = storage::get_next_event_id(env, base + 1);
storage::set_next_event_id(env, id + 1);
```

**Assessment: Real.** While overflow of a `u64` event ID counter is practically impossible (~18 quintillion events), Scout correctly identifies bare `+` on unsigned integers. **Fixed:** replaced with `saturating_add(1)` on both lines. Tests: 80/80 passing.

---

### C-4 — Bitwise XOR `^` flagged as potential exponentiation error: `idempotency.rs:60,74,75`

```rust
payload[0] ^= op_tag;
payload[1] ^= sub_idx;
```

**Assessment: False positive.** These are intentional XOR operations for child op_id derivation, explicitly documented in comments:
> "XOR with a per-op tag in the first byte: cheap, deterministic, and the orchestrator's sha256-based parent op_ids make collisions effectively impossible."

Rust has no `**` exponentiation operator; `^` is always XOR in Rust. No change to logic required.

---

### C-5 — Subtraction underflow: `events/src/storage.rs:303,305,327`

```rust
let idx = slot - 1;          // line 303
let last_idx = count - 1;    // line 305
let new_count = count - 1;   // line 327
```

**Assessment: False positive.** All three operations are inside `remove_applicant()` which begins with:
```rust
if slot == 0 { return Err(Error::ApplicantNotApplied); }
```
So `slot >= 1` at line 303. `count >= 1` follows because a non-zero slot implies at least one registered applicant. Scout does not trace these bounds through the guard above. No code change required.

---

## MEDIUM Findings

### M-1 — Unsafe `.unwrap()` on Map: `event_ops.rs:693,707`

```rust
let percent = event.winner_distribution.get(spec.position).unwrap() as i128;
```

**Assessment: Real.** Although the first pass validates all positions via `.is_none()` at line 670, the second pass uses `.unwrap()` which is non-idiomatic and Scout correctly flags it. **Fixed:** replaced both with `.ok_or(Error::InvalidDistribution)?`. Tests: 80/80 passing.

---

### M-2 — Unsafe `.expect()`: `storage.rs:309`

```rust
let last_addr = applicant_at(env, id, last_idx).expect("count > 0 implies last present");
```

**Assessment: Real.** Even though the reasoning is sound, `expect()` triggers a host-level panic rather than a typed contract error. **Fixed:** replaced with `.ok_or(Error::EventNotFound)?`. Tests: 80/80 passing.

---

### M-3 — Unsafe Map access: `crowdfunding.rs:51`, `event_ops.rs:670`, `grant.rs:155`

**Assessment: False positives.** All three use `ok_or(Error::...)` or `.is_none()` — the safe optional access pattern. Scout's multi-line expression parser tagged the method chain start rather than the terminal accessor. Example from `crowdfunding.rs`:
```rust
let percent = record
    .winner_distribution
    .get(1)
    .ok_or(Error::InvalidDistribution)?;   // safe — ? propagates error
```

---

### M-4 — Unbounded operations: `event_ops.rs:346,642`, `grant.rs:115`, `storage.rs:341,416`

**Assessment: Confirmed for the original `start_cancel`; remediated in events 1.2.0.** The contributor cap of 5,000 did not make the old snapshot loop safe because every iteration read both the contributor address and amount. The transaction could exceed Soroban resource limits well before reaching the cap.

The 1.2.0 fix maintains `NonOwnerContributionTotal` in `add_funds`, so `start_cancel` snapshots cancellation math with one aggregate read. Mainnet was verified to contain no event rows before this storage change. Older zero-contributor rows initialize the key lazily, while a missing total with existing contributors fails closed. Each crank processes at most 25 contributor slots. `process_cancel_batch` and `finalize_cancel` are permissionless after cancellation starts, removing manager availability as a second refund-liveness dependency. Documented in `docs/threat-model.md` under DoS.2.

---

### M-5 — Transfer amount not checked against minimum: `escrow.rs:34,84`

**Assessment: False positive for this context.** Scout suggests a minimum transfer amount check to prevent front-running. The amounts here are computed by the contract itself (fee math on `deposit_with_fee`, exact escrow amounts on `release`) — not user-supplied values. An organizer-set `total_budget` is validated at `create_event` time (must be > 0). No additional check is needed.

---

### M-6 — Dynamic types in storage: `profile/storage.rs:140,162`, `events/storage.rs:145,167`

**Assessment: Low risk, advisory.** The flagged calls store Soroban `String` values for the contract version (e.g., `"0.2.0"`). These are bounded semver strings written only by the admin-gated `apply_upgrade` / `migrate` paths — not unbounded user input. The theoretical growth risk is negligible. No change required for testnet; may convert to a fixed-size encoding before mainnet if auditors require it.

---

### M-7 — Storage operation without access control: `storage.rs:343`

**Assessment: False positive.** The flagged storage operation is a helper function called exclusively by `apply()` (bounty application flow), which has `applicant.require_auth()` in its caller. Scout does not trace call hierarchy for access control. No change required.

---

### M-8 — Vec/Map parameters without content validation: (events contract)

**Assessment: Advisory.** The `winners: Vec<WinnerSpec>` parameter in `select_winners` is validated in the function body (position bounds check, total amount check, non-empty check). No unvalidated Vec is stored raw from user input. No change required.

---

### M-9 — Storage push_back without access control: `storage.rs:343,418,498`

**Assessment: False positive.** `out.push_back(addr)` and `out.push_back(w)` inside `applicants_snapshot`, `winners_snapshot`, and `contributors_snapshot` are building an in-memory `Vec` (a local variable, not a storage write). Scout flags `push_back` as a storage mutation but these are plain Soroban SDK Vec accumulations. The snapshot functions are read-only helpers -- they do not write to contract storage. No change required.

---

## ENHANCEMENT Findings (Advisory Only)

| Finding | Count | Assessment |
|---|---|---|
| Use latest Soroban version (was 23.5.2, latest 27.0.0) | 2 | **Resolved.** Bumped `soroban-sdk` to `27.0.0` (workspace `Cargo.toml`), Rust toolchain to `1.91.0` (`rust-toolchain.toml`), and build target to `wasm32v1-none`. All 197 events + 66 profile tests pass. |
| Emit events when storage is modified (profile + events contracts) | ~44 | **No action.** Scout flags `lib.rs` dispatcher functions, which are thin wrappers. The underlying implementation modules (`event_ops.rs`, `grant.rs`, `crowdfunding.rs`, `admin.rs`, `reputation.rs`, etc.) emit a typed Soroban event for every state-changing operation. Scout cannot trace through function calls. All flags verified by code inspection. The one genuinely missing event (`ManagerChanged`) is tracked in issue #3 and is not part of this scanner batch. |

---

## Remediation Round 2 (June 2026)

Five additional fixes applied after the second Scout run:

**C-6 (Real) -- Subtraction underflow: `profile/credits.rs:81`**
```rust
// Before
if profile.credits < amount {
    return Err(Error::InsufficientCredits);
}
profile.credits -= amount;

// After
profile.credits = profile.credits
    .checked_sub(amount)
    .ok_or(Error::InsufficientCredits)?;
```
Simplified to a single checked operation. The separate guard was redundant. Scout correctly identified the bare subtraction. Fixed.

**C-7 (Real) -- Subtraction underflow: `events/storage.rs:303,305,327`**
```rust
// Before
let idx = slot - 1;
let count = applicant_count(env, id);
let last_idx = count - 1;
// ...
let new_count = count - 1;

// After
let idx = slot.checked_sub(1).ok_or(Error::ApplicantNotApplied)?;
let count = applicant_count(env, id);
let last_idx = count.checked_sub(1).ok_or(Error::EventNotFound)?;
// ...
let new_count = count.checked_sub(1).ok_or(Error::EventNotFound)?;
```
Although the `if slot == 0` guard above makes underflow practically impossible, the bare `- 1` operations are not idiomatic Soroban and Scout correctly flags them. Replaced with `checked_sub(1).ok_or()` throughout `remove_applicant()`. Fixed.

---

## Post-Remediation Scan Summary

After all five remediation rounds (C-3, M-1, M-2 in round 1; C-6, C-7 in round 2):

**Scan result (events):** 4 CRITICAL (all false positives), 19 MEDIUM (all false positives), 26 ENHANCEMENT (advisory)
**Scan result (profile):** 1 CRITICAL (false positive), 2 MEDIUM (false positives), 24 ENHANCEMENT (advisory)

- 0 real CRITICAL findings
- 0 real MEDIUM findings
- All remaining warnings are documented false positives or advisory enhancements

**Tests post-remediation:** `cargo test --all` -- 226 passed, 0 failed (149 events + 77 profile).

---

## False Positive Registry

For audit panel reference -- these Scout warnings require no code change:

| ID | File | Line | Reason |
|---|---|---|---|
| C-2 | `profile/admin.rs` | 262 | Inside admin-gated `apply_upgrade()`; full timelocked flow |
| C-2b | `events/admin.rs` | (upgrade) | Same pattern in events contract `apply_upgrade()` |
| C-4 | `events/idempotency.rs` | 60,74,75 | Intentional XOR for op_id derivation; documented in comments |
| M-3 | `crowdfunding.rs`, `event_ops.rs`, `grant.rs` | various | All use `.ok_or()?` safe accessor -- Scout truncates multi-line span and misses the terminal `.ok_or()` |
| M-4 | `event_ops.rs`, `grant.rs`, `storage.rs` | various | Confirmed for original `start_cancel`; remediated in events 1.2.0 with O(1) aggregate snapshots |
| M-5 | `escrow.rs` | 34,84 | Amounts are contract-computed, not user-supplied |
| M-6 | `profile/storage.rs`, `events/storage.rs` | various | Bounded semver strings written by admin-only paths |
| M-7 | `storage.rs` | 343 | Called only from auth-gated `apply()` |
| M-8 | `event_ops.rs` | — | Vec validated in function body |
| M-9 | `storage.rs` | 343,418,498 | In-memory Vec accumulation in read-only helpers; not a storage write |
| ENHANCEMENT (soroban_version) | `Cargo.toml` | — | Resolved: bumped to soroban-sdk 27.0.0 / Rust 1.91.0 / wasm32v1-none target. |
| ENHANCEMENT (storage_change_events, ~44 flags) | `lib.rs` dispatcher functions | — | No action: dispatcher wrappers delegate to impl modules that already emit events. Scout cannot trace through function calls. `ManagerChanged` is the only genuine gap; tracked in issue #3. |

**Fixed findings (no longer in scan output):**

| ID | File | Fix |
|---|---|---|
| C-3 | `events/idempotency.rs:35-36` | `saturating_add(1)` on u64 event ID counter |
| C-6 | `profile/credits.rs:81` | `checked_sub(amount).ok_or(Error::InsufficientCredits)?` |
| C-7 | `events/storage.rs:303,305,327` | `checked_sub(1).ok_or()` in `remove_applicant()` |
| M-1 | `events/event_ops.rs:693,707` | `.ok_or(Error::InvalidDistribution)?` replacing `.unwrap()` |
| M-2 | `events/storage.rs:309` | `.ok_or(Error::EventNotFound)?` replacing `.expect()` |

---

## Re-scan — September 2026 (events 1.7.0, profile 1.2.0)

**Date:** 2026-09-11
**Source:** `feat/escrow-open-pool` @ `0943d26`
**Tool:** `cargo-scout-audit` 0.3.16, detectors toolchain `nightly-2025-08-07`
**Command:** `cargo scout-audit --output-format md -- --target=wasm32v1-none --no-default-features`, run per crate

**Tooling caveat.** Scout 0.3.16 cannot compile a soroban-sdk 27 project: its
pinned nightly predates the stabilisation of `str::floor_char_boundary`, which
`soroban-sdk-macros 27.0.0` uses (`E0658` in `doc.rs:25`), and its default
`--target=wasm32-unknown-unknown` is rejected outright by the SDK 27 build
script. The scan was therefore run on a scratch copy of the same source with
the workspace pinned to `soroban-sdk = "23.5.2"` (the version the June scans
used) and the `compatibility` crate excluded. The contract source compiles
unchanged against 23.5.2, so detector coverage of `contracts/events/src` and
`contracts/profile/src` is unaffected; only the `soroban_version` enhancement
flags are an artifact of the pin.

| Crate | Critical | Medium | Minor | Enhancement |
|---|---|---|---|---|
| `boundless_events` | 2 | 19 | 0 | 30 |
| `boundless_profile` | 1 | 2 | 0 | 19 |

### Real findings

| ID | File | Finding | Assessment |
|---|---|---|---|
| S-1 | `events/src/event_ops.rs:745` | `let anchor_idx = existing_count + (idx as u32);` — bare `+` on `u32` in the Single-release branch of `select_winners`. | **Real, low.** Needs 2³² winner rows to overflow and `overflow-checks = true` would trap rather than wrap, but it breaks the checked-arithmetic rule the repo adopted for C-3. Change to `existing_count.saturating_add(idx as u32)` before audit fieldwork. |

### False positives (same classes as June)

| Detector | Locations | Reason |
|---|---|---|
| `unprotected_update_current_contract_wasm` (CRITICAL ×2) | `events/admin.rs:236`, `profile/admin.rs:216` | Inside `apply_upgrade()`, which opens with `require_admin()`; Scout does not trace access control through the enclosing function (June C-2). |
| `unsafe_map_get` (MEDIUM ×3) | `event_ops.rs:725,793`, `admin.rs:474` | `Map::get` returns `Option`; every site is `if let Some(floor) = …`. |
| `dos_unexpected_revert_with_storage` (MEDIUM ×4) | `admin.rs:383`, `storage.rs:421,700,793` | `floors.set` runs inside admin-gated `migrate_events`; the three `storage.rs` sites are `push_back` on an in-memory `Vec` inside paged read helpers (June M-9). |
| `dos_unbounded_operation` (MEDIUM ×7) | `admin.rs:425-440,464-488`, `event_ops.rs:675-681`, `grant.rs:63-81`, `storage.rs:419-423,698-702,791-795` | Read helpers are bounded by `limit.min(VIEW_PAGE_LIMIT)`. Migration loops walk one event's winner rows per call, admin-only, one-time. `select_winners` (Multi) and `claim_milestone` scan an event's winner rows, bounded by prior selections × milestones and gated by manager/owner auth. |
| `front_running` (MEDIUM ×2) | `escrow.rs:34,72` | Amounts are validated at every call site (`InvalidBudget`, `BelowMinimumContribution`, floor checks); Soroban has no public mempool ordering game equivalent to the detector's model (June M-5). |
| `dynamic_storage` (MEDIUM ×4) | `events/storage.rs:132,162`, `profile/storage.rs:113,135` | Bounded semver strings on admin-only paths (June M-6). |
| `avoid_vec_map_input` (MEDIUM ×1) | `events/lib.rs:225` (`select_winners`) | Every element validated: dedupe, `amount > 0`, floor, ≤ 50 (June M-8). |
| `storage_change_events` (ENHANCEMENT ×47) | `lib.rs` dispatchers, both crates | Dispatcher wrappers delegate to modules that emit events; `ManagerChanged` shipped with two-step delegation (#88), closing the one genuine gap noted in July. |
| `soroban_version` (ENHANCEMENT ×2) | `Cargo.toml` | Artifact of the 23.5.2 pin used to run the tool; the workspace is on 27.0.0. |

Net: one real low-severity finding (S-1), queued; zero real CRITICAL or MEDIUM.
