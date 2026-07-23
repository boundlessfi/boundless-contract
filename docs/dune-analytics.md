# Boundless On-chain Analytics — Dune Integration Guide

This document covers everything needed to build and maintain the Boundless Dune
dashboard: the complete emitted-event schema (topic names, ScVal layout, field
semantics), how Dune indexes Stellar/Soroban events, and ready-to-run query
skeletons for each dashboard panel.

Reference implementation: [Boundless On-chain Dashboard](https://dune.com) _(link updated once published)_.

---

## 1. How Dune indexes Soroban events

Dune has indexed Stellar Soroban contract events since Meridian 2024.
The relevant tables are:

| Table | Contents |
|-------|----------|
| `stellar.history_contract_events` | Soroban contract events + call traces. Filter by `contract_id`. |
| `stellar.contract_data` | Contract instance and persistent storage (escrow balances, TTL). |
| `stellar.history_transactions` | Transaction-level detail, signers, fees. |
| `stellar.history_ledgers` | Ledger metadata, close-time for time-series bucketing. |

All Boundless analytics are built on `stellar.history_contract_events`.

### Filtering to the Boundless contract

```sql
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
```

Replace `{{CONTRACT_ADDRESS}}` with the deployed contract address (see
`docs/DEPLOYMENT.md`). Use a Dune parameter so the same queries work across
testnet and mainnet.

### Soroban event structure in Dune

Each row in `history_contract_events` has:

| Column | Type | Description |
|--------|------|-------------|
| `ledger_sequence` | `BIGINT` | Ledger number |
| `closed_at` | `TIMESTAMP` | Ledger close time |
| `contract_id` | `VARCHAR` | Contract address (StrKey) |
| `topics_decoded` | `VARCHAR` | JSON **array of ScVal objects**. The event name is at `$[0].symbol`, e.g. `[{"symbol":"EventCreated"}]` |
| `data_decoded` | `VARCHAR` | JSON **ScVal map**: `{"map":[{"key":{"symbol":"id"},"val":{"u64":"7"}}, ...]}`. Each value is wrapped by its ScVal type |
| `closed_at_date` | `DATE` | **Partition column** — always filter it to avoid full-table scans |
| `topics` / `data` | `VARCHAR` | Raw XDR (use the `*_decoded` columns instead) |
| `type_string` | `VARCHAR` | Event type string (`"contract"` for Soroban events) |
| `transaction_hash` | `VARBINARY` | Transaction hash — wrap with `to_hex()` for a readable string |
| `successful` | `BOOLEAN` | Whether the transaction succeeded |

**Decoding, correctly (verified against live `stellar.history_contract_events`):**

1. **Event name** is *not* `$[0]` — that element is an object. Use:
   `JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol')`.
2. **Fields** are *not* flat `$.field` — they live inside a positional `map`,
   each value wrapped by its ScVal type. Rebuild the map into
   `MAP(field_name → ScVal JSON)`, then read each field by type:

```sql
-- reusable decode: f['<field>'] -> that field's ScVal JSON
map_from_entries(
    transform(
        CAST(JSON_EXTRACT(data_decoded, '$.map') AS ARRAY(JSON)),
        e -> ROW(JSON_EXTRACT_SCALAR(e, '$.key.symbol'), JSON_EXTRACT(e, '$.val'))
    )
) AS f
-- then:  JSON_EXTRACT_SCALAR(f['id'], '$.u64')        -- u64
--        JSON_EXTRACT_SCALAR(f['amount'], '$.i128')   -- i128
--        JSON_EXTRACT_SCALAR(f['owner'], '$.address')  -- Address (StrKey)
--        JSON_EXTRACT_SCALAR(f['title'], '$.string')   -- String
--        JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol')  -- unit enum
```

The canonical, Dune-tested queries live in [`docs/dune-queries/`](./dune-queries);
start with `10_event_created_decode_test.sql`. The snippets in §4 below illustrate
intent — treat the `.sql` files as the source of truth.

---

## 2. Complete event schema

Every event emitted by the contract is listed below with its topic name,
data fields, ScVal type, and notes for analytics use.

### 2.1 Core lifecycle events

#### `EventCreated`
Emitted by `create_event` for every pillar.

| Field | ScVal type | Analytics notes |
|-------|-----------|-----------------|
| `id` | `U64` | Stable bounty/event identifier |
| `pillar` | enum string (`"Hackathon"`, `"Bounty"`, `"Grant"`, `"Crowdfunding"`) | Segment by type |
| `owner` | `Address` | Organizer wallet |
| `token` | `Address` | Payment token (USDC, XLM, …) |
| `total_budget` | `I128` (stroops) | Gross budget; divide by `10^7` for display |
| `content_uri` | `String` | IPFS/S3 URI; omit from aggregates |
| `title` | `String` | Human label; omit from aggregates |

**TVL note.** For all pillars except Crowdfunding, the **full** `total_budget`
is escrowed at create time — the protocol fee is charged *on top* (the owner
pays `budget + fee`; the fee is forwarded to the fee account, and escrow is
credited the full `total_budget`). So `total_budget` is exactly the escrow
inflow; use it directly. For Crowdfunding, escrow starts at 0 and grows via
`FundsAdded`. Track live TVL by summing `EventCreated.total_budget`
(non-Crowdfunding) + `FundsAdded.amount`, debiting `WinnerPaid` +
`MilestoneClaimed` + `ContributorRefunded` + `OwnerResidualRefunded`.

> **Fee revenue is not derivable from these events.** The fee is transferred to
> the fee account with no dedicated event and is not embedded in any event
> `amount`. For protocol-fee analytics, read `stellar.history_transactions` /
> effects for transfers to the fee account, not `history_contract_events`.

#### `EventCancelled`
Emitted at the end of the cancel flow (`start_cancel` fast-path or
`finalize_cancel`).

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `id` | `U64` | Matches `EventCreated.id` |

#### `FundsAdded`
Emitted on every successful `add_funds` call (partner top-ups, crowdfunding
contributions). `amount` is exactly what escrow was credited (for
non-Crowdfunding the fee is charged on top; Crowdfunding contributions are
fee-free, with the fee taken later at `claim_milestone`).

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `contributor` | `Address` | |
| `amount` | `I128` | Amount credited to escrow |
| `new_remaining` | `I128` | `remaining_escrow` after this deposit |

#### `ContributorRefunded`
Emitted once per contributor per batch during paged cancel.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `contributor` | `Address` | |
| `amount` | `I128` | Refunded amount |

#### `OwnerResidualRefunded`
Emitted when the event owner receives their escrow residual at cancel time.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `owner` | `Address` | |
| `amount` | `I128` | |

---

### 2.2 Participation events

#### `Applied`
Emitted when a builder applies to a Bounty or Grant.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `applicant` | `Address` | |

#### `ApplicationWithdrawn`
Emitted when a builder withdraws their application before submitting.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `applicant` | `Address` | |

#### `Submitted`
Emitted on every `submit` call (first submit and re-submits). Hackathon, Bounty,
Grant only — Crowdfunding has no submission step.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `applicant` | `Address` | |
| `content_uri` | `String` | |

#### `SubmissionWithdrawn`
Emitted when a builder removes their submission before deadline.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `applicant` | `Address` | |

---

### 2.3 Payout events

#### `WinnersSelected`
Emitted once at the end of `select_winners`, regardless of pillar.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `count` | `U32` | Number of winners chosen |

#### `WinnerPaid`
**Single-release pillars only** (Hackathon, Bounty). One event per winner.
Grant and Crowdfunding winners receive funds via `MilestoneClaimed`.

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `recipient` | `Address` | Winner wallet |
| `position` | `U32` | 1st, 2nd, … |
| `amount` | `I128` | Amount transferred (stroops) |
| `milestone` | `U32` or `NULL` | Always `NULL` when emitted by `select_winners` |

#### `MilestoneClaimed`
Grant and Crowdfunding payouts. One event per (recipient, milestone).

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `recipient` | `Address` | |
| `milestone` | `U32` | 0-based milestone index |
| `amount` | `I128` | Payout amount |

---

### 2.4 Admin / config events

These are useful for governance dashboards but not for TVL/payout metrics.

| Event name | Key fields | Notes |
|-----------|-----------|-------|
| `AdminUpdated` | `new_admin: Address` | Admin rotation completed |
| `PendingAdminSet` | `target: Address` | Two-step rotation initiated |
| `FeeAccountUpdated` | `new_account: Address` | Fee recipient changed |
| `FeeBpsUpdated` | `new_bps: U32` | Protocol fee rate changed (max 1 000 bps = 10 %) |
| `ProfileContractUpdated` | `new_addr: Address` | Companion profile contract changed |
| `TokenRegistered` | `token: Address` | New payment token whitelisted |
| `TokenDeregistered` | `token: Address` | Token removed from whitelist |
| `ManagerProposed` | `event_id: U64`, `target: Address`, `expires_at_ledger: U32` | Two-step manager delegation proposed for an event |
| `ManagerChanged` | `event_id: U64`, `new_manager: Address` | Manager delegation accepted (authority transferred) |
| `PendingManagerCancelled` | `event_id: U64` | Pending manager proposal vetoed |
| `Paused` | _(no fields)_ | Contract paused |
| `Unpaused` | _(no fields)_ | Contract unpaused |

---

### 2.5 Upgrade / migration events

| Event name | Key fields | Notes |
|-----------|-----------|-------|
| `PendingUpgradeProposed` | `wasm_hash, new_version, available_at_ledger, expires_at_ledger` | Upgrade queued (timelocked ~1 day) |
| `PendingUpgradeCancelled` | `cancelled_at_ledger: U32` | Upgrade cancelled before apply |
| `UpgradeApplied` | `wasm_hash, new_version: String` | Preferred event; new wasm is live |
| `Upgraded` | `new_wasm_hash` | Legacy alias for `UpgradeApplied`; indexers should prefer `UpgradeApplied` |
| `Migrated` | `from_version, to_version: String` | Post-upgrade data migration ran |

---

## 3. TVL accounting model

TVL (Total Value Locked) = escrow currently held by the contract.

**Inflows (+)**
- `EventCreated.total_budget` — for Hackathon, Bounty, Grant (Crowdfunding starts at 0)
- `FundsAdded.amount` — every partner / community top-up

**Outflows (−)**
- `WinnerPaid.amount` — immediate payout at `select_winners` (Single pillars)
- `MilestoneClaimed.amount` — milestone payout (Grant + Crowdfunding)
- `ContributorRefunded.amount` — partner refund during cancel
- `OwnerResidualRefunded.amount` — owner residual refund during cancel

**Protocol fee** is charged on top of the deposited amount (the payer sends
`amount + fee`; the fee is forwarded to the fee account). Every event `amount`
therefore equals exactly what escrow was credited or released — use them
directly, no fee adjustment in SQL. The fee itself is *not* in any event; see
the fee-revenue note in §2.1.

---

## 4. Dune SQL queries

All queries use `{{CONTRACT_ADDRESS}}` as a Dune parameter.
Amounts are in stroops (7 decimal places); divide by `1e7` for human units.

### 4.1 Total Value Locked (current)

> Canonical, Dune-tested query: [`docs/dune-queries/01_tvl_current.sql`](./dune-queries/01_tvl_current.sql)

### 4.2 TVL over time (daily)

> Canonical, Dune-tested query: [`docs/dune-queries/02_tvl_over_time.sql`](./dune-queries/02_tvl_over_time.sql)

### 4.3 Bounties funded (count + volume by pillar)

> Canonical, Dune-tested query: [`docs/dune-queries/03_bounties_funded.sql`](./dune-queries/03_bounties_funded.sql)

### 4.4 Total payouts to builders

> Canonical, Dune-tested query: [`docs/dune-queries/04_total_payouts.sql`](./dune-queries/04_total_payouts.sql)

### 4.5 Unique participants (applicants + recipients)

> Canonical, Dune-tested query: [`docs/dune-queries/05_unique_participants.sql`](./dune-queries/05_unique_participants.sql)

### 4.6 Unique organizers

> Canonical, Dune-tested query: [`docs/dune-queries/05_unique_participants.sql`](./dune-queries/05_unique_participants.sql)

### 4.7 Funded vs completed vs cancelled events

> Canonical, Dune-tested query: [`docs/dune-queries/06_event_outcomes.sql`](./dune-queries/06_event_outcomes.sql)

### 4.8 Average bounty size and time-to-payout

> Canonical, Dune-tested query: [`docs/dune-queries/07_avg_size_and_ttp.sql`](./dune-queries/07_avg_size_and_ttp.sql)

### 4.9 Payout volume by token address

> Canonical, Dune-tested query: [`docs/dune-queries/08_payout_by_token.sql`](./dune-queries/08_payout_by_token.sql)

### 4.10 Crowdfunding contributions over time

> Canonical, Dune-tested query: [`docs/dune-queries/09_crowdfunding_contributions.sql`](./dune-queries/09_crowdfunding_contributions.sql)

---

## 5. Dashboard layout

Suggested panel arrangement for the **Boundless On-chain** public Dune
dashboard:

| Row | Panel | Query |
|-----|-------|-------|
| 1 | **TVL (current)** — single number | §4.1 |
| 1 | **Total paid out** — single number | §4.4 (sum only) |
| 1 | **Events created** — single number | §4.3 (count only) |
| 1 | **Unique builders** — single number | §4.5 |
| 2 | **TVL over time** — area chart | §4.2 |
| 2 | **Monthly payouts** — bar chart | §4.4 |
| 3 | **Events by pillar** — donut chart | §4.3 (count, no time) |
| 3 | **Funded vs completed vs cancelled** — bar | §4.7 |
| 4 | **Average budget & time-to-payout** — table | §4.8 |
| 4 | **Payout volume by token** — table | §4.9 |
| 5 | **Crowdfunding daily contributions** — line chart | §4.10 |

---

## 6. Acceptance checklist

- [ ] `EventCreated` decoded: `id`, `pillar`, `owner`, `token`, `total_budget` all readable in Dune
- [ ] `FundsAdded` decoded: `amount`, `new_remaining` numeric
- [ ] `WinnerPaid` decoded: `recipient`, `amount` numeric
- [ ] `MilestoneClaimed` decoded: `recipient`, `milestone`, `amount` numeric
- [ ] TVL query (§4.1) returns a non-negative number on mainnet
- [ ] "Bounties funded" chart (§4.3) renders at least one bar
- [ ] "Total payouts" chart (§4.4) matches known payout transactions
- [ ] Dashboard is set to **public** in Dune

---

## 7. Maintenance notes

- **Contract upgrades.** After `apply_upgrade` + `migrate`, verify the new
  version's events still decode correctly. The `Migrated` event in Dune
  confirms the on-chain migration ran; cross-check with `UpgradeApplied`.
- **New tokens.** When `TokenRegistered` appears for a new address, add it
  to any off-chain symbol map used by query §4.9.
- **Fee changes.** The protocol fee is charged on top of the deposited amount,
  so event `amount` fields already equal the escrow credit/release. No SQL
  adjustment is needed if the fee rate changes. (Fee revenue itself is not in
  these events — see §2.1.)
- **Crowdfunding vs other pillars.** `EventCreated.total_budget` is a
  *funding goal* for Crowdfunding, not an escrow deposit. Exclude
  `pillar = 'Crowdfunding'` from inflow sums based on `EventCreated` and
  use only `FundsAdded` for their inflows (queries §4.1 and §4.2 already
  do this).
