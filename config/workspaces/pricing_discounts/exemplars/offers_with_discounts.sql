-- Example: List offers with their associated discounts
-- Use case: Discount effectiveness analysis, promotional offers review
-- Demonstrates: Offer-discount join, handling nullable discount_id

SELECT
  o.id AS offer_id,
  o.name AS offer_name,
  o.status AS offer_status,
  d.id AS discount_id,
  d.currency AS discount_currency
FROM offers_latest o
LEFT JOIN discounts_latest d
  ON d.id = o.discount_id
  AND d.profile = o.profile
WHERE
  o.profile = 'main'
  AND COALESCE(o.deleted, false) = false
  AND COALESCE(d.deleted, false) = false
  AND o.discount_id IS NOT NULL
ORDER BY
  o.id;
