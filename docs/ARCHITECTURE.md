# Architecture Overview — Genie QueryGPT

This repository implements **Genie QueryGPT**, a compiler-style, deterministic SQL generation system.

The architecture is deliberately split into phases to ensure:
- determinism
- explainability
- safety before introducing AI assistance

## Canonical Pipeline

```
ReportSpec (JSON / YAML / Rust)
        ↓
compile_report_spec
        ↓
IntermediatePlan (validated, canonical)
        ↓
render_sql
        ↓
Deterministic SQL
```

## Key Principles

- **Compiler owns semantics**  
  All validation, normalization, and error handling happens during compilation.

- **Renderer is pure**  
  The SQL renderer never fixes or guesses intent. It only serializes a valid plan.

- **Determinism is mandatory**  
  Identical inputs always produce identical SQL.

- **Snapshots are contracts**  
  Snapshot tests define backwards compatibility and behavior guarantees.

## Phase A Status

Phase A is complete and locked.

See:
- `crates/querygpt-core/README.md` — detailed Phase A design
- `PHASES.md` — roadmap for future phases
