-- Boundless On-chain: Current TVL
-- Panel type: counter
-- Description: Total escrow balance currently held by the contract.
--
-- Real Dune column names (stellar.history_contract_events):
--   topics_decoded  VARCHAR  — JSON array; index 0 is the event-name symbol
--   data_decoded    VARCHAR  — JSON object with all event fields
--
-- Inflows:
--   EventCreated.total_budget  (non-Crowdfunding pillars — escrowed at creation)
--   FundsAdded.amount          (partner top-ups and crowdfunding contributions)
-- Outflows:
--   WinnerPaid.amount          (single-release payout at select_winners)
--   MilestoneClaimed.amount    (grant / crowdfunding milestone payout)
--   ContributorRefunded.amount (partner refund during paged cancel)
--   OwnerResidualRefunded.amount (owner residual at cancel)
--
-- All amounts are net-of-fee (protocol fee is deducted before events fire).
-- Divide by 1e7 to convert from stroops to USDC / XLM display units.

WITH inflows AS (
    -- Non-crowdfunding events: budget deposited at creation
    SELECT CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.total_budget') AS DOUBLE) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'EventCreated'
      AND JSON_EXTRACT_SCALAR(data_decoded, '$.pillar') != 'Crowdfunding'

    UNION ALL

    -- All add_funds deposits (crowdfunding contributions + partner top-ups)
    SELECT CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount') AS DOUBLE) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') = 'FundsAdded'
),
outflows AS (
    SELECT CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount') AS DOUBLE) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN (
          'WinnerPaid',
          'MilestoneClaimed',
          'ContributorRefunded',
          'OwnerResidualRefunded'
      )
)
SELECT
    (COALESCE(SUM(i.amount), 0) - COALESCE(SUM(o.amount), 0)) / 1e7 AS tvl_usdc
FROM inflows i
FULL OUTER JOIN outflows o ON 1 = 1
