-- Boundless On-chain: TVL over time (daily running balance)
-- Panel type: area chart  x=day  y=tvl_display
--
-- Decoding (see 10_event_created_decode_test.sql): event name is
-- topics_decoded '$[0].symbol' (snake_case), fields come from the
-- data_decoded '$.map' ScVal map, read by type ($.i128, ...).

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
      AND data_decoded LIKE '%"map"%'
),
signed AS (
    SELECT
        day,
        CASE
            -- Inflow: non-crowdfunding creation escrows the budget
            WHEN ev_name = 'event_created'
              AND JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol') <> 'Crowdfunding'
            THEN  CAST(JSON_EXTRACT_SCALAR(f['total_budget'], '$.i128') AS DOUBLE)

            -- Inflow: add_funds (crowdfunding + partner top-ups)
            WHEN ev_name = 'funds_added'
            THEN  CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE)

            -- Outflow: payouts and refunds
            WHEN ev_name IN ('winner_paid', 'milestone_claimed',
                             'contributor_refunded', 'owner_residual_refunded')
            THEN -CAST(JSON_EXTRACT_SCALAR(f['amount'], '$.i128') AS DOUBLE)

            ELSE 0
        END AS delta
    FROM ev
    WHERE ev_name IN ('event_created', 'funds_added', 'winner_paid',
                      'milestone_claimed', 'contributor_refunded', 'owner_residual_refunded')
),
daily_delta AS (
    SELECT day, SUM(delta) / 1e7 AS daily_change_display
    FROM signed
    GROUP BY 1
)
SELECT
    day,
    daily_change_display,
    SUM(daily_change_display) OVER (ORDER BY day ROWS UNBOUNDED PRECEDING) AS tvl_display
FROM daily_delta
ORDER BY day ASC
