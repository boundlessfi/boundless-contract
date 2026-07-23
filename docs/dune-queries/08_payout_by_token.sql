-- Boundless On-chain: Payout volume grouped by payment token
-- Panel type: table
--
-- Token address is on EventCreated (field 'token'); join to payout events by
-- event_id. Add a CASE map for readable token symbols.
-- Decoding: see 10_event_created_decode_test.sql.

WITH ev AS (
    SELECT
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
          IN ('EventCreated', 'WinnerPaid', 'MilestoneClaimed')
),
payouts AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(f['event_id'], '$.u64') AS BIGINT)          AS event_id,
        CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE) / 1e7     AS amount_display
    FROM ev WHERE ev_name IN ('WinnerPaid', 'MilestoneClaimed')
),
created AS (
    SELECT
        CAST(JSON_EXTRACT_SCALAR(f['id'], '$.u64') AS BIGINT)  AS event_id,
        JSON_EXTRACT_SCALAR(f['token'], '$.address')          AS token_address
    FROM ev WHERE ev_name = 'EventCreated'
)
SELECT
    c.token_address,
    SUM(p.amount_display) AS total_paid_display,
    COUNT(*)              AS payout_count
FROM payouts p
JOIN created c ON p.event_id = c.event_id
GROUP BY 1
ORDER BY 2 DESC
