-- Boundless On-chain: Event outcome funnel
-- Panel type: bar chart or table
--
-- Buckets every created event into completed_with_payout / cancelled /
-- active_or_pending. Note the id field name differs by event:
--   event_created and event_cancelled carry 'id';
--   winner_paid / milestone_claimed carry 'event_id'.
-- Decoding: see 10_event_created_decode_test.sql.

WITH ev AS (
    SELECT
        closed_at,
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
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol')
          IN ('event_created', 'event_cancelled', 'winner_paid', 'milestone_claimed')
),
created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(f['id'], '$.u64') AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol')   AS pillar
    FROM ev WHERE ev_name = 'event_created'
),
cancelled AS (
    SELECT DISTINCT CAST(JSON_EXTRACT_SCALAR(f['id'], '$.u64') AS BIGINT) AS event_id
    FROM ev WHERE ev_name = 'event_cancelled'
),
paid AS (
    SELECT DISTINCT CAST(JSON_EXTRACT_SCALAR(f['event_id'], '$.u64') AS BIGINT) AS event_id
    FROM ev WHERE ev_name IN ('winner_paid', 'milestone_claimed')
)
SELECT
    c.pillar,
    COUNT(c.event_id)                                        AS total_created,
    COUNT(p.event_id)                                        AS completed_with_payout,
    COUNT(cx.event_id)                                       AS cancelled,
    COUNT(c.event_id) - COUNT(p.event_id) - COUNT(cx.event_id) AS active_or_pending
FROM created c
LEFT JOIN paid      p  ON c.event_id = p.event_id
LEFT JOIN cancelled cx ON c.event_id = cx.event_id
GROUP BY 1
ORDER BY total_created DESC
