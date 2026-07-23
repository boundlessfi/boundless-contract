-- Boundless On-chain: Total payouts to builders
-- Panel type: bar chart  x=month  y=total_paid_display  color=payout_type
--
-- WinnerPaid       -> Hackathon / Bounty (single-release; fires at claim_prize)
-- MilestoneClaimed -> Grant / Crowdfunding (multi-release, per milestone)
-- Both carry event_id, recipient (address), amount (i128).
-- Decoding: see 10_event_created_decode_test.sql.

WITH ev AS (
    SELECT
        DATE_TRUNC('month', closed_at) AS month,
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
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') IN ('WinnerPaid', 'MilestoneClaimed')
)
SELECT
    month,
    ev_name                                                                 AS payout_type,
    COUNT(*)                                                                AS payout_count,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(f['recipient'], '$.address'))         AS unique_recipients,
    SUM(CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE)) / 1e7    AS total_paid_display
FROM ev
GROUP BY 1, 2
ORDER BY 1 DESC, 2 ASC
