-- Boundless On-chain: Unique builder and organizer wallets
-- Panel type: counter (two numbers side-by-side)
--
-- topics_decoded index 0 = event name symbol
-- data_decoded   = JSON object with event fields

-- Unique builders: wallets that applied to or received a payout from any event
SELECT
    'builders' AS role,
    COUNT(DISTINCT addr) AS unique_wallets
FROM (
    SELECT JSON_EXTRACT_SCALAR(data_decoded, '$.applicant') AS addr
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'Applied'

    UNION

    SELECT JSON_EXTRACT_SCALAR(data_decoded, '$.recipient') AS addr
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN ('WinnerPaid', 'MilestoneClaimed')
) t

UNION ALL

-- Unique organizers: wallets that created at least one event
SELECT
    'organizers' AS role,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(data_decoded, '$.owner')) AS unique_wallets
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
