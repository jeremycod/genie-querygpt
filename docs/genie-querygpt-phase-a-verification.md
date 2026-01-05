# Genie QueryGPT – Phase A Verification and System Description

## Overview of Phase A

The Genie QueryGPT project aims to build a deterministic, compiler-style SQL generation pipeline and then gradually introduce AI assistance at the planning stage without allowing the model to influence semantics or SQL correctness. Phase A focused on creating a compiler that translates a structured ReportSpec into a canonical IntermediatePlan and then renders deterministic SQL. The repository's current state confirms that Phase A is complete and locked.

### Key Invariants

- **Human-authored ReportSpec → IntermediatePlan → SQL pipeline**: A report spec is a JSON/YAML structure defining selected fields, filters, ordering and pagination. The `compile_report_spec` function validates the spec against the schema, assigns deterministic table aliases, builds safe joins based on the schema's join graph, translates projections and filters and emits a canonical IntermediatePlan. The renderer (`render_sql`) serialises this plan without changing semantics and produces deterministic SQL.

- **Compiler owns semantics & determinism**: All join and field resolution logic lives in the compiler. The renderer simply prints the plan with a deterministic join order, stable root selection and fixed alias ordering. The compiler normalises join conditions (`normalize_join_condition_for_aliases`) by swapping LHS/RHS fields when aliases are reversed and emitting an error on invalid or ambiguous conditions.

- **Pagination support**: The compiler validates that limit and offset in the report spec are non-negative. Negative values result in `CompileError::InvalidLimit` or `InvalidOffset`, which are converted into structured diagnostics. Valid values are stored as `Option<u64>` in the plan and deterministically rendered in SQL.

- **Snapshot tests & determinism**: Tests compile a real report spec (e.g., `campaigns_offers_prepaid_apac.json`) and assert on a JSON snapshot of the resulting IntermediatePlan and the final SQL string. There are also tests verifying that rendering two plans with tables in different orders yields identical SQL. This ensures that the renderer's join ordering algorithm is deterministic.

- **Structured compiler diagnostics**: The compiler does not panic on errors; instead it returns a `CompileDiagnostics` containing stable error codes (`unknown_field`, `invalid_join`, `schema_mismatch`, `pagination_out_of_range`, etc.) and structured metadata. The `compile_report_spec` wrapper converts any `CompilerError` or `SpecError` into these diagnostics. Tests snapshot the diagnostic JSON for various error cases—unknown fields, invalid joins, schema mismatch and out-of-range pagination.

## Completed Phase A Features

| Feature | Evidence from repository |
|---------|--------------------------|
| **Join normalisation moved to compiler** | `normalize_join_condition_for_aliases` swaps fields when aliases are reversed and errors otherwise. The renderer no longer fixes joins. |
| **Unambiguous join logic** | The compiler builds joins only along edges of the schema's join graph. Invalid or malformed join expressions result in `InvalidJoin` diagnostics. |
| **Pagination support** | `compile_pagination` validates non-negative limit/offset and stores them in the plan. Tests snapshot pagination errors. |
| **IntermediatePlan & renderer determinism** | `compile_report_spec` builds a canonical plan with deterministic alias assignment and join order. `render_sql` chooses a root table, orders joins so each join's left alias has already been introduced, and renders SELECT, FROM/JOIN, WHERE, GROUP BY and ORDER BY clauses deterministically. |
| **Pipeline-level snapshot tests** | Test `full_query_snapshot_from_reportspec` compiles a prepaid APAC report spec and snapshots the resulting plan and SQL. Another test constructs different table orders but asserts that the rendered SQL is identical. |
| **Structured diagnostics** | `CompileDiagnostics` enumerates stable error codes and maps `CompileError`, `SpecError` and `CompilerError` to diagnostics. Snapshot tests verify unknown fields, invalid joins, schema mismatches and pagination errors. |
| **Planner trait defined** | A `Planner` trait with `suggest_report_spec` and `suggest_plan` methods and a `StubPlanner` implementation returning `PlannerError::Unimplemented` is defined. This lays the boundary for Phase B but no AI logic is implemented yet. |

## Example Walk-Through

To illustrate how the system currently works, consider the prepaid APAC export report spec used in tests. The spec requests partnership ID, campaign ID and name, offer ID and name, an `expired_or_live_status` derived field, the current workflow status, countries, a concatenated list of product IDs, and the package ID. Filters restrict to prepaid offers, APAC countries, and published/expired status, and the result is ordered by partnership, campaign and offer IDs:

