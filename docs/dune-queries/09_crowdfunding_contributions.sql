-- Boundless On-chain: Crowdfunding daily contribution volume
-- Panel type: line chart  x=day  y=total_contributed_usdc
--
-- FundsAdded covers both crowdfunding community contributions and partner
-- top-ups on Hackathon/Bounty/Grant events. Filter to Crowdfunding events
-- by joining on EventCreated.pillar.

WITH cf_events AS (
    SELECT CAST(JSON_EXTRACT_SCALAR(data, '$.id') AS BIGINT) AS event_id
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
      AND JSON_EXTRACT_SCALAR(data, '$.pillar') = 'Crowdfunding'
)
SELECT
    DATE_TRUNC('day', hce.closed_at)                                         AS day,
    COUNT(*)                                                                  AS contribution_count,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(hce.data, '$.contributor'))            AS unique_contributors,
    SUM(CAST(JSON_EXTRACT_SCALAR(hce.data, '$.amount') AS DOUBLE)) / 1e7     AS total_contributed_usdc
FROM stellar.history_contract_events hce
JOIN cf_events cf ON CAST(JSON_EXTRACT_SCALAR(hce.data, '$.event_id') AS BIGINT) = cf.event_id
WHERE hce.contract_id = '{{CONTRACT_ADDRESS}}'
  AND hce.topic_1 = 'FundsAdded'
GROUP BY 1
ORDER BY 1 DESC
