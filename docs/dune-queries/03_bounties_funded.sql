-- Boundless On-chain: Events created — count and budget by pillar and month
-- Panel type: grouped bar chart  x=month  y=events_created  color=pillar
--
-- Decoding: see 10_event_created_decode_test.sql.

WITH ev AS (
    SELECT
        DATE_TRUNC('month', closed_at) AS month,
        map_from_entries(
            transform(
                CAST(JSON_EXTRACT(data_decoded, '$.map') AS ARRAY(JSON)),
                e -> ROW(JSON_EXTRACT_SCALAR(e, '$.key.symbol'), JSON_EXTRACT(e, '$.val'))
            )
        ) AS f
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND closed_at_date >= DATE '{{START_DATE}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') = 'EventCreated'
)
SELECT
    month,
    JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol')                     AS pillar,
    COUNT(*)                                                                AS events_created,
    SUM(CAST(JSON_EXTRACT_SCALAR(f['total_budget'], '$.i128') AS DOUBLE)) / 1e7
                                                                            AS total_budget_display
FROM ev
GROUP BY 1, 2
ORDER BY 1 DESC, 2 ASC
