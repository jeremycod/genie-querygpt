# Genie QueryGPT (Rust)

A QueryGPT-style text-to-SQL reporting system for Genie DB.

## What’s inside
- **querygpt-core**: schema registry, DSL, join graph, agents, validation
- **querygpt-server**: HTTP API (Axum) for generating/explaining SQL and running reports
- **querygpt-worker**: LISTEN/NOTIFY debounced refresher for *_latest materialized views

## Quick start
```bash
cargo build
cargo run -p querygpt-server
```

POST:
```bash
curl -X POST http://localhost:8080/generate   -H 'content-type: application/json'   -d '{"user_prompt":"Export campaigns with prepaid offers in APAC"}'
```

## Config
- Workspace index: `config/workspaces/*.index.json`
- Schema Cards: `config/workspaces/*.schema_cards.json`
- Exemplars: `config/workspaces/<ws>/exemplars/*.sql`
