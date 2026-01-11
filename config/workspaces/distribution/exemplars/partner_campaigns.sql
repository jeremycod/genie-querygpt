-- Example: List all partners with their campaigns
-- Use case: Partner reporting, understand partner activity
-- Demonstrates: Partner-campaign join, handling partner versioning

WITH latest_partners AS (
  SELECT DISTINCT ON (id, profile)
    id, profile, version, deleted
  FROM partners
  WHERE profile = 'main'
  ORDER BY id, profile, version DESC
)
SELECT
  lp.id AS partner_id,
  c.id AS campaign_id,
  c.name AS campaign_name,
  c.description AS campaign_description
FROM latest_partners lp
LEFT JOIN campaigns_latest c
  ON c.partner_id = lp.id
  AND c.profile = lp.profile
WHERE
  COALESCE(lp.deleted, false) = false
  AND (c.deleted IS NULL OR COALESCE(c.deleted, false) = false)
ORDER BY
  lp.id, c.id;
