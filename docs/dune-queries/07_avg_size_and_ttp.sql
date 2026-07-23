-- Boundless On-chain: Average budget and time-to-first-payout
-- Panel type: table / single numbers
--
-- Time-to-payout is measured to the first WinnerPaid/MilestoneClaimed. Under
-- the pull model, WinnerPaid fires when a winner claims (claim_prize), not at
-- select_winners, so this includes claim latency.
-- Decoding: see 10_event_created_decode_test.sql.

WITH ev AS (
    SELECT
        closed_at,
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
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol')
          IN ('EventCreated', 'WinnerPaid', 'MilestoneClaimed')
),
created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(f['id'], '$.u64') AS BIGINT)                   AS event_id,
        CAST(JSON_EXTRACT_SCALAR(f['total_budget'], '$.i128') AS DOUBLE) / 1e7  AS budget_display,
        closed_at                                                              AS created_at
    FROM ev WHERE ev_name = 'EventCreated'
),
first_payout AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(f['event_id'], '$.u64') AS BIGINT) AS event_id,
        MIN(closed_at)                                             AS first_paid_at
    FROM ev WHERE ev_name IN ('WinnerPaid', 'MilestoneClaimed')
    GROUP BY 1
)
SELECT
    AVG(c.budget_display)                                 AS avg_budget_display,
    AVG(DATE_DIFF('day', c.created_at, fp.first_paid_at)) AS avg_days_to_payout
FROM created c
JOIN first_payout fp ON c.event_id = fp.event_id
