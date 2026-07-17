-- Boundless On-chain: Current TVL
-- Panel type: counter
-- Description: Total escrow balance currently held by the contract.
--
-- Inflows:
--   EventCreated.total_budget  (non-Crowdfunding pillars — escrowed at creation)
--   FundsAdded.amount          (partner top-ups and crowdfunding contributions)
-- Outflows:
--   WinnerPaid.amount          (single-release payout at select_winners)
--   MilestoneClaimed.amount    (grant / crowdfunding milestone payout)
--   ContributorRefunded.amount (partner refund during cancel)
--   OwnerResidualRefunded.amount (owner residual at cancel)
--
-- All amounts are net-of-fee (protocol fee is deducted before events are emitted).
-- Divide by 1e7 to convert from stroops to USDC / XLM display units.

WITH inflows AS (
    -- Non-crowdfunding events: budget deposited at creation
    SELECT CAST(JSON_EXTRACT_SCALAR(data, '$.total_budget') AS DOUBLE) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'EventCreated'
      AND JSON_EXTRACT_SCALAR(data, '$.pillar') != 'Crowdfunding'

    UNION ALL

    -- All add_funds deposits (crowdfunding + partner top-ups)
    SELECT CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 = 'FundsAdded'
),
outflows AS (
    SELECT CAST(JSON_EXTRACT_SCALAR(data, '$.amount') AS DOUBLE) AS amount
    FROM stellar.history_contract_events
    WHERE contract_id = '{{CONTRACT_ADDRESS}}'
      AND topic_1 IN (
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
