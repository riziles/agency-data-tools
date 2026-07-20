# AGENTS.md — Notes for AI Coding Agents

## Project

Fannie Mae loan performance data ingestion pipeline:
- **Fetch**: OAuth2 → Fannie Mae API → signed S3 URL → ZIP download
- **Convert**: CSV (pipe-delimited, no headers, 108 CRT data fields) → Parquet
- **Store**: Cloudflare R2 (bucket: `fannie-mae-poc`)
- **Query**: DuckDB WASM or DataFusion WASM (planned)

## ⚠️ Critical: Do NOT Re-download Data

**Fannie Mae rate-limits downloads. Do not repeatedly download the same ZIP or CSV files.**

- The ingestion tool caches to `fannie_{year}_{quarter}.zip` and `fannie_{year}_{quarter}.csv` in the project root
- The tool checks for cached files before downloading — if the CSV already exists, it skips the entire fetch
- These cache files are gitignored (`*.zip`, `*.csv` in `.gitignore`)
- When testing schema/parsing changes, use the `--local-csv` flag to feed an already-extracted CSV:
  ```bash
  ./ingest/target/debug/fannie-ingest --local-csv fannie_2024_Q1.csv --output test-data/2024Q1.parquet
  ```
- The test data file `test-data/sample.csv` (5 rows) is safe for rapid iteration
- Each signed S3 URL is valid for ~1 hour

## Auth (Critical — Easy to Get Wrong)

The token endpoint in the RUST CODE is correct. Do NOT change it to `fmsso-prod.fanniemae.com`:

```
Token endpoint: https://auth.pingone.com/4c2b23f9-52b1-4f8f-aa1f-1d477590770c/as/token
API header:     x-public-access-token: {token}
API accepts:    Accept: application/json
```

This was the original bug that blocked the pipeline for a long time. The Python reference client from Fannie Mae's own repo confirms this endpoint.

## Schema (113 Columns)

The CSV is pipe-delimited with NO header row. The schema has 113 fields:

| Range | Count | Description |
|-------|-------|-------------|
| Field 1 | 1 | Leading empty (artifact of `|` delimiter at line start) |
| Fields 2–109 | 108 | Data fields per CRT File Layout (PDF positions 2–108 for SF data) |
| Field 110 | 1 | Extra field (value "7" in 2024 Q1 data — not in June 2023 PDF) |
| Fields 110–113 | 4 | Trailing empty (artifact of trailing pipes) |

**Field type notes:**
- Most numeric columns are `Float64` (not `Int32`) because empty values cause parse errors with `Int32`
- Coded indicator columns (e.g., `first_time_home_buyer_indicator`) contain values like "Y", "N", "7", "9" — not booleans
- Credit scores are `Int32` and CAN be empty (single-borrower loans have no co-borrower score)

**Reference:** [CRT File Layout and Glossary PDF](http://capitalmarkets.fanniemae.com/sites/capmrkt/files/2023-06/crt-file-layout-and-glossary.pdf) (108 positions; note position 1 "Reference Pool ID" is NA for SF data)

## Build & Run

```bash
# Build (from project root)
cd ingest && cargo build

# Fetch + convert one quarter (~4 min for 94 MB ZIP → 1.28 GB CSV → 39 MB Parquet)
./ingest/target/debug/fannie-ingest --year 2024 --quarter Q1 --output test-data/2024Q1.parquet

# Convert local CSV (use when iterating on schema)
./ingest/target/debug/fannie-ingest --local-csv fannie_2024_Q1.csv --output test-data/2024Q1.parquet

# Upload to R2 (needs R2_ACCESS_KEY_ID and R2_SECRET_ACCESS_KEY in .env)
./ingest/target/debug/fannie-ingest --year 2024 --quarter Q1 \
  --output test-data/2024Q1.parquet \
  --r2-bucket fannie-mae-poc --r2-key 2024/Q1/loans.parquet
```

## Environment

- `.env` contains credentials (gitignored) — includes `FANNIE_CLIENT_ID`, `FANNIE_CLIENT_SECRET`, `R2_ACCOUNT_ID`
- `secrets/datadynamics.yaml` is the canonical credential store (even more gitignored)
- `scripts/setup-env.sh` reads secrets YAML → `.env`

## Key Files

| File | Purpose |
|------|---------|
| `ingest/src/main.rs` | Full pipeline: schema, API auth, ZIP extract, CSV→Parquet, R2 upload |
| `ingest/Cargo.toml` | Rust deps: datafusion, reqwest, object_store, zip, clap |
| `test-data/sample.csv` | 5-row test file (safe for rapid iteration) |
| `ARCHITECTURE.md` | Architecture analysis (R2 costs, compute options, query layer comparison) |

## Fannie Mae API

| Item | Value |
|------|-------|
| Base URL | `https://api.fanniemae.com` |
| App name | `datafusion01` |
| API product | `SingleFamilyLphExchangeAPI` |
| Python ref client | [fnmapublic-python-clients](https://github.com/Developer-Portal-Fannie-Mae/fnmapublic-python-clients) |
| Data page | [capitalmarkets.fanniemae.com](https://capitalmarkets.fanniemae.com/credit-risk-transfer/single-family-credit-risk-transfer/fannie-mae-single-family-loan-performance-data) |

### Endpoints

- `GET /v1/sf-loan-performance-data/years/{year}/quarters/{quarter}` — returns `{ lphResponse: [{ s3Uri, year, quarter }] }`
- The `s3Uri` is a signed AWS S3 URL (valid ~1h) pointing to a ZIP containing a pipe-delimited CSV
- Coda pack reference: [ramiisaac/coda-pack-fannie-mae-developer-api](https://github.com/ramiisaac/coda-pack-fannie-mae-developer-api) (uses same auth flow but for different API products)

## Performance Notes

- 2024 Q1: 94 MB ZIP → 1.28 GB CSV → **39 MB Parquet** (33:1 compression)
- 3,989,404 rows in 2024 Q1
- Download ~10s, CSV extraction ~15s, Parquet conversion ~90s (debug build)
- Full dataset is hundreds of GB — each quarter is ~1-1.5 GB uncompressed CSV
- The 1.28 GB CSV stays on disk (not loaded entirely into memory) — DataFusion streams it
- Debug builds are slower; `--release` builds will be significantly faster for large batches
