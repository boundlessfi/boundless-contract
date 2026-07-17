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
| `topic_1` … `topic_N` | `VARCHAR` | XDR-decoded topic values, one column per position |
| `data` | `JSON` / `VARCHAR` | XDR-decoded event data map |
| `transaction_hash` | `VARCHAR` | Transaction hash for drill-down |

`topic_1` is always the event-name symbol emitted by the `#[contractevent]`
macro. All subsequent fields land in `data` as a JSON map keyed by field name.

> **Dune decoding note.** Dune's Stellar pipeline decodes ScVal automatically:
> `ScVal::Symbol` → string, `ScVal::U64` / `ScVal::I128` → numeric,
> `ScVal::Address` → StrKey string, `ScVal::Bytes` → hex string,
> `ScVal::Void` → SQL `NULL`, enum variants → string of the variant name.
> No manual XDR parsing is required in SQL.

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

**TVL note.** For all pillars except Crowdfunding, `total_budget` is deposited
into escrow at create time (minus the protocol fee). For Crowdfunding, escrow
starts at 0 and grows via `FundsAdded`. Track live TVL by summing `FundsAdded`
debiting `WinnerPaid` + `MilestoneClaimed` + `ContributorRefunded` +
`OwnerResidualRefunded` events.

#### `EventCancelled`
Emitted at the end of the cancel flow (`start_cancel` fast-path or
`finalize_cancel`).

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `id` | `U64` | Matches `EventCreated.id` |

#### `FundsAdded`
Emitted on every successful `add_funds` call (partner top-ups, crowdfunding
contributions). **Amount is net of fee.**

| Field | ScVal type | Notes |
|-------|-----------|-------|
| `event_id` | `U64` | |
| `contributor` | `Address` | |
| `amount` | `I128` | Net credited amount after fee deduction |
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

**Protocol fee** is deducted at deposit time and never enters `remaining_escrow`.
All `*amount*` fields in events are already net-of-fee values — they match what the contract actually holds. Use them directly; no fee adjustment needed in SQL.

---

## 4. Dune SQL queries

All queries use `{{CONTRACT_ADDRESS}}` as a Dune parameter.
Amounts are in stroops (7 decimal places); divide by `1e7` for human units.

### 4.1 Total Value Locked (current)

```sql
-- Boundless: Current TVL
-- Sums all escrow inflows minus outflows to give the live balance.

WITH inflows AS (
    SELECT
        COALESCE(CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE), 0) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
      AND JSON_EXTRACT_SCALAR(data, '$.pillar') != 'Crowdfunding'

    UNION ALL

    SELECT
        COALESCE(CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE), 0) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'FundsAdded'
),
outflows AS (
    SELECT
        COALESCE(CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE), 0) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed', 'ContributorRefunded', 'OwnerResidualRefunded')
)
SELECT
    (SUM(i.amount) - SUM(o.amount)) / 1e7 AS tvl_usdc
FROM inflows i, outflows o
```

### 4.2 TVL over time (daily)

```sql
-- Boundless: Daily TVL (running balance)

WITH events AS (
    SELECT
        DATE_TRUNC('day', closed_at) AS day,
        CASE
            WHEN topic_1 = 'EventCreated'
              AND JSON_EXTRACT_SCALAR(data, '$.pillar') != 'Crowdfunding'
            THEN  CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE)
            WHEN topic_1 = 'FundsAdded'
            THEN  CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE)
            WHEN topic_1 IN ('WinnerPaid', 'MilestoneClaimed',
                             'ContributorRefunded', 'OwnerResidualRefunded')
            THEN -CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE)
            ELSE 0
        END AS delta
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN (
          'EventCreated', 'FundsAdded',
          'WinnerPaid', 'MilestoneClaimed',
          'ContributorRefunded', 'OwnerResidualRefunded'
      )
),
daily_delta AS (
    SELECT day, SUM(delta) / 1e7 AS daily_change
    FROM events
    GROUP BY 1
)
SELECT
    day,
    daily_change,
    SUM(daily_change) OVER (ORDER BY day ROWS UNBOUNDED PRECEDING) AS tvl_usdc
FROM daily_delta
ORDER BY 1
```

### 4.3 Bounties funded (count + volume by pillar)

```sql
-- Boundless: Events created — count and total budget by pillar and month

SELECT
    DATE_TRUNC('month', closed_at)                              AS month,
    JSON_EXTRACT_SCALAR(data, '$.pillar')                       AS pillar,
    COUNT(*)                                                    AS events_created,
    SUM(CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE)) / 1e7
                                                                AS total_budget_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 = 'EventCreated'
GROUP BY 1, 2
ORDER BY 1 DESC, 2
```

