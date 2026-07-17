-- Boundless On-chain: Average event budget and time-to-first-payout
-- Panel type: counter / table

WITH created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.id')              AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(data, '$.pillar')                           AS pillar,
        CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget')    AS DOUBLE) / 1e7
                                                                        AS budget_usdc,
        closed_at                                                       AS created_at
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
    c.pillar,
    COUNT(c.event_id)                                           AS events_with_payout,
    AVG(c.budget_usdc)                                          AS avg_budget_usdc,
    MIN(c.budget_usdc)                                          AS min_budget_usdc,
    MAX(c.budget_usdc)                                          AS max_budget_usdc,
    AVG(DATE_DIFF('day', c.created_at, fp.first_paid_at))       AS avg_days_to_first_payout
FROM created c
JOIN first_payout fp ON c.event_id = fp.event_id
GROUP BY 1
ORDER BY avg_budget_usdc DESC
