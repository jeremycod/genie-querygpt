-- Example: Compare prices for the same product across different currencies
-- Use case: International pricing analysis, currency arbitrage detection
-- Demonstrates: Aggregation by product, multiple currency display, price conversion

SELECT
  p.id AS product_id,
  p.name AS product_name,
  COUNT(DISTINCT pr.currency) AS currency_count,
  STRING_AGG(
    DISTINCT pr.currency || ': ' || TO_CHAR(CAST(pr.amount AS DECIMAL) / 100, 'FM999999999.00'),
    ', '
    ORDER BY pr.currency || ': ' || TO_CHAR(CAST(pr.amount AS DECIMAL) / 100, 'FM999999999.00')
  ) AS prices_by_currency
FROM products_latest p
INNER JOIN prices pr
  ON pr.product_id = p.id
  AND pr.profile = p.profile
WHERE
  p.profile = 'main'
  AND COALESCE(p.deleted, false) = false
  AND pr.active = true
GROUP BY
  p.id, p.name
HAVING
  COUNT(DISTINCT pr.currency) > 1
ORDER BY
  currency_count DESC, p.id;
