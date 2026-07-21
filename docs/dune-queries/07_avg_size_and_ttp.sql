-- Boundless On-chain: Average budget and time-to-first-payout
-- Panel type: table / single numbers

WITH created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.id') AS BIGINT)              AS event_id,
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.total_budget') AS DOUBLE) / 1e7
                                                                               AS budget_usdc,
        closed_at                                                              AS created_at
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
),
first_payout AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.event_id') AS BIGINT) AS event_id,
        MIN(closed_at)                                                  AS first_paid_at
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN ('WinnerPaid', 'MilestoneClaimed')
    GROUP BY 1
)
SELECT
    AVG(c.budget_usdc)                                              AS avg_budget_usdc,
    AVG(DATE_DIFF('day', c.created_at, fp.first_paid_at))          AS avg_days_to_payout
FROM created c
JOIN first_payout fp ON c.event_id = fp.event_id
