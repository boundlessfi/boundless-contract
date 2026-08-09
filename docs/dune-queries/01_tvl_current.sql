-- Boundless On-chain: Current TVL
-- Panel type: counter
-- Total escrow currently held by the contract = inflows - outflows.
--
-- Decoding (see 10_event_created_decode_test.sql for the why):
--   event name  -> topics_decoded '$[0].symbol' (snake_case: #[contractevent]
--                  lowercases the struct name, e.g. EventCreated -> event_created)
--   fields      -> rebuild data_decoded '$.map' into MAP(name -> ScVal JSON),
--                  then read each field by its ScVal type ($.i128, $.u64, ...).
--
-- Inflows:  event_created.total_budget (non-Crowdfunding — escrowed at creation)
--           funds_added.amount         (partner top-ups + crowdfunding contributions)
-- Outflows: winner_paid.amount, milestone_claimed.amount,
--           contributor_refunded.amount, owner_residual_refunded.amount
-- Amounts are the values the contract actually escrows/releases (the protocol
-- fee is charged separately, not embedded here). Divide by 1e7 for display.

WITH ev AS (
    SELECT
        JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') AS ev_name,
        map_from_entries(
            transform(
                CAST(JSON_EXTRACT(data_decoded, '$.map') AS ARRAY(JSON)),
                e -> ROW(JSON_EXTRACT_SCALAR(e, '$.key.symbol'), JSON_EXTRACT(e, '$.val'))
            )
        ) AS f
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND closed_at_date >= DATE '{{START_DATE}}'
      AND data_decoded LIKE '%"map"%'
),
inflows AS (
    SELECT CAST(JSON_EXTRACT_SCALAR(f['total_budget'], '$.i128') AS DOUBLE) AS amount
    FROM ev
    WHERE ev_name = 'event_created'
      AND JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol') <> 'Crowdfunding'
    UNION ALL
    SELECT CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE)
    FROM ev
    WHERE ev_name = 'funds_added'
),
outflows AS (
    SELECT CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE) AS amount
    FROM ev
    WHERE ev_name IN ('winner_paid', 'milestone_claimed', 'contributor_refunded', 'owner_residual_refunded')
)
SELECT
    (COALESCE((SELECT SUM(amount) FROM inflows), 0)
        - COALESCE((SELECT SUM(amount) FROM outflows), 0)) / 1e7 AS tvl_display
