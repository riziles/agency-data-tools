# AGENTS.md — Notes for AI Coding Agents

## ⚠️ Secrets

- **Do NOT read secrets files into the chat.** Use `dotenvy` in Rust or source `.env` in bash to access credentials without exposing them.
- `.env` and `secrets/` contain real Fannie Mae API credentials — never display, log, or commit these values
- If you need to test an API call, use `source .env && curl ... -u "${FANNIE_CLIENT_ID}:${FANNIE_CLIENT_SECRET}"` — the env vars stay in the shell, not the chat
- The `scripts/setup-env.sh` script reads `secrets/datadynamics.yaml` and writes `.env` — use that instead of manually copying secrets

## Project

Fannie Mae loan performance data pipeline:
- **Fetch**: OAuth2 → Fannie Mae API → signed S3 URL → ZIP download
- **Convert**: CSV (pipe-delimited, no headers, 113 columns) → Parquet (DF 54)
- **Load**: Streaming `add-quarter` → DuckLake catalog (SQLite metadata + Parquet data)
- **Query**: Flight SQL server (DF 54 + DuckLake 0.5) → browser via gRPC-web + Node proxy
- **Deploy**: Docker container + optional Cloudflare Tunnel

**Status**: 32 quarters loaded (2018 Q1 – 2025 Q4), ~751M rows, 16 GB DuckLake catalog.

## ⚠️ Critical: Do NOT Re-download Data

**Fannie Mae rate-limits downloads. Do not repeatedly download the same ZIP or CSV files.**

- The ingestion tool caches to `fannie_{year}_{quarter}.zip` and `fannie_{year}_{quarter}.csv` in the project root
- These cache files are gitignored (`*.zip`, `*.csv` in `.gitignore`)
- When testing schema/parsing changes, use the `--local-csv` flag:
  ```bash
  ./ingest/target/release/fannie-ingest --local-csv fannie_2024_Q1.csv --output test-data/2024Q1.parquet
  ```
- Each signed S3 URL is valid for ~1 hour
- **Wait 2 minutes between API calls** to avoid hitting rate limits

## ⚠️ Do NOT run add-quarter while flight-server is running

Concurrent writes to the DuckLake SQLite catalog corrupt data. A file lock (`data/.lock`) enforces this, but it's a safety net — don't rely on it.

## Auth (Critical — Easy to Get Wrong)

```
Token endpoint: https://auth.pingone.com/4c2b23f9-52b1-4f8f-aa1f-1d477590770c/as/token
API header:     x-public-access-token: {token}
API accepts:    Accept: application/json
```

Do NOT change the token endpoint to `fmsso-prod.fanniemae.com`.

## Schema (113 Columns)

The CSV is pipe-delimited with NO header row. 113 fields (1 leading empty + 108 data + 1 extra + 3 trailing empty).

**Field type notes:**
- Most numeric columns are `Float64` (not `Int32`) — empty values cause parse errors with Int32
- Coded indicators (e.g., `first_time_home_buyer_indicator`) are strings "Y"/"N"/"7"/"9" — not booleans
- Credit scores are `Int32` and CAN be empty (single-borrower loans)

**Reference:** [CRT File Layout and Glossary PDF](http://capitalmarkets.fanniemae.com/sites/capmrkt/files/2023-06/crt-file-layout-and-glossary.pdf)

## Two Branches

| Branch | What | Status |
|--------|------|--------|
| `flight-sql-server` | Full pipeline: DuckLake + Flight SQL + Docker | **Active** (this branch) |
| `wasm-query` | DataFusion WASM demo querying R2 | Demo only |

## Build & Run

### Ingest a quarter

```bash
cd ingest && cargo build --release

# Download + convert
source ../.env
./target/release/fannie-ingest --year 2024 --quarter Q1 --output ../test-data/2024Q1.parquet

# Or convert existing CSV (no API call)
./target/release/fannie-ingest --local-csv ../fannie_2024_Q1.csv --output ../test-data/2024Q1.parquet
```

### Load into DuckLake

```bash
cd flight-server && cargo build --release --bin flight-sql-server --bin add-quarter

# Streaming load (500k-row chunks, up to 4GB memory)
cp ../test-data/2024Q1.parquet data/
./target/release/add-quarter --data-dir data --parquet data/2024Q1.parquet --max-memory-mb 4000
rm data/2024Q1.parquet
```

### Start query server

```bash
# Host
./target/release/flight-sql-server --data-dir data --parquet data/seed.parquet &
cd public && APP_PASSWORD=demo node server.mjs &
# → http://localhost:8765

# Or Docker
docker run -d -p 8765:8765 -v ./data:/app/data -e APP_PASSWORD=demo fannie-flight
```

## Key Files

| File | Purpose |
|------|---------|
| `ingest/src/main.rs` | API auth, ZIP extract, CSV→Parquet (DF 54) |
| `ingest/Cargo.toml` | datafusion 54, reqwest, zip, clap |
| `flight-server/src/main.rs` | Flight SQL server + DuckLake catalog |
| `flight-server/src/lock.rs` | File locking (flock) — prevents concurrent writes |
| `flight-server/src/bin/add_quarter.rs` | Streaming Parquet→DuckLake ingest |
| `flight-server/public/server.mjs` | Node proxy: password gate, gRPC-web, static files |
| `flight-server/public/index.html` | Browser query UI (dark theme) |
| `flight-server/public/src/app.js` | @sparrowflight/js Flight SQL client |
| `flight-server/Dockerfile` | Docker image (804MB, data bind-mounted) |
| `flight-server/data/` | DuckLake catalog: catalog.db + datalake/*.parquet |
| `test-data/*.parquet` | Source Parquet backups (~7 GB total) |
| `fannie_*.zip` | Downloaded ZIP caches (~17 GB, gitignored) |

## Fannie Mae API

| Item | Value |
|------|-------|
| Base URL | `https://api.fanniemae.com` |
| Endpoint | `GET /v1/sf-loan-performance-data/years/{year}/quarters/{quarter}` |
| Response | `{ lphResponse: [{ s3Uri, year, quarter }] }` |
| S3 URL valid | ~1 hour |
| App name | `datafusion01` |
| API product | `SingleFamilyLphExchangeAPI` |

## Performance Notes

- 751M rows across 32 quarters, 16 GB DuckLake catalog
- Streaming ingest: ~860 MB memory per 500k-row chunk, deletes CSV after conversion
- Query: ~500ms for `count(*)`, ~80ms for filtered queries
- Parquet compression: ~33:1 (1.8 GB CSV → 55 MB Parquet for large quarters)
- Largest quarter: 2021 Q1 (73.5M rows, 1.66 GB ZIP)
- Docker image: 804 MB (no data baked in)
- Data release lag: ~6 months (2025 Q4 is latest as of July 2026)
