# Genie DB — QueryGPT-Style Text-to-SQL System
## Version-Safe Design with “Latest” Materialized Views

---

## Purpose

This document describes a complete, production-ready design for a **QueryGPT-style text-to-SQL system** built on top of **Genie DB**.

The system enables users to describe complex analytical or export-style reports in **natural language**, and reliably generates, validates, explains, and executes SQL queries while respecting Genie DB’s **versioned data model**, **multi-tenant profiles**, and **business semantics**.

This design is inspired by Uber’s QueryGPT, but adapted specifically for Genie DB.

---

## Core Design Principles

1. **All core business entities are versioned**
2. “Latest” must be explicit and centralized
3. Version ownership differs by table type
4. LLMs must never infer joins or versions
5. SQL generation must be pattern-driven, not creative

---

## High-Level Architecture

```
User (Natural Language Report Request)
        ↓
Intent Agent
        ↓
Workspace Resolver
        ↓
Table Agent (version-safe join plan)
        ↓
Column Prune Agent
        ↓
RAG SQL Generator
        ↓
Static Validator
        ↓
Explainer
        ↓
SQL Execution
```

---

## 1. Workspaces (Domain Partitioning)

Workspaces constrain schemas, joins, and examples so the LLM operates within safe boundaries.

### 1.1 Campaigns & Offers Workspace

Used for campaign/offer exports and reports.

**Entity Heads (always latest):**
- offers_latest
- campaigns_latest

**Satellite Tables:**
- offer_phases (versioned by offer)
- offer_products (versioned by offer)
- campaign_offers (versioned by campaign)
- partners

### Join Semantics

| Table | Version Ownership |
|-----|------------------|
| offers_latest | offer |
| offer_phases | offer |
| offer_products | offer |
| campaign_offers | campaign |
| campaigns_latest | campaign |

---

### 1.2 Products & SKUs Workspace

**Entity Heads:**
- products_latest
- skus_latest

**Satellites:**
- offer_products
- sku_offers

---

### 1.3 Pricing & Discounts Workspace

**Entity Heads:**
- discounts_latest

**Satellites:**
- pricing / amount tables

---

## 2. Intent Agent

The Intent Agent classifies the request into:

- Workspace
- Task type (export, count, trend, aggregation)
- Filter hints (countries, statuses, promo types)

Example:

```
"Campaigns with Pre-Paid offers in APAC"
→ Workspace: Campaigns & Offers
→ Task: Export
→ Filters: promo_type=PREPAID, countries=APAC
```

---

## 3. Table Agent (Critical for Correctness)

The Table Agent selects tables and produces a **version-safe join plan**.

### Golden Rules

- Entity heads → always *_latest
- Offer satellites → match offer version
- Campaign links → match campaign version
- Never match campaign_offers.version to offer version

### Correct Join Pattern

```sql
offers_latest o
→ offer_phases op
  ON op.offer_id=o.id AND op.profile=o.profile AND op.version=o.version

offers_latest o
→ campaign_offers co
  ON co.offer_id=o.id AND co.profile=o.profile

campaign_offers co
→ campaigns_latest c
  ON c.id=co.campaign_id
 AND c.profile=co.profile
 AND co.version=c.version
```

---

## 4. Column Prune Agent

Reduces schema size before LLM usage.

**Keeps:**
- Requested output columns
- Filter columns
- Join keys
- Derived fields

**Removes:**
- Unused columns
- Large JSON blobs unless referenced

---

## 5. Retrieval-Augmented SQL Generation (RAG)

### Inputs to the LLM

- Schema Cards (pruned)
- Join recipe
- 2–4 exemplar SQL queries
- Derived field definitions

### Enforced SQL Patterns

- Use *_latest views
- Explicit joins
- Approved CASE expressions
- Approved aggregations

---

## 6. Static Validation Layer

Before execution:

- Table existence
- Column existence
- GROUP BY validation
- Join safety
- Version correctness
- Profile isolation

Queries failing validation are rejected.

---

## 7. Explanation Layer

Each generated query includes a plain-English explanation:

- Tables used
- Join logic
- Filters applied
- Derived fields

---

## 8. Latest Materialized View Strategy

### Key Decision

**Deleted rows are not filtered out.**  
If the latest version is deleted, it is still considered the authoritative latest row.

Consumers may filter manually if needed.

---

## 9. Entities with Latest Materialized Views

| Entity | Latest MV |
|------|-----------|
| Offers | offers_latest |
| Campaigns | campaigns_latest |
| Products | products_latest |
| Discounts | discounts_latest |
| SKUs | skus_latest |

---

## 10. Materialized View Pattern

```sql
ROW_NUMBER() OVER (
  PARTITION BY id, profile
  ORDER BY version DESC
) = 1
```

No filtering on deleted.

---

## 11. Refresh Strategy

### Constraints

- REFRESH MATERIALIZED VIEW CONCURRENTLY cannot run inside triggers
- Must not block writers

### Recommended Approach

1. Triggers emit NOTIFY events on INSERT / UPDATE(version, deleted)
2. External worker LISTENs and debounces
3. Worker refreshes MVs using CONCURRENTLY

Cron-based refresh is also acceptable if minute-level freshness is enough.

---

## 12. Human-Readable Report DSL

```yaml
report: Campaigns with Pre-Paid Offers in APAC
workspace: Campaigns & Offers
filters:
  promo_type: PREPAID
  countries_any_of: [KR, JP, TW, SG, HK]
fields:
  - partner.id
  - campaign.id
  - campaign.name
  - offer.id
  - offer.name
  - derived.expired_or_live_status
  - offer.countries
  - agg.products
```

---

## 13. Example Generated SQL

```sql
SELECT
  p.id AS partnership_id,
  c.id AS campaign_id,
  c.name AS campaign_name,
  o.id AS offer_id,
  o.name AS offer_name,
  CASE WHEN o.end_date::date < CURRENT_DATE
       THEN 'EXPIRED' ELSE o.status END AS expired_or_live_status,
  o.status,
  o.countries,
  STRING_AGG(DISTINCT opr.product_id, ',') AS products
FROM offers_latest o
JOIN offer_phases op
  ON op.offer_id=o.id AND op.profile=o.profile AND op.version=o.version
JOIN offer_products opr
  ON opr.offer_id=o.id AND opr.profile=o.profile AND opr.version=o.version
JOIN campaign_offers co
  ON co.offer_id=o.id AND co.profile=o.profile
JOIN campaigns_latest c
  ON c.id=co.campaign_id AND c.profile=co.profile AND co.version=c.version
LEFT JOIN partners p
  ON p.id=c.partner_id AND p.profile=c.profile
WHERE
  op.legacy->>'phase_type'='PREPAID'
GROUP BY
  p.id, c.id, c.name, o.id, o.name, o.end_date, o.status, o.countries;
```

---

## 14. Why This Design Works

- Version correctness by construction
- No LLM guesswork
- Deleted semantics preserved
- Low token usage
- Highly explainable
- Easy to extend

---

## Final Recommendation

Treat the LLM as a **SQL compiler**, not a database detective.

Centralize version logic, constrain joins, and let materialized views and agents do the hard work.

---
