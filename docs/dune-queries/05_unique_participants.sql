-- Boundless On-chain: Unique builder and organizer wallets
-- Panel type: counter (two rows: builders, organizers)
--
-- Decoding: event name is topics_decoded '$[0].symbol'; addresses are read
-- from the data_decoded '$.map' as '$.address'. See 10_event_created_decode_test.sql.

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
          IN ('Applied', 'WinnerPaid', 'MilestoneClaimed', 'EventCreated')
)
-- Builders: applied to or received a payout from any event
SELECT
    'builders' AS role,
    COUNT(DISTINCT addr) AS unique_wallets
FROM (
    SELECT JSON_EXTRACT_SCALAR(f['applicant'], '$.address') AS addr
    FROM ev WHERE ev_name = 'Applied'
    UNION
    SELECT JSON_EXTRACT_SCALAR(f['recipient'], '$.address')
    FROM ev WHERE ev_name IN ('WinnerPaid', 'MilestoneClaimed')
) t

UNION ALL

-- Organizers: created at least one event
SELECT
    'organizers' AS role,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(f['owner'], '$.address')) AS unique_wallets
FROM ev
WHERE ev_name = 'EventCreated'
