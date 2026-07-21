-- Boundless On-chain: Events created — count and budget by pillar and month
-- Panel type: bar chart (grouped)  x=month  y=events_created  color=pillar

SELECT
    DATE_TRUNC('month', closed_at)                                                   AS month,
    JSON_EXTRACT_SCALAR(data_decoded, '$.pillar')                                    AS pillar,
    COUNT(*)                                                                         AS events_created,
    SUM(CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.total_budget') AS DOUBLE)) / 1e7  AS total_budget_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
GROUP BY 1, 2
ORDER BY 1 DESC, 2 ASC
