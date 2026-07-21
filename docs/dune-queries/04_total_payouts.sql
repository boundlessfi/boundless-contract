-- Boundless On-chain: Total payouts to builders
-- Panel type: bar chart  x=month  y=total_paid_usdc  color=payout_type
--
-- WinnerPaid       → Hackathon / Bounty (single-release, paid at select_winners)
-- MilestoneClaimed → Grant / Crowdfunding (multi-release, paid per milestone)

SELECT
    DATE_TRUNC('month', closed_at)                                                AS month,
    JSON_EXTRACT_SCALAR(topics_decoded, '$[0]')                                   AS payout_type,
    COUNT(*)                                                                      AS payout_count,
    COUNT(DISTINCT JSON_EXTRACT_SCALAR(data_decoded, '$.recipient'))              AS unique_recipients,
    SUM(CAST(JSON_EXTRACT_SCALAR(data_decoded, '$.amount') AS DOUBLE)) / 1e7     AS total_paid_usdc
FROM stellar.history_contract_events
WHERE contract_id = '{{CONTRACT_ADDRESS}}'
  AND JSON_EXTRACT_SCALAR(topics_decoded, '$[0]') IN ('WinnerPaid', 'MilestoneClaimed')
GROUP BY 1, 2
ORDER BY 1 DESC, 2 ASC
