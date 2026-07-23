-- Boundless On-chain: Contribution inflows — daily volume
-- Panel type: line chart  x=day  y=total_contributed_display
--
-- FundsAdded fires for every add_funds call (crowdfunding contributions AND
-- partner top-ups on other pillars). To isolate crowdfunding, join event_id to
-- EventCreated where pillar = 'Crowdfunding'.
-- Decoding: see 10_event_created_decode_test.sql.

WITH ev AS (
    SELECT
        DATE_TRUNC('day', closed_at) AS day,
        JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') AS ev_name,
        map_from_entries(
            transform(
                CAST(JSON_EXTRACT(data_decoded, '$.map') AS ARRAY(JSON)),
                e -> ROW(JSON_EXTRACT_SCALAR(e, '$.key.symbol'), JSON_EXTRACT(e, '$.val'))
            )
        ) AS f
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND closed_at_date >= DATE '{{START_DATE}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') = 'FundsAdded'
)
SELECT
    day,
    COUNT(*)                                                                AS contribution_count,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(f['contributor'], '$.address'))       AS unique_contributors,
    SUM(CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE)) / 1e7    AS total_contributed_display
FROM ev
GROUP BY 1
ORDER BY 1 DESC
