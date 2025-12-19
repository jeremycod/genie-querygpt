# QueryGPT (Genie DB) — Recommended Implementation Roadmap

This document describes **practical, staged next steps** to implement QueryGPT end-to-end for a **restricted domain first**, then safely expand it by adding more workspaces.

The goal is to **ship something correct and usable early**, and only then introduce LLMs where they add value.

---

## Guiding principles

1. **Correctness beats intelligence**
   - Deterministic joins and version semantics must be enforced in code.
   - LLMs must *not* invent joins or tables.

2. **LLM ≠ database engine**
   - SQL generation should be compiler-like.
   - LLMs help with intent, mapping, and phrasing—not execution logic.

3. **Scale via configuration, not new logic**
   - New domains = new workspaces (Schema Cards + exemplars).
   - Core engine should remain unchanged.

---

# Phase 1 — Ship one production-quality workspace

### Choose a hard-but-contained workspace
Start with **Campaigns & Offers**, because it exercises:
- versioned entity heads (`offers_latest`, `campaigns_latest`)
- offer-owned satellites (`offer_phases`, `offer_products`)
- campaign-owned link tables (`campaign_offers.version → campaign version`)

If this workspace works, the rest are easy.

---

## 1.1 Build a deterministic compiler pipeline (no LLM yet)

**Goal:** Given a structured request, always generate correct SQL.

### Implement these components

#### `ReportSpec` (input)
A structured request format (JSON/YAML), e.g.:

```json
{
  "workspace": "campaigns_offers",
  "filters": {
    "promo_type": "PREPAID",
    "countries": ["KR","JP","TW","SG","HK"],
    "offer_status": ["PUBLISHED","EXPIRED"]
  },
  "fields": [
    "partnership_id",
    "campaign_id",
    "campaign_name",
    "offer_id",
    "offer_name",
    "products",
    "package_id",
    "expired_or_live_status"
  ],
  "order_by": ["partnership_id", "campaign_id", "offer_id"]
}
```

#### `IntermediatePlan` (compiler output)
Contains:
- selected entities
- fixed joins with version semantics
- projections
- filters
- group by / order by
- derived fields

#### SQL Renderer
- Renders joins **deterministically**
- Never lets joins be LLM-generated
- Uses plan → SQL mapping

**Result:** You can already generate and execute correct SQL end-to-end.

---

## 1.2 Add schema-aware validation

**Goal:** Fail fast on incorrect or unsafe queries.

### Required validations
- Read-only only (no INSERT/UPDATE/DELETE/DDL)
- Tables must be in workspace allowlist
- Joins must exist in Join Graph
- Version correctness:
  - offer satellites join on offer version
  - campaign links join on campaign version
- Columns must exist (Schema Cards first; pg_catalog optional)
- LIMIT required unless explicit export mode

Validation should run **before execution**.

---

## 1.3 Add execution endpoint

Add `/execute` endpoint to the server:
- Uses a read-only DB role
- Applies statement timeout
- Streams rows
- Optional CSV export mode

At this point you have:
> **A working, safe, non-LLM reporting engine**

---

## 1.4 Add explanation (from the plan, not SQL)

Generate explanation from `IntermediatePlan`:
- Which tables were used and why
- Which filters were applied
- What derived fields mean
- Why version semantics matter

This is essential for user trust.

---

## 1.5 Decide MV freshness strategy

For the first workspace, keep it simple:
- Option A: refresh `*_latest` MVs on a fixed interval (cron / pg_cron)
- Option B: LISTEN/NOTIFY worker (already scaffolded)

Cron is fine for v1.

---

# Phase 2 — Add LLM safely (constrained mode)

Once deterministic generation works, introduce LLMs carefully.

---

## 2.1 NL → ReportSpec (not NL → SQL)

**Golden rule:** LLM outputs **structured specs**, not raw SQL.

Flow:
```
User prompt
  → LLM
    → ReportSpec JSON
      → Compiler
        → Validator
          → Execution
```

LLM is no longer allowed to invent joins.

---

## 2.2 Add RAG only where it helps

Use RAG for:
- mapping user language → known fields
- mapping statuses and business terminology
- choosing derived fields

Do **not** use RAG to infer joins.

Start with:
- keyword search over exemplars
- add embeddings later if needed

---

## 2.3 Add human-in-the-loop control

Before execution:
- show explanation
- show SQL
- allow user to tweak spec (not SQL)
- recompile and revalidate

This prevents costly mistakes.

---

# Phase 3 — Scale by adding workspaces

At this point, the engine is stable.

---

## 3.1 Create a Workspace template

Each new workspace adds only **configuration**:

- `config/workspaces/<ws>.index.json`
- `config/workspaces/<ws>.schema_cards.json`
- `config/workspaces/<ws>/exemplars/*.sql`
- Join Graph edges
- Version semantics notes
- Derived fields

No new core logic.

---

## 3.2 Recommended workspace expansion order

1. **Campaigns & Offers** ✅
2. **Products & SKUs**
   - `products_latest`, `skus_latest`
   - simpler join graph
3. **Discounts & Pricing**
   - `discounts_latest`
   - pricing satellites

---

## 3.3 Add workspace-level tests

For each workspace:
- prompt/spec → expected plan snapshot
- expected SQL snapshot
- validation invariants
- execution sanity checks in staging DB

This prevents regressions.

---

# Milestone checklist

### Milestone A — Deterministic engine
- [ ] Implement `ReportSpec`
- [ ] Implement `IntermediatePlan`
- [ ] Implement compiler (`dsl::compile`)
- [ ] Implement SQL renderer
- [ ] Implement schema-aware validator
- [ ] Add `/execute` endpoint

### Milestone B — LLM integration
- [ ] NL → ReportSpec generation
- [ ] RAG exemplar retrieval
- [ ] Prompt constraints + tests
- [ ] Human approval loop

### Milestone C — Expansion
- [ ] Second workspace (products/skus)
- [ ] Workspace tests
- [ ] Docs + onboarding

---

## Final recommendation

If you do **one thing next**, do this:

> **Implement the compiler pipeline for one workspace without any LLM dependency.**

Once that works, everything else becomes incremental and safe.

---
