-- Boundless On-chain: Crowdfunding inflows — daily contribution volume
-- Panel type: line chart  x=day  y=total_contributed_usdc

SELECT
    DATE_TRUNC('day', closed_at)                                                AS day,
    COUNT(*)                                                                    AS contribution_count,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(data_decoded, '$.contributor'))          AS unique_contributors,
    SUM(CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount') AS DOUBLE)) / 1e7   AS total_contributed_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'FundsAdded'
GROUP BY 1
ORDER BY 1 DESC
