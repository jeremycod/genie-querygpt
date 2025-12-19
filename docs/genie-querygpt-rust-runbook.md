# Genie QueryGPT (Rust) — Runbook

This runbook explains how to **run the repo skeleton** you downloaded (`genie-querygpt-rust-repo-skeleton.zip`).
It includes:
- prerequisites
- how to build + run the API server
- how to run the MV refresh worker
- how to test the `/generate` endpoint
- common troubleshooting

> Note: This repo is a **skeleton** — the `/generate` endpoint currently returns placeholder SQL/explanations.
> The goal is to give you a clean structure and config layout to implement the full pipeline.

---

## 1) Prerequisites

### Rust toolchain
Install Rust (stable) using rustup:

- macOS/Linux:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

Verify:
```bash
rustc --version
cargo --version
```

### Postgres (optional for the worker)
The **worker** connects to Postgres and issues:
- `LISTEN` on `*_changed` channels
- `REFRESH MATERIALIZED VIEW CONCURRENTLY <mv>;`

So you need:
- a reachable Postgres instance (local or remote)
- the `*_latest` materialized views created in that DB (or stub out the refresh calls)

---

## 2) Unzip the project

```bash
unzip genie-querygpt-rust-repo-skeleton.zip -d genie-querygpt
cd genie-querygpt
```

You should see:
- `Cargo.toml` (workspace)
- `crates/querygpt-core`
- `crates/querygpt-server`
- `crates/querygpt-worker`
- `config/workspaces/...`

---

## 3) Build everything

From the repo root:

```bash
cargo build
```

If you want a faster dev loop:
```bash
cargo build -q
```

---

## 4) Run the API server

The server is an **Axum** app that exposes a stub endpoint:

- `POST /generate`

Start it:

```bash
cargo run -p querygpt-server
```

By default it binds to:
- `0.0.0.0:8080`

### Test the endpoint

In another terminal:

```bash
curl -X POST http://localhost:8080/generate   -H 'content-type: application/json'   -d '{"user_prompt":"Export campaigns with prepaid offers in APAC"}'
```

Expected output (stubbed):
```json
{
  "workspace": "campaigns_offers",
  "sql": "-- SQL generation pipeline TBD",
  "explanation": "-- Explanation TBD"
}
```

---

## 5) Run the LISTEN/NOTIFY MV refresh worker (optional)

### 5.1 Configure environment variables

Copy the example environment file and edit as needed:

```bash
cp .env.example .env
```

Edit `.env` to set your database connection and other settings:
```
DATABASE_URL=postgres://USER:PASSWORD@HOST:5432/DBNAME
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
REFRESH_INTERVAL_SECONDS=60
```

### 5.2 Run the worker

```bash
cargo run -p querygpt-worker
```

It will:
- `LISTEN offers_changed`
- `LISTEN campaigns_changed`
- `LISTEN products_changed`
- `LISTEN discounts_changed`
- `LISTEN skus_changed`
- debounce notifications (~15s)
- refresh all MVs (simple strategy in skeleton)

> If your DB doesn’t have these MVs yet, the worker will error on refresh.
> Either create them first or comment out refresh statements in `crates/querygpt-worker/src/main.rs`.

---

## 6) Create the Latest Materialized Views (DB-side)

If you want the worker and “latest semantics” to work end-to-end, create the MVs in your database.
Use the script we generated earlier that keeps deleted rows as latest.

Typical steps:
1. connect to your database
2. run the `CREATE MATERIALIZED VIEW ...` statements for:
   - `offers_latest`
   - `campaigns_latest`
   - `products_latest`
   - `discounts_latest`
   - `skus_latest`
3. ensure unique indexes exist for `REFRESH ... CONCURRENTLY`:
   - `(id, profile)` on each MV

Then you can manually refresh once:
```sql
REFRESH MATERIALIZED VIEW CONCURRENTLY offers_latest;
REFRESH MATERIALIZED VIEW CONCURRENTLY campaigns_latest;
REFRESH MATERIALIZED VIEW CONCURRENTLY products_latest;
REFRESH MATERIALIZED VIEW CONCURRENTLY discounts_latest;
REFRESH MATERIALIZED VIEW CONCURRENTLY skus_latest;
```

---

## 7) Configuration layout

### Workspace index
`config/workspaces/campaigns_offers.index.json`

This tells the system where to find:
- schema cards JSON
- exemplar SQL directory

### Schema cards
`config/workspaces/campaigns_offers.schema_cards.json`

Includes:
- entity cards (columns, JSON paths, filter hints)
- join graph (safe, version-correct edges)
- derived fields

### Exemplars
`config/workspaces/campaigns_offers/exemplars/*.sql`

These are used by your future RAG retrieval step.

---

## 8) Common issues & fixes

### “address already in use” on 8080
Another process is using the port.

Fix:
- stop the other process, or
- change the bind address in:
  `crates/querygpt-server/src/main.rs`

### Worker fails: “relation offers_latest does not exist”
The DB doesn’t have the MVs yet.

Fix:
- create the MVs in Postgres (Section 6), or
- temporarily comment the refresh loop in the worker.

### Worker never refreshes
No notifications are being emitted.

Fix:
- add NOTIFY triggers in the DB (we generated a trigger script earlier), or
- use `pg_cron` refresh jobs instead

---

## 9) Next implementation steps (recommended order)

1. Implement **SchemaRegistry** loading for all workspaces.
2. Implement **Intent Agent** (rules + embedding search).
3. Implement **Table Agent** using Join Graph safety checks.
4. Implement **Column Prune Agent**.
5. Add **RAG retrieval** from exemplar SQL.
6. Implement **SQL generation** (deterministic plan + LLM fill-in).
7. Add **static validation** using pg_catalog metadata.
8. Add **execution** (read-only) + **explanation**.

---

## 10) Quick commands reference

```bash
# build all crates
cargo build

# run server
cargo run -p querygpt-server

# run worker
export DATABASE_URL="postgres://..."
cargo run -p querygpt-worker

# test API
curl -X POST http://localhost:8080/generate   -H 'content-type: application/json'   -d '{"user_prompt":"Export campaigns with prepaid offers in APAC"}'
```

---

If you want, I can generate:
- a `docker-compose.yml` (Postgres + server + worker),
- migrations/scripts wired into `make` tasks,
- a real `/generate` pipeline that reads Schema Cards and emits actual SQL.
