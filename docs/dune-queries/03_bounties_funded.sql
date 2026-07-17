-- Boundless On-chain: Events created — count and budget by pillar and month
-- Panel type: bar chart (grouped)  x=month  y=events_created  color=pillar
--             or table for detailed view

SELECT
    DATE_TRUNC('month', closed_at)                                          AS month,
    JSON_EXTRACT_SCALAR(data, '$.pillar')                                   AS pillar,
    COUNT(*)                                                                AS events_created,
    SUM(CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE)) / 1e7 AS total_budget_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 = 'EventCreated'
GROUP BY 1, 2
ORDER BY 1 DESC, 2 ASC
