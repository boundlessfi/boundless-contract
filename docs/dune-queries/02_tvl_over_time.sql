-- Boundless On-chain: TVL over time (daily running balance)
-- Panel type: area chart  x=day  y=tvl_usdc
--
-- topics_decoded: JSON array — index 0 is the event-name symbol
-- data_decoded:   JSON object — field values

WITH raw_events AS (
    SELECT
        DATE_TRUNC('day', closed_at) AS day,
        CASE
            -- Inflow: non-crowdfunding creation
            WHEN JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
              AND JSON_EXTRACT_SCALAR(data_decoded, '$.pillar') != 'Crowdfunding'
            THEN  CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.total_budget') AS DOUBLE)

            -- Inflow: add_funds (crowdfunding + partner top-ups)
            WHEN JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'FundsAdded'
            THEN  CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount') AS DOUBLE)

            -- Outflow: winner / milestone payouts and refunds
            WHEN JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN (
                'WinnerPaid', 'MilestoneClaimed',
                'ContributorRefunded', 'OwnerResidualRefunded'
            )
            THEN -CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount') AS DOUBLE)

            ELSE 0
        END AS delta
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN (
          'EventCreated', 'FundsAdded',
          'WinnerPaid', 'MilestoneClaimed',
          'ContributorRefunded', 'OwnerResidualRefunded'
      )
),
daily_delta AS (
    SELECT
        day,
        SUM(delta) / 1e7 AS daily_change_usdc
    FROM raw_events
    GROUP BY 1
)
SELECT
    day,
    daily_change_usdc,
    SUM(daily_change_usdc) OVER (ORDER BY day ROWS UNBOUNDED PRECEDING) AS tvl_usdc
FROM daily_delta
ORDER BY day ASC
