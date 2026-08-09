-- Boundless On-chain: Crowdfunding contribution inflows — daily volume
-- Panel type: line chart  x=day  y=total_contributed_display
--
-- funds_added fires for every add_funds call (crowdfunding contributions AND
-- partner top-ups on other pillars). This query isolates crowdfunding by
-- joining each funds_added.event_id to event_created where pillar =
-- 'Crowdfunding'. For all-pillar contribution volume, drop the join.
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
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') IN ('funds_added', 'event_created')
),
crowdfunding_events AS (
    SELECT DISTINCT CAST(JSON_EXTRACT_SCALAR(f['id'], '$.u64') AS BIGINT) AS event_id
    FROM ev
    WHERE ev_name = 'event_created'
      AND JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol') = 'Crowdfunding'
),
contributions AS (
    SELECT
        day,
        CAST(JSON_EXTRACT_SCALAR(f['event_id'], '$.u64') AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(f['contributor'], '$.address')          AS contributor,
        CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE)   AS amount
    FROM ev
    WHERE ev_name = 'funds_added'
)
SELECT
    c.day,
    COUNT(*)                        AS contribution_count,
    COUNT(DISTINCT c.contributor)   AS unique_contributors,
    SUM(c.amount) / 1e7             AS total_contributed_display
FROM contributions c
JOIN crowdfunding_events cf ON c.event_id = cf.event_id
GROUP BY 1
ORDER BY 1 DESC