### 4.4 Total payouts to builders

```sql
-- Boundless: Total paid out to builders (WinnerPaid + MilestoneClaimed)

SELECT
    DATE_TRUNC('month', closed_at)  AS month,
    topic_1                         AS payout_type,
    COUNT(*)                        AS payout_count,
    SUM(CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE)) / 1e7
                                    AS total_paid_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
GROUP BY 1, 2
ORDER BY 1 DESC, 2
```

### 4.5 Unique participants (applicants + recipients)

```sql
-- Boundless: Unique builder wallets that applied or received a payout

SELECT
    COUNT(DISTINCT applicant_or_recipient) AS unique_builders
FROM (
    SELECT JSON_EXTRACT_SCALAR(data, '$.applicant')  AS applicant_or_recipient
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'Applied'

    UNION

    SELECT JSON_EXTRACT_SCALAR(data, '$.recipient')  AS applicant_or_recipient
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
) AS combined
```

### 4.6 Unique organizers

```sql
-- Boundless: Unique organizer wallets that created events

SELECT COUNT(DISTINCT JSON_EXTRACT_SCALAR(data, '$.owner')) AS unique_organizers
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 = 'EventCreated'
```

### 4.7 Funded vs completed vs cancelled events

```sql
-- Boundless: Event outcome funnel

WITH created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.id') AS BIGINT) AS event_id,
        closed_at AS created_at
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
),
cancelled AS (
    SELECT DISTINCT CAST(JSON_EXTRACT_SCALAR(data, '$.id') AS BIGINT) AS event_id
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCancelled'
),
paid AS (
    -- An event is "completed" if it has at least one payout event
    SELECT DISTINCT CAST(JSON_EXTRACT_SCALAR(data, '$.event_id') AS BIGINT) AS event_id
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
)
SELECT
    COUNT(c.event_id)                                        AS total_created,
    COUNT(p.event_id)                                        AS completed_with_payout,
    COUNT(cx.event_id)                                       AS cancelled,
    COUNT(c.event_id) - COUNT(p.event_id) - COUNT(cx.event_id) AS active_or_pending
FROM created c
LEFT JOIN paid   p  ON c.event_id = p.event_id
LEFT JOIN cancelled cx ON c.event_id = cx.event_id
```

### 4.8 Average bounty size and time-to-payout

```sql
-- Boundless: Average budget and time from creation to first payout (days)

WITH created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.id') AS BIGINT)              AS event_id,
        CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE) / 1e7 AS budget_usdc,
        closed_at                                                      AS created_at
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
),
first_payout AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.event_id') AS BIGINT) AS event_id,
        MIN(closed_at)                                          AS first_paid_at
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
    GROUP BY 1
)
SELECT
    AVG(c.budget_usdc)                                          AS avg_budget_usdc,
    AVG(DATE_DIFF('day', c.created_at, fp.first_paid_at))       AS avg_days_to_payout
FROM created c
JOIN first_payout fp ON c.event_id = fp.event_id
```

### 4.9 Payout volume by token address

```sql
-- Boundless: Payout volume grouped by payment token
-- (Join to an off-chain token symbol map for readable names)

WITH payouts AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.event_id') AS BIGINT) AS event_id,
        CAST(JSON_EXTRACT_SCALAR(data, '$.amount')   AS DOUBLE) / 1e7 AS amount_usdc,
        topic_1
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
),
created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.id')    AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(data, '$.token')                  AS token_address
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
)
SELECT
    c.token_address,
    SUM(p.amount_usdc) AS total_paid_usdc,
    COUNT(*)           AS payout_count
FROM payouts p
JOIN created c ON p.event_id = c.event_id
GROUP BY 1
ORDER BY 2 DESC
```

### 4.10 Crowdfunding contributions over time

```sql
-- Boundless: Crowdfunding inflows — daily contribution volume

SELECT
    DATE_TRUNC('day', closed_at)                                           AS day,
    COUNT(*)                                                               AS contribution_count,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(data, '$.contributor'))             AS unique_contributors,
    SUM(CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE)) / 1e7      AS total_contributed_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 = 'FundsAdded'
GROUP BY 1
ORDER BY 1 DESC
```

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
- **Fee changes.** Protocol fee is deducted before `remaining_escrow` is
  credited. All `amount` fields in events are already net-of-fee. No SQL
  adjustment needed if the fee rate changes.
- **Crowdfunding vs other pillars.** `EventCreated.total_budget` is a
  *funding goal* for Crowdfunding, not an escrow deposit. Exclude
  `pillar = 'Crowdfunding'` from inflow sums based on `EventCreated` and
  use only `FundsAdded` for their inflows (queries §4.1 and §4.2 already
  do this).
