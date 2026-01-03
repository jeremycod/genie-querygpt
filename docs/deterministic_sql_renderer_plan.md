## Implementation Plan: Deterministic SQL Renderer

The goal of this task is to write a **deterministic SQL renderer** that takes an
`IntermediatePlan` (produced by the compiler) and emits a fully‐formed SQL
query.  The renderer must not consult the LLM or any external state; it
operates purely on the data contained in the plan.  The generated SQL should
always be the same for the same plan (deterministic ordering), it must use
correct join semantics and grouping, and it should be covered by snapshot
tests.

### 1. Understand the `IntermediatePlan` structure

Before writing the renderer, make sure you are comfortable with the structure
defined in `crates/querygpt-core/src/dsl/plan.rs`:

- `workspace: String` – the name of the workspace for context
- `tables: Vec<PlanTable>` – each entry has a `name` (entity name) and `alias`
- `joins: Vec<PlanJoin>` – each join has `left_alias`, `right_alias`, a
  `join_type` (inner or left), and a list of equality predicates (`JoinCondition`)
- `projections: Vec<PlanProjection>` – each has a report `field`, its SQL
  `expression`, and an optional alias
- `filters: Vec<PlanFilter>` – each holds a single SQL expression
- `order_by: Vec<PlanOrder>` – each has the SQL expression and sort direction

The renderer should only read from these fields; it must not re‐resolve
entities or apply any further schema logic.

### 2. Decide a deterministic table and join order

To produce a stable `FROM ... JOIN ...` section, choose a reproducible
ordering for the tables and joins:

1. **Base table selection**:  pick the first entry in
   `IntermediatePlan.tables` as the base table.  Because the tables are
   derived from a `BTreeSet` in the compiler, their order is already
   deterministic (alphabetical by entity name), which should be preserved
   here.  Use the `alias` field when emitting SQL (e.g. `offers_latest AS o`).
2. **Join ordering**:  iterate over `IntermediatePlan.joins` in the order
   provided by the compiler (it builds joins by iterating through the join
   graph deterministically).  For each join, emit either `JOIN` or
   `LEFT JOIN` according to the `join_type`, and use the aliases in
   `left_alias`/`right_alias` for the join condition.  All equality
   predicates within a join must be combined with `AND`.

The combination of sorted tables and compiler‐ordered joins guarantees that
two equivalent plans always produce the same FROM/JOIN clause.

### 3. Compose the SELECT clause

Walk through `IntermediatePlan.projections` in order.  For each projection:

1. Use the `expression` property as the literal SQL to select.  If
   `alias` is present, append `AS {alias}` so that the column appears with the
   user‐defined name.  Otherwise, use the expression as is.
2. Join all projection strings with commas and newlines for readability.  For
   example:

   ```sql
   SELECT
       o.id AS offer_id,
       c.id AS campaign_id,
       STRING_AGG(DISTINCT opr.product_id, ',') AS products
   ```

### 4. Compose the WHERE clause

If `IntermediatePlan.filters` is non‐empty, emit a `WHERE` clause.  Combine
all filter expressions using `AND`, preserving their order in the plan.  If
there are no filters, omit the `WHERE` clause entirely.

### 5. Compose the GROUP BY clause

The renderer is responsible for grouping correctly when aggregates appear in
projections.  A simple and deterministic strategy is:

1. **Detect aggregated expressions**:  examine each projection’s
   `expression`.  If it contains a known aggregate keyword
   (e.g. `STRING_AGG`, `COUNT`, `SUM`, `MIN`, `MAX`, `AVG`) or the derived
   field definition uses an aggregate, treat it as an aggregate.  These
   expressions should **not** appear in the GROUP BY list.
2. **Collect grouping expressions**:  for every projection that is
   *not* an aggregate, take its SQL expression up to the first `AS`.  Use
   these expressions verbatim in the `GROUP BY` clause.  For example,
   `o.id AS offer_id` contributes `o.id` to the group list.  If multiple
   projections reference the same underlying column, they should only appear
   once.
3. **Emit GROUP BY only when needed**:  if the plan contains at least one
   aggregate expression *and* at least one non‐aggregate projection, emit
   `GROUP BY` followed by a comma‐separated list of unique grouping
   expressions in the order of appearance.  Otherwise, omit the clause.

This strategy ensures correct grouping for the current use case
(`STRING_AGG` for product IDs) and can be extended later if more complex
aggregations are introduced.

### 6. Compose the ORDER BY clause

If `IntermediatePlan.order_by` is non‐empty, emit an `ORDER BY` clause.  For
each entry:

1. Use the `expression` directly (it was derived from projections or
   user requests).  Append `ASC` or `DESC` based on the `direction` field.
2. Join the ordering expressions with commas and spaces.  Maintain the
   ordering defined by the plan.

### 7. Assemble the final SQL string

Combine the pieces with line breaks for readability.  The general
structure will be:

```sql
SELECT
    <projection_1>,
    <projection_2>,
    ...
FROM
    <base_table> AS <base_alias>
    <join_type> JOIN <table> AS <alias>
        ON <condition_1> AND <condition_2>
    ...
<WHERE clause>
<GROUP BY clause>
<ORDER BY clause>;
```

Ensure that there are no trailing commas and that each clause appears only
when needed.  Terminate the query with a semicolon for SQL consistency.

### 8. Write snapshot tests

Create snapshot tests for the renderer analogous to those used for
`compile_report_spec`.  For example, take the existing prepaid APAC example
plan (obtained by compiling the `campaigns_offers_prepaid_apac.json` spec) and
assert that the rendered SQL matches the expected string.  You can use the
`insta` crate in Rust to manage snapshots.  A test might look like this:

```rust
#[test]
fn render_prepaid_apac_sql() {
    let registry = SchemaRegistry::load("config/workspaces/campaigns_offers.index.json").unwrap();
    let spec = load_spec_fixture("campaigns_offers_prepaid_apac.json");
    let plan = compile_report_spec(&registry, &spec).unwrap();
    let sql = render_sql(&plan);
    insta::assert_snapshot!(sql);
}
```

Snapshots make it easy to update the expected output if the renderer changes,
while ensuring that new changes are deliberate.

### 9. Additional considerations

1. **SQL injection safety**:  Since both the compiler and renderer work with
   trusted schema metadata and validated report specs, SQL injection is not a
   concern here.
2. **Future aggregates**:  If new derived fields introduce other aggregates
   (e.g. `COUNT(DISTINCT ...)`), extend the aggregate detection logic.
3. **JOIN ordering for complex plans**:  If future workspaces introduce
   multiple valid join paths, consider ordering joins by dependency
   (a topological sort of the join graph) to ensure the ON conditions always
   reference tables that have already been introduced.

With this plan implemented, your deterministic SQL renderer will produce
reliable, predictable SQL statements from any `IntermediatePlan` and allow
snapshotted unit tests to verify their correctness.