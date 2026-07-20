# AGENTS.md — Notes for AI Coding Agents

## ⚠️ Secrets

- **Do NOT read secrets files into the chat.** Use `dotenvy` in Rust or source `.env` in bash to access credentials without exposing them.
- `.env` and `secrets/` contain real Fannie Mae API credentials — never display, log, or commit these values
- If you need to test an API call, use `source .env && curl ... -u "${FANNIE_CLIENT_ID}:${FANNIE_CLIENT_SECRET}"` — the env vars stay in the shell, not the chat
- The `scripts/setup-env.sh` script reads `secrets/datadynamics.yaml` and writes `.env` — use that instead of manually copying secrets

## Project

Fannie Mae loan performance data ingestion and analytics pipeline:
- **Fetch**: OAuth2 → Fannie Mae API → signed S3 URL → ZIP download
- **Convert**: CSV (pipe-delimited, no headers, 108 CRT data fields) → Parquet
- **Store**: Cloudflare R2 (bucket: `fannie-mae-poc`)
- **Query**: Browser-based DataFusion WASM app (wren-core-wasm)
- **Deploy**: GitHub Pages via Actions (`.github/workflows/deploy.yml`)
- **Live site**: https://riziles.github.io/agency-data-tools/

## Query Layer

- **`query/`** — Browser-based DataFusion WASM query app
  - `query/index.html` — Self-contained app (dark theme, SQL editor, results table, progress bar)
  - `query/server.cjs` — Node.js dev server with CORS + COOP/COEP headers (run locally: `node query/server.cjs`)
  - **Inline mode**: Downloads 38.8 MB Parquet from R2 on load (~7s), then queries are instant (<1s)
  - Imports `@wrenai/wren-core-wasm` from CDN (DataFusion WASM via wren-core)
  - 28 useful columns exposed in MDL (loan_id, credit scores, UPB, interest rates, LTV, DTI, property state, etc.)
  - **URL mode is broken** — `ListingTable` parallel scans trigger `condvar wait not supported` crash in WASM
  - **Fix branch**: `/home/seanan/Documents/repos/wrenai-wasm-fix-fork-maybe` on `fix/wren-core-wasm-condvar-url-mode`
    - Adds `datafusion.execution.parquet.maximum_parallel_row_groups = 1` to `WrenEngine::new()`
    - See `core/wren-core-wasm/FIX_CONDVAR_URL_MODE.md` in that repo for details

## ⚠️ Critical: Do NOT Re-download Data

**Fannie Mae rate-limits downloads. Do not repeatedly download the same ZIP or CSV files.**

- The ingestion tool caches to `fannie_{year}_{quarter}.zip` and `fannie_{year}_{quarter}.csv` in the project root
- The tool checks for cached files before downloading — if the CSV already exists, it skips the entire fetch
- These cache files are gitignored (`*.zip`, `*.csv` in `.gitignore`)
- When testing schema/parsing changes, use the `--local-csv` flag to feed an already-extracted CSV
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

## R2 Upload

- **Use wrangler, not the Rust S3 API.** The direct S3 endpoint (`account-id.r2.cloudflarestorage.com`) may have connectivity issues. Wrangler's API-based upload works reliably:
  ```bash
  pnpm wrangler r2 object put fannie-mae-poc/2024/Q1/loans.parquet --file test-data/2024Q1.parquet
  ```
- The `--r2-bucket` flag on the ingest tool is available but may time out depending on network conditions
- Bucket: `fannie-mae-poc`
- Public dev URL: `https://pub-a0dfcedd4df848e5bfca15cca210aea5.r2.dev`

## Build & Run

```bash
# Build (from project root)
cd ingest && cargo build

# Fetch + convert one quarter (~4 min for 94 MB ZIP → 1.28 GB CSV → 39 MB Parquet)
./ingest/target/debug/fannie-ingest --year 2024 --quarter Q1 --output test-data/2024Q1.parquet

# Convert local CSV (use when iterating on schema)
./ingest/target/debug/fannie-ingest --local-csv fannie_2024_Q1.csv --output test-data/2024Q1.parquet

# Upload to R2
pnpm wrangler r2 object put fannie-mae-poc/2024/Q1/loans.parquet --file test-data/2024Q1.parquet

# Run query app locally
node query/server.cjs
# Open http://localhost:8765
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
| `query/index.html` | Browser query app (DataFusion WASM, dark theme) |
| `query/server.cjs` | Dev server (Node.js, CORS + COOP/COEP) |
| `test-data/sample.csv` | 5-row test file (safe for rapid iteration) |
| `ARCHITECTURE.md` | Architecture analysis (R2 costs, compute options, query layer comparison) |
| `.github/workflows/deploy.yml` | GH Pages deploy action |

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
- Query app: ~7s initial R2 download + parquet parse, then <1s per query

## Schema (113 Columns)

The CSV is pipe-delimited with NO header row. The schema has 113 fields:

| Range | Count | Description |
|-------|-------|-------------|
| Field 1 | 1 | Leading empty (artifact of `|` delimiter at line start) |
| Fields 2–109 | 108 | Data fields per CRT File Layout (PDF positions 2–108 for SF data) |
| Field 110 | 1 | Extra field (value "7" in 2024 Q1 data — not in June 2023 PDF) |
| Fields 110–113 | 4 | Trailing empty (artifact of trailing pipes) |

**Reference:** [CRT File Layout and Glossary PDF](http://capitalmarkets.fanniemae.com/sites/capmrkt/files/2023-06/crt-file-layout-and-glossary.pdf)
