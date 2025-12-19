-- Exemplar: Prepaid offers in APAC, joined to latest campaigns via campaign_offers version.
SELECT
  p.id AS partnership_id,
  c.id AS campaign_id,
  c.name AS campaign_name,
  o.id AS offer_id,
  o.name AS offer_name,
  CASE WHEN o.end_date::date < CURRENT_DATE THEN 'EXPIRED' ELSE o.status END AS expired_or_live_status,
  o.status AS workflow_status,
  o.countries,
  STRING_AGG(DISTINCT opr.product_id, ',') AS products,
  o.attributes ->> 'packageId' AS package
FROM offers_latest o
JOIN offer_phases op
  ON op.offer_id = o.id AND op.profile = o.profile AND op.version = o.version
JOIN offer_products opr
  ON opr.offer_id = o.id AND opr.profile = o.profile AND opr.version = o.version
JOIN campaign_offers co
  ON co.offer_id = o.id AND co.profile = o.profile
JOIN campaigns_latest c
  ON c.id = co.campaign_id AND c.profile = co.profile AND co.version = c.version
LEFT JOIN partners p
  ON p.id = c.partner_id AND p.profile = c.profile
WHERE
  op.legacy::jsonb ->> 'phase_type' = 'PREPAID'
  AND o.countries && ARRAY['KR','JP','TW','SG','HK'];
