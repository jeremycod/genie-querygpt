# Genie QueryGPT — Phase B Implementation Plan

## Purpose of This Document

This document is a **precise, implementation-ready specification** for Phase B of Genie QueryGPT. It is intended to be handed directly to an **Amazon Q agent or other coding agent**.

**Key requirement:** Phase A is complete and locked. Phase B MUST NOT weaken, bypass, or reinterpret any Phase A guarantees.

---

## Phase B Objective

Enable **AI-assisted authoring of ReportSpec** while preserving:

- Compiler-owned semantics
- Deterministic output
- Snapshot stability
- Renderer purity

Phase B introduces AI **only as a suggestion layer**, never as an execution or semantic authority.

---

## Non‑Negotiable Constraints (Global)

These rules apply to all Phase B work:

❌ LLMs must NOT generate SQL
❌ LLMs must NOT modify IntermediatePlan semantics
❌ Renderer must NOT be changed
❌ Compiler must NOT be bypassed
❌ No silent corrections or heuristics

✅ Compiler remains the single source of truth
✅ All AI output is treated as untrusted input
✅ Every AI-generated artifact must be recompiled

Violating any of the above invalidates Phase B.

---

## Existing Phase A Architecture (Context)

```
ReportSpec (JSON / YAML / Rust)
        ↓
compile_report_spec   ← semantic authority
        ↓
IntermediatePlan      ← canonical, validated
        ↓
render_sql             ← pure serializer
        ↓
Deterministic SQL
```

Phase B layers **around** this pipeline — never inside it.

---

## Phase B High‑Level Architecture

```
User Intent (Natural Language)
        ↓
Planner (LLM, suggestion-only)
        ↓
Draft ReportSpec
        ↓
compile_report_spec   ← authoritative gate
        ↓
IntermediatePlan
        ↓
render_sql (unchanged)
```

The compiler is always invoked before execution or rendering.

---

## Core Phase B Components

### 1. Planner Boundary (Mandatory)

The planner is a **strict interface** that isolates AI behavior.

#### Planner Trait

```rust
pub trait Planner {
    fn suggest_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
    ) -> PlannerResult<ReportSpecDraft>;

    fn revise_report_spec(
        &self,
        prompt: &str,
        ctx: PlannerContext,
        diagnostics: &CompilerDiagnostics,
    ) -> PlannerResult<ReportSpecDraft>;
}
```

#### Key Rules

- Planner returns **drafts only**, never executable artifacts
- Planner output MUST go through compiler
- Planner does NOT access renderer or SQL

---

### 2. Planner Output Model

```rust
pub struct ReportSpecDraft {
    pub spec: ReportSpec,
    pub rationale: Option<String>,
    pub assumptions: Vec<String>,
}
```

Notes:
- `rationale` and `assumptions` are informational only
- They are never consumed by the compiler

---

### 3. Structured Compiler Diagnostics (Critical)

Compiler errors must be **machine-readable, stable, and snapshot-tested**.

#### Diagnostic Types

```rust
pub struct CompilerDiagnostics {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub spans: Vec<Span>,
    pub details: serde_json::Value,
    pub help: Vec<String>,
}

pub enum DiagnosticCode {
    UnknownField,
    UnknownTable,
    InvalidJoin,
    AmbiguousJoin,
    InvalidPagination,
    SchemaMismatch,
    InvalidFilterValue,
}

pub struct Span {
    pub pointer: String, // JSON Pointer into ReportSpec
}
```

#### Requirements

- `DiagnosticCode` must be stable and versioned
- `details` must contain structured metadata (aliases, field names, etc.)
- Diagnostics must be snapshot-tested

---

### 4. Compiler → Planner Feedback Loop

A **bounded retry loop** coordinates AI assistance safely.

#### Algorithm

1. Planner generates `ReportSpecDraft`
2. Compiler attempts `compile_report_spec`
3. If compilation succeeds:
   - Proceed to rendering / explanation
4. If compilation fails:
   - Return `CompilerDiagnostics`
   - Planner may revise spec
5. Retry limit enforced (e.g. max 3 attempts)
6. User confirmation required before final acceptance

#### Safety Rules

- Compiler is always executed
- Retry loop must terminate
- No automatic acceptance of AI revisions

---

### 5. Diff & Confirmation Layer

Before execution:

- Show structured diff between original and revised ReportSpec
- Require explicit user confirmation
- Recompile after confirmation

This prevents silent semantic drift.

---

### 6. Explainability (Read‑Only)

Optional utilities that **observe** compiler output.

#### Explain Functions

```rust
fn explain_spec(spec: &ReportSpec) -> Explanation;
fn explain_plan(plan: &IntermediatePlan) -> Explanation;
```

```rust
pub struct Explanation {
    pub summary: String,
    pub joins: Vec<String>,
    pub filters: Vec<String>,
    pub pagination: Option<String>,
}
```

Rules:
- No mutation of spec or plan
- No influence on compilation or rendering

---

## Implementation Order (Strict)

### Step 1 — Planner Interface + NoopPlanner

- Add `planner` module
- Implement `NoopPlanner`
- Wire compile-only flow

**Acceptance Criteria:** Phase A tests unchanged

---

### Step 2 — Structured Diagnostics

- Replace ad-hoc compiler errors
- Add diagnostics snapshots
- Ensure stable error codes

**This step is mandatory before AI integration**

---

### Step 3 — Retry Orchestration (No LLM)

- Implement feedback loop using `NoopPlanner`
- Add diff display
- Enforce retry limit

---

### Step 4 — LLM Planner Integration

- Implement `Planner` using LLM
- Treat output as untrusted
- Require confirmation
- Always recompile

---

## Explicit Non‑Goals (Do NOT Implement)

❌ AI SQL generation
❌ AI plan execution
❌ Renderer heuristics
❌ Schema inference at runtime
❌ Automatic fixes without compiler approval

---

## Recommended Module Layout

```
querygpt-core/
  compiler/
    diagnostics.rs
  planner/
    trait.rs
    noop.rs
    llm.rs
  explain/
    explain_plan.rs
    explain_spec.rs
```

---

## Phase B Success Criteria

- Natural language → valid ReportSpec
- Invalid specs rejected deterministically
- SQL output remains unchanged and snapshot-safe
- Phase A invariants remain intact

---

## Final Instruction to Amazon Q Agent

**Implement Phase B strictly as defined in this document.**

- Do not modify compiler semantics
- Do not modify renderer behavior
- Treat AI output as untrusted
- Preserve determinism and snapshots

If any step conflicts with Phase A invariants, **do not implement it**.