```json
{
  "version": 1,
  "workspace": "campaigns_offers",
  "select": [
    { "field": "partnership_id" },
    { "field": "campaign_id" },
    { "field": "campaign_name" },
    { "field": "offer_id" },
    { "field": "offer_name" },
    { "field": "expired_or_live_status" },
    { "field": "workflow_status" },
    { "field": "countries" },
    { "field": "products_csv" },
    { "field": "package_id" }
  ],
  "filters": [
    { "field": "promo_type", "op": "eq", "value": "PREPAID" },
    { "field": "countries", "op": "overlaps", "value": ["KR","JP","TW","SG","HK"] },
    { "field": "workflow_status", "op": "in", "value": ["PUBLISHED","EXPIRED"] }
  ],
  "order_by": [
    { "field": "partnership_id", "dir": "asc" },
    { "field": "campaign_id", "dir": "asc" },
    { "field": "offer_id", "dir": "asc" }
  ],
  "mode": "export"
}
```

### Compilation

`compile_report_spec` loads the schema registry for the `campaigns_offers` workspace and resolves each field to its underlying entity. For example:
- `partnership_id` maps to the `partners` entity
- `campaign_id` and `campaign_name` map to `campaigns_latest`
- `offer_id` and `offer_name` map to `offers_latest`
- `expired_or_live_status` maps to a derived field defined in the schema cards
- `products_csv` maps to `offer_products`
- `promo_type` filter maps to `offer_phases`

The compiler assigns deterministic aliases (`p`, `c`, `o`, `co`, `oph`, `opr`) and selects the minimal set of joins required. Join predicates are generated from the schema's join graph and normalised so that the left/right aliases match the plan ordering.

### IntermediatePlan

The resulting IntermediatePlan contains:

- **A list of tables** with names and aliases (`offers_latest` as `o`, `campaigns_latest` as `c`, `campaign_offers` as `co`, `offer_phases` as `oph`, `offer_products` as `opr`, `partners` as `p`)

- **A set of PlanJoin entries** connecting these tables with INNER or LEFT joins. Each join has a list of normalised join conditions

- **A list of PlanProjection entries** mapping each requested field to a fully qualified SQL expression (e.g., `o.id`, `c.name`, `o.attributes ->> 'packageId'`). For derived fields, the compiler substitutes the SQL expression defined in the schema cards and replaces entity names with aliases

- **Filters translated into SQL expressions** (e.g., `oph.promo_type = 'PREPAID'`, `o.countries && ARRAY['KR','JP','TW','SG','HK']`, `o.status IN ('PUBLISHED','EXPIRED')`)

- **Ordering instructions** for `partnership_id`, `campaign_id` and `offer_id`

- **No limit/offset** because the mode is export

### Rendering

`render_sql` selects a root table (one not joined on the right side of any join) and deterministically orders the joins so each join's left table has already been introduced. It then prints:

- **SELECT** with each projection expression, including AS aliases when provided
- **FROM <root_table> <root_alias>**
- **A series of JOIN clauses** with fully normalised ON predicates
- **WHERE** with all filter predicates combined with AND
- **GROUP BY** if any projection expression is an aggregate and ORDER BY based on the plan's ordering

The final SQL is deterministic; it will always produce the same string for the same IntermediatePlan, and any change in table/field ordering or alias assignment in the input spec will still lead to identical output. Snapshot tests in `sql_render_full_query.rs` lock in this SQL string and thereby enforce pipeline stability.

## Unimplemented and Missing Components (Phase B)

While Phase A is complete, several components required for a full text-to-SQL system remain stubs or incomplete:

### Planner and AI Integration
The `Planner` trait exists but its implementation (`StubPlanner`) always returns an `Unimplemented` error. There is no logic to parse natural-language prompts into a ReportSpec or plan. Phase B proposes to implement a safe planner that suggests report specs based on user prompts and revises them in response to compiler diagnostics. This component must respect hard constraints: the LLM suggests but never decides, and all drafts are validated by the compiler before being executed.

### Workspace Resolver and Agents
In the expanded design document, the Intent Agent, Table Agent, Column Prune Agent, RAG retrieval, Prompt builder, Validator, Explainer and Telemetry modules are stubs. These are essential for going from natural language to a valid report spec and for providing user feedback and guardrails. For example:
- The Table Agent must use the schema's join graph to pick the minimal set of tables and safe joins
- The Column Prune Agent must reduce prompt size
- The Validator must block unsafe SQL

### AI Feedback Loop
The design calls for a bounded loop where the planner receives compiler diagnostics and revises the report spec. The current code does not support passing `CompileDiagnostics` back to the planner or iterating on the spec.

