-- Boundless On-chain: Event outcome funnel
-- Panel type: bar chart or table
--
-- Buckets every created event into:
--   completed_with_payout  — had at least one WinnerPaid or MilestoneClaimed
--   cancelled              — received an EventCancelled
--   active_or_pending      — neither (still running or awaiting selection)

WITH created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.id') AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(data_decoded, '$.pillar')              AS pillar,
        closed_at                                                  AS created_at
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
),
cancelled AS (
    SELECT DISTINCT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.id') AS BIGINT) AS event_id
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCancelled'
),
paid AS (
    SELECT DISTINCT
        CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.event_id') AS BIGINT) AS event_id
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN ('WinnerPaid', 'MilestoneClaimed')
)
SELECT
    c.pillar,
    COUNT(c.event_id)                                                    AS total_created,
    COUNT(p.event_id)                                                    AS completed_with_payout,
    COUNT(cx.event_id)                                                   AS cancelled,
    COUNT(c.event_id) - COUNT(p.event_id) - COUNT(cx.event_id)          AS active_or_pending
FROM created c
LEFT JOIN paid       p  ON c.event_id = p.event_id
LEFT JOIN cancelled  cx ON c.event_id = cx.event_id
GROUP BY 1
ORDER BY total_created DESC
