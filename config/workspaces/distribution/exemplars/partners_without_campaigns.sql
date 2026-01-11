-- Example: Find partners that have no active campaigns
-- Use case: Data quality check, identify inactive partners
-- Demonstrates: LEFT JOIN with NULL check, partner versioning

WITH latest_partners AS (
  SELECT DISTINCT ON (id, profile)
    id, profile, version, deleted
  FROM partners
  WHERE profile = 'main'
  ORDER BY id, profile, version DESC
)
SELECT
  lp.id AS partner_id,
  lp.version AS partner_version
FROM latest_partners lp
LEFT JOIN campaigns_latest c
  ON c.partner_id = lp.id
  AND c.profile = lp.profile
  AND COALESCE(c.deleted, false) = false
WHERE
  COALESCE(lp.deleted, false) = false
  AND c.id IS NULL
ORDER BY
  lp.id;
