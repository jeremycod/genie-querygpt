# Genie QueryGPT — Phases

This document defines the boundaries and guarantees of each development phase.

---

## Phase A — Deterministic Compiler (LOCKED)

### Goals
- Deterministic SQL generation
- Compiler-style validation
- No AI / LLM involvement

### Characteristics
- Human-authored `ReportSpec`
- Canonical `IntermediatePlan`
- Snapshot-tested SQL output
- Join normalization enforced by compiler
- LIMIT / OFFSET supported deterministically

### Explicitly Excluded
- LLMs
- SQL templating
- Runtime heuristics
- Renderer-side fixes

**Phase A is production-safe and locked.**

---

## Phase B — AI-Assisted Planning (FUTURE)

### Goals
- Improve usability and discovery
- Assist users in authoring `ReportSpec`

### Allowed AI Responsibilities
- Suggesting `ReportSpec`
- Suggesting `IntermediatePlan`
- Explaining compiler errors

### Forbidden
- AI-generated SQL
- AI-driven execution decisions

The compiler remains the single source of truth.

---

## Phase C — Interactive & Adaptive Querying (FUTURE)

### Possible Capabilities
- Conversational refinement
- Cost estimation and optimization hints
- Human-in-the-loop validation flows

### Non-Negotiable Rule
SQL generation remains deterministic and compiler-owned.

---

## Design Boundary Rule

Each phase may *add* capabilities, but never weaken:
- determinism
- auditability
- explainability
