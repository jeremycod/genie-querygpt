-- Example: List all active SKUs grouped by platform
-- Use case: SKU inventory by platform, distribution channel analysis
-- Demonstrates: SKU filtering, platform grouping

SELECT
  platform,
  COUNT(*) AS sku_count,
  STRING_AGG(DISTINCT id, ', ' ORDER BY id) AS sku_ids
FROM skus_latest
WHERE
  profile = 'main'
  AND COALESCE(deleted, false) = false
  AND platform IS NOT NULL
GROUP BY
  platform
ORDER BY
  sku_count DESC, platform;
