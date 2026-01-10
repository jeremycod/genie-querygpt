-- Create materialized views for latest data based on actual schema
-- These views maintain the latest version of each record, including soft-deleted records
--
-- IMPORTANT: The 'legacy' column is cast from TEXT to JSONB to enable efficient querying
-- Examples of JSONB queries on legacy field:
--   - Extract field: SELECT legacy->>'hulu_discount_id' FROM offers_latest;
--   - Filter by field: WHERE legacy->>'namespace' = 'disney'
--   - Check existence: WHERE legacy ? 'effective_date'
--   - Array contains: WHERE legacy->'disney_campaign' @> '["DISNEY_CYOS_2024_PROMO_CAMPAIGN"]'

-- Drop existing materialized views/tables if they exist
DROP MATERIALIZED VIEW IF EXISTS mayoffers_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS campaigns_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS products_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS discounts_latest CASCADE;
DROP MATERIALIZED VIEW IF EXISTS skus_latest CASCADE;

-- Create offers_latest materialized view
CREATE MATERIALIZED VIEW offers_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description, discount_id,
    CASE
        WHEN legacy IS NULL THEN NULL
        WHEN legacy = '' THEN NULL
        ELSE legacy::jsonb
    END AS legacy,
    author, datetime, profile,
    version, deleted, start_date, end_date, type, status, attributes,
    ts_name, billing_frequency, countries, currency_code, brands,
    archived, is_prototype
FROM offers
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX offers_latest_id_profile_idx
ON offers_latest (id, profile);

-- GIN index on legacy JSONB column for efficient querying
CREATE INDEX offers_latest_legacy_idx
ON offers_latest USING gin (legacy);

-- Create campaigns_latest materialized view
CREATE MATERIALIZED VIEW campaigns_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description, partner_id,
    CASE
        WHEN legacy IS NULL THEN NULL
        WHEN legacy = '' THEN NULL
        ELSE legacy::jsonb
    END AS legacy,
    author, datetime, profile,
    version, deleted, attributes
FROM campaigns
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX campaigns_latest_id_profile_idx
ON campaigns_latest (id, profile);

-- GIN index on legacy JSONB column for efficient querying
CREATE INDEX campaigns_latest_legacy_idx
ON campaigns_latest USING gin (legacy);

-- Create products_latest materialized view
CREATE MATERIALIZED VIEW products_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description,
    CASE
        WHEN legacy IS NULL THEN NULL
        WHEN legacy = '' THEN NULL
        ELSE legacy::jsonb
    END AS legacy,
    author, datetime, profile,
    version, deleted, attributes
FROM products
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX products_latest_id_profile_idx
ON products_latest (id, profile);

-- GIN index on legacy JSONB column for efficient querying
CREATE INDEX products_latest_legacy_idx
ON products_latest USING gin (legacy);

-- Create discounts_latest materialized view
CREATE MATERIALIZED VIEW discounts_latest AS
SELECT DISTINCT ON (id, profile)
    id, currency,
    CASE
        WHEN legacy IS NULL THEN NULL
        WHEN legacy = '' THEN NULL
        ELSE legacy::jsonb
    END AS legacy,
    author, datetime, profile,
    version, deleted, attributes
FROM discounts
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX discounts_latest_id_profile_idx
ON discounts_latest (id, profile);

-- GIN index on legacy JSONB column for efficient querying
CREATE INDEX discounts_latest_legacy_idx
ON discounts_latest USING gin (legacy);

-- Create skus_latest materialized view
CREATE MATERIALIZED VIEW skus_latest AS
SELECT DISTINCT ON (id, profile)
    id, name, description, platform, countries, author, datetime, profile,
    version, deleted,
    CASE
        WHEN legacy IS NULL THEN NULL
        WHEN legacy = '' THEN NULL
        ELSE legacy::jsonb
    END AS legacy,
    billing_type, attributes
FROM skus
ORDER BY id, profile, version DESC;

CREATE UNIQUE INDEX skus_latest_id_profile_idx
ON skus_latest (id, profile);

-- GIN index on legacy JSONB column for efficient querying
CREATE INDEX skus_latest_legacy_idx
ON skus_latest USING gin (legacy);