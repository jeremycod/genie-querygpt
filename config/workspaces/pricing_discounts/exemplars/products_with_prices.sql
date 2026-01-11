-- Example: List all active products with their prices across all currencies
-- Use case: Product catalog with pricing information
-- Demonstrates: Product-price join, currency handling, price conversion

SELECT
  p.id AS product_id,
  p.name AS product_name,
  pr.id AS price_id,
  pr.currency,
  pr.amount AS amount_minor_units,
  CAST(pr.amount AS DECIMAL) / 100 AS amount_major_units,
  pr.active AS price_active
FROM products_latest p
LEFT JOIN prices pr
  ON pr.product_id = p.id
  AND pr.profile = p.profile
WHERE
  p.profile = 'main'
  AND COALESCE(p.deleted, false) = false
  AND (pr.active = true OR pr.active IS NULL)
ORDER BY
  p.id, pr.currency;
