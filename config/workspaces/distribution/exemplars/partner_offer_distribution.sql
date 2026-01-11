-- Example: Show partner distribution with campaign and offer counts
-- Use case: Partner performance analysis, channel effectiveness
-- Demonstrates: Multi-level aggregation, partner→campaign→offer chain

WITH latest_partners AS (
  SELECT DISTINCT ON (id, profile)
    id, profile, version, deleted
  FROM partners
  WHERE profile = 'main'
  ORDER BY id, profile, version DESC
)
SELECT
  lp.id AS partner_id,
  COUNT(DISTINCT c.id) AS campaign_count,
  COUNT(DISTINCT co.offer_id) AS offer_count,
  STRING_AGG(DISTINCT c.id, ', ' ORDER BY c.id) AS campaign_ids
FROM latest_partners lp
LEFT JOIN campaigns_latest c
  ON c.partner_id = lp.id
  AND c.profile = lp.profile
  AND COALESCE(c.deleted, false) = false
LEFT JOIN campaign_offers co
  ON co.campaign_id = c.id
  AND co.profile = c.profile
  AND co.version = c.version
  AND COALESCE(co.deleted, false) = false
WHERE
  COALESCE(lp.deleted, false) = false
GROUP BY
  lp.id
ORDER BY
  offer_count DESC, campaign_count DESC, lp.id;
