-- Example: Find products that have no active prices defined
-- Use case: Data quality check, identify products missing pricing
-- Demonstrates: LEFT JOIN with NULL check, filtering by active flag

SELECT
  p.id AS product_id,
  p.name AS product_name,
  p.description
FROM products_latest p
LEFT JOIN prices pr
  ON pr.product_id = p.id
  AND pr.profile = p.profile
  AND pr.active = true
WHERE
  p.profile = 'main'
  AND COALESCE(p.deleted, false) = false
  AND pr.id IS NULL
ORDER BY
  p.id;
