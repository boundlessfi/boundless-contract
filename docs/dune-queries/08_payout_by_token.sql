-- Boundless On-chain: Payout volume by payment token
-- Panel type: table or donut chart
--
-- Join on stellar.assets (or a manual symbol map) to resolve token address
-- to a human-readable ticker. For now we report the raw contract address.

WITH payouts AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.event_id') AS BIGINT) AS event_id,
        CAST(JSON_EXTRACT_SCALAR(data, '$.amount')   AS DOUBLE) / 1e7 AS amount_usdc,
        topic_1                                                        AS payout_type
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
),
event_tokens AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data, '$.id')    AS BIGINT) AS event_id,
        JSON_EXTRACT_SCALAR(data, '$.token')                 AS token_address
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
)
SELECT
    t.token_address,
    p.payout_type,
    COUNT(*)               AS payout_count,
    SUM(p.amount_usdc)     AS total_paid_usdc
FROM payouts p
JOIN event_tokens t ON p.event_id = t.event_id
GROUP BY 1, 2
ORDER BY total_paid_usdc DESC
