-- Boundless On-chain: EventCreated decode test
-- Use this query first to verify the Dune pipeline is correctly decoding
-- events for the deployed contract address.
--
-- Expected columns and types:
--   event_id          BIGINT    (u64 from contract)
--   pillar            VARCHAR   ('Hackathon' | 'Bounty' | 'Grant' | 'Crowdfunding')
--   owner             VARCHAR   (StrKey G... address)
--   token             VARCHAR   (StrKey address)
--   total_budget_raw  BIGINT    (stroops, raw)
--   total_budget_usdc DOUBLE    (divided by 1e7)
--   title             VARCHAR
--   tx_hash           VARCHAR
--   ledger            BIGINT
--   closed_at         TIMESTAMP
--
-- If this returns rows, the pipeline is live and all other queries will work.

SELECT
    CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.id')            AS BIGINT)   AS event_id,
    JSON_EXTRACT_SCALAR(data_decoded, '$.pillar')                          AS pillar,
    JSON_EXTRACT_SCALAR(data_decoded, '$.owner')                           AS owner,
    JSON_EXTRACT_SCALAR(data_decoded, '$.token')                           AS token,
    CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.total_budget')  AS BIGINT)   AS total_budget_raw,
    CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.total_budget')  AS DOUBLE) / 1e7
                                                                           AS total_budget_usdc,
    JSON_EXTRACT_SCALAR(data_decoded, '$.title')                           AS title,
    transaction_hash                                                       AS tx_hash,
    ledger_sequence                                                        AS ledger,
    closed_at
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
ORDER BY ledger_sequence DESC
LIMIT 50