### Explainability
There is no implementation of the explain module. The system should provide human-readable explanations of the generated SQL, the chosen joins, and why certain version semantics are enforced. This is important for user trust and debugging.

### Server API and Execution
The `querygpt-server` crate contains only a skeleton HTTP server. There is no endpoint to execute the generated SQL against a database, and security controls (read-only roles, timeouts, row limits) are not yet implemented.

### SQL Validator
The static validator currently only checks that the SQL parses successfully using sqlparser. It does not enforce workspace boundaries, join safety or version semantics.

### Materialised View Worker & Freshness
Although the worker skeleton exists, the logic to listen for NOTIFY messages and refresh materialised views is not fully implemented.

## Plan for Building the Remaining Pieces

To move from a deterministic compiler to a complete QueryGPT-style assistant (Phase B and beyond), the following high-level steps are recommended:

### 1. Implement Structured Planner Loop

- Extend the existing `Planner` trait to return a draft report spec (`ReportSpecDraft`) instead of a final spec and include optional rationale/assumptions
- Add a `revise_report_spec(prompt, diagnostics)` method so the planner can adjust its draft in response to compiler diagnostics
- Begin with a rule-based or retrieval-augmented planner that uses the schema's derived fields, join graph and example specs to build a candidate spec
- Use heuristics or LLM suggestions to map natural-language filters and fields to schema names
- Always pass the planner's output through `compile_report_spec`. If compilation fails, convert the diagnostics into a user-friendly message and feed them back to the planner
- Limit the number of retries and require user confirmation before executing the final SQL

### 2. Develop the Intent, Table and Column Agents

- **Intent Agent**: classify the prompt to a workspace (e.g., `campaigns_offers`) and task (e.g., export). Start with keyword rules or a simple classifier; log which rule matched for transparency
- **Table Agent**: determine which entities are required to satisfy the requested fields and filters. Use the join graph to find the shortest path (Steiner tree) connecting these entities and ensure only safe joins are used
- **Column Prune Agent**: given the table plan and selected fields, compute the minimal set of columns, JSON paths and derived fields needed for the LLM prompt

### 3. Implement Retrieval-Augmented Generation (RAG)

- Index exemplar SQL files (e.g., `config/workspaces/campaigns_offers/exemplars/prepaid_apac_export.sql`) and schema documentation
- Implement a simple BM25 or embedding search to fetch the most relevant examples
- Use these examples to provide pattern guidance to the LLM

### 4. Build a Prompt Builder and SQL Renderer for Mixed-Initiative Generation

- Use the deterministic join scaffold from the Table Agent and Column Prune Agent to build a structured prompt for the LLM
- Include allowed tables, join predicates, derived field definitions, exemplar snippets and explicit instructions not to invent joins or columns
- Modify the SQL renderer to accept partial LLM outputs for the SELECT, WHERE, GROUP BY and ORDER BY clauses while still enforcing the deterministic FROM/JOIN block

### 5. Extend Validation & Policy Enforcement

- Enhance the static validator to enforce workspace boundaries, ensure that only allowed entities/columns appear in the SQL
- Verify that join predicates match the schema join graph and enforce version semantics (offer satellites vs. campaign links)
- Add policy rules for maximum row counts, allowable aggregates, and read-only access

### 6. Add Explainability and Telemetry

- Implement `explain.rs` to generate user-friendly summaries of the SQL: which tables are joined, why certain filters are applied, and how derived fields are computed
- Use this to help users understand and trust the system
- Integrate tracing and metrics via the telemetry module to log the workspace, tables chosen, exemplar retrieval hits and final SQL hash for debugging and monitoring

### 7. Complete Server & Execution Layer

- Expand the HTTP server to expose endpoints for generating report specs from prompts, returning diagnostics, executing SQL against a Postgres read-only connection and streaming results
- Implement the LISTEN/NOTIFY worker to refresh materialised views used by the compiler
- Add authentication and role-based access control

## Conclusion

The Genie QueryGPT repository shows that Phase A is indeed complete: there is a deterministic compiler and SQL renderer, snapshot tests lock down the pipeline, join normalisation and pagination support are implemented, and structured diagnostics provide clear error messages. The `Planner` trait exists but no AI logic is wired up, and the larger ecosystem of agents, retrieval and validation remains to be built. Phase B should focus on implementing a safe planning loop around the existing compiler, introducing intent classification, table selection, prompt construction and validation while strictly preserving the compiler's semantic authority.