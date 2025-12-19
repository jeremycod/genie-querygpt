-- Create materialized views for latest data based on actual schema
-- These views maintain the latest version of each record, including soft-deleted records

-- Drop existing materialized views/tables if they exist
DROP TABLE IF EXISTS offers_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS campaigns_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS products_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS discounts_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS skus_latest CASCADE;

-- Create offers_latest materialized view
CREATE MATERIALIZED VIEW offers_latest AS
SELECT DISTINCT ON (id, profile) 
    id, name, description, discount_id, legacy, author, datetime, profile, 
    version, deleted, start_date, end_date, type, status, attributes, 
    ts_name, billing_frequency, countries, currency_code, brands, 
    archived, is_prototype
FROM offers 
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX offers_latest_id_profile_idx 
ON offers_latest (id, profile);

-- Create campaigns_latest materialized view
CREATE MATERIALIZED VIEW campaigns_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description, partner_id, legacy, author, datetime, profile,
    version, deleted, attributes
FROM campaigns
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX campaigns_latest_id_profile_idx 
ON campaigns_latest (id, profile);

-- Create products_latest materialized view
CREATE MATERIALIZED VIEW products_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description, legacy, author, datetime, profile,
    version, deleted, attributes
FROM products
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX products_latest_id_profile_idx 
ON products_latest (id, profile);

-- Create discounts_latest materialized view
CREATE MATERIALIZED VIEW discounts_latest AS
SELECT DISTINCT ON (id, profile)
    id, currency, legacy, author, datetime, profile,
    version, deleted, attributes
FROM discounts
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX discounts_latest_id_profile_idx 
ON discounts_latest (id, profile);

-- Create skus_latest materialized view
CREATE MATERIALIZED VIEW skus_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description, platform, countries, author, datetime, profile,
    version, deleted, legacy, billing_type, attributes
FROM skus
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX skus_latest_id_profile_idx 
ON skus_latest (id, profile);