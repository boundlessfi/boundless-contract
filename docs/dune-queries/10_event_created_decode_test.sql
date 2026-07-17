-- Boundless On-chain: EventCreated decode test
-- Use this query to verify that the Dune pipeline is correctly decoding
-- EventCreated events for the deployed contract address.
--
-- Expected columns and types:
--   event_id         BIGINT    (u64 from contract)
--   pillar           VARCHAR   ('Hackathon' | 'Bounty' | 'Grant' | 'Crowdfunding')
--   owner            VARCHAR   (StrKey address, starts with 'G...')
--   token            VARCHAR   (StrKey address)
--   total_budget_raw BIGINT    (stroops, raw)
--   total_budget_usdc DOUBLE   (divide by 1e7)
--   title            VARCHAR
--   tx_hash          VARCHAR
--   ledger           BIGINT
--   closed_at        TIMESTAMP

SELECT
    CAST(JSON_EXTRACT_SCALAR(data, '$.id')           AS BIGINT)   AS event_id,
    JSON_EXTRACT_SCALAR(data, '$.pillar')                         AS pillar,
    JSON_EXTRACT_SCALAR(data, '$.owner')                          AS owner,
    JSON_EXTRACT_SCALAR(data, '$.token')                          AS token,
    CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS BIGINT)   AS total_budget_raw,
    CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE) / 1e7
                                                                  AS total_budget_usdc,
    JSON_EXTRACT_SCALAR(data, '$.title')                          AS title,
    transaction_hash                                              AS tx_hash,
    ledger_sequence                                               AS ledger,
    closed_at
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND topic_1 = 'EventCreated'
ORDER BY ledger_sequence DESC
LIMIT 50
