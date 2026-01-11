-- Example: List all active discounts with usage count
-- Use case: Discount inventory, identify unused discounts
-- Demonstrates: Aggregation with LEFT JOIN, counting offers per discount

SELECT
  d.id AS discount_id,
  d.currency AS discount_currency,
  COUNT(o.id) AS offer_count
FROM discounts_latest d
LEFT JOIN offers_latest o
  ON o.discount_id = d.id
  AND o.profile = d.profile
  AND COALESCE(o.deleted, false) = false
WHERE
  d.profile = 'main'
  AND COALESCE(d.deleted, false) = false
GROUP BY
  d.id, d.currency
ORDER BY
  offer_count DESC, d.id;
