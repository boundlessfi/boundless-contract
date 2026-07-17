-- Boundless On-chain: Unique builder and organizer wallets
-- Panel type: counter (two numbers side-by-side)

-- Unique builders: wallets that applied to or received a payout from any event
SELECT
    'builders' AS role,
    COUNT(DISTINCT addr) AS unique_wallets
FROM (
    SELECT JSON_EXTRACT_SCALAR(data, '$.applicant')  AS addr
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'Applied'

    UNION

    SELECT JSON_EXTRACT_SCALAR(data, '$.recipient')  AS addr
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN ('WinnerPaid', 'MilestoneClaimed')
) t

UNION ALL

-- Unique organizers: wallets that created at least one event
SELECT
    'organizers' AS role,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(data, '$.owner')) AS unique_wallets
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 = 'EventCreated'
