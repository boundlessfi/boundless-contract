-- Boundless On-chain: event_created decode test
-- Run this FIRST to confirm the Dune pipeline decodes events for the contract.
--
-- Real stellar.history_contract_events shapes (verified against live mainnet):
--   topics_decoded : JSON array of ScVal objects. Event name is at $[0].symbol
--                    and is SNAKE_CASE — #[contractevent] lowercases the struct
--                    name, so EventCreated is emitted as 'event_created',
--                    WinnerPaid as 'winner_paid', etc. NOT PascalCase.
--   data_decoded   : ScVal map — {"map":[{"key":{"symbol":"id"},"val":{"u64":"7"}}, ...]}.
--                    Each field's value is wrapped by its ScVal type; there is no
--                    flat $.id. We rebuild it into a MAP(field_name -> ScVal JSON)
--                    with map_from_entries(), then read each field by its type.
--   closed_at_date : PARTITION column — always filter it (avoids full scans).
--   transaction_hash : varbinary — to_hex() for a readable hash.

WITH ev AS (
    SELECT
        transaction_hash,
        ledger_sequence,
        closed_at,
        map_from_entries(
            transform(
                CAST(JSON_EXTRACT(data_decoded, '$.map') AS ARRAY(JSON)),
                e -> ROW(
                    JSON_EXTRACT_SCALAR(e, '$.key.symbol'),
                    JSON_EXTRACT(e, '$.val')
                )
            )
        ) AS f
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND closed_at_date >= DATE '{{START_DATE}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0].symbol') = 'event_created'
)
SELECT
    CAST(JSON_EXTRACT_SCALAR(f['id'], '$.u64') AS BIGINT)                    AS event_id,
    -- Pillar is a unit enum. Soroban serializes it as a 1-element vec of a
    -- symbol; if this column is NULL on the first real event, inspect
    -- pillar_raw below and switch the path (e.g. '$.symbol').
    JSON_EXTRACT_SCALAR(f['pillar'], '$.vec[0].symbol')                     AS pillar,
    f['pillar']                                                             AS pillar_raw,
    JSON_EXTRACT_SCALAR(f['owner'], '$.address')                           AS owner,
    JSON_EXTRACT_SCALAR(f['token'], '$.address')                           AS token,
    CAST(JSON_EXTRACT_SCALAR(f['total_budget'], '$.i128') AS DECIMAL(38,0)) AS total_budget_raw,
    CAST(JSON_EXTRACT_SCALAR(f['total_budget'], '$.i128') AS DOUBLE) / 1e7  AS total_budget_display,
    JSON_EXTRACT_SCALAR(f['title'], '$.string')                            AS title,
    to_hex(transaction_hash)                                               AS tx_hash,
    ledger_sequence                                                        AS ledger,
    closed_at
FROM ev
ORDER BY ledger_sequence DESC
LIMIT 50
