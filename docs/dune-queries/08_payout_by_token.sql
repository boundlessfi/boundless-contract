-- Boundless On-chain: Payout volume grouped by payment token
-- Panel type: table
--
-- Token address comes from EventCreated; join via event_id to payout events.
-- Add an off-chain symbol map or CASE expression for readable token names.

WITH payouts AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.event_id') AS BIGINT)        AS event_id,
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount')   AS DOUBLE) / 1e7  AS amount_usdc,
        JSON_EXTRACT_SCALAR(topics_decoded, '$[0]')                            AS payout_type
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN ('WinnerPaid', 'MilestoneClaimed')
),
created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.id')    AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(data_decoded, '$.token')                  AS token_address
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
)
SELECT
    c.token_address,
    SUM(p.amount_usdc)  AS total_paid_usdc,
    COUNT(*)            AS payout_count
FROM payouts p
JOIN created c ON p.event_id = c.event_id
GROUP BY 1
ORDER BY 2 DESC
