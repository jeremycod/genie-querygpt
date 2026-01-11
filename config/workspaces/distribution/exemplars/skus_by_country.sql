-- Example: Find SKUs available in specific countries
-- Use case: Regional SKU availability, market analysis
-- Demonstrates: Array overlap filtering, country-based queries

SELECT
  s.id AS sku_id,
  s.name AS sku_name,
  s.platform,
  s.countries,
  s.billing_type
FROM skus_latest s
WHERE
  s.profile = 'main'
  AND COALESCE(s.deleted, false) = false
  AND s.countries && ARRAY['US', 'CA', 'GB']  -- North America and UK
ORDER BY
  s.platform, s.id;
