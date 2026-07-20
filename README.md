# Agency Data Tools

Fannie Mae loan performance data ingestion and analytics pipeline.

## Status

**Pipeline working end-to-end** 🎉 — Fetches real data from Fannie Mae API, extracts ZIP, converts CSV → Parquet. 2024 Q1 = 4M rows, 1.28 GB CSV → 39 MB Parquet (33:1 compression).

### What Works
- ✅ Cloudflare R2 bucket `fannie-mae-poc` created
- ✅ Rust ingestion tool compiles (DataFusion CSV→Parquet conversion)
- ✅ CSV→Parquet on test data works (pipe-delimited, no headers)
- ✅ Wrangler CLI authenticated to Cloudflare
- ✅ Fannie Mae API auth: `auth.pingone.com` → `x-public-access-token` header → 200 OK
- ✅ Full pipeline: API → signed S3 URL → ZIP download → CSV extract → Parquet
- ✅ 2024 Q1: 3,989,404 rows, 94 MB ZIP → 39 MB Parquet
- ✅ Cloudflare skills installed (`.pi/skills/`)
- ✅ Git repo pushed to GitHub

### Resolved Auth Issue
- The bug: code was hitting `fmsso-prod.fanniemae.com` for tokens (doesn't resolve)
- The fix: use `auth.pingone.com/4c2b23f9-52b1-4f8f-aa1f-1d477590770c/as/token` (matches Fannie Mae's own Python reference client)
- Auth header: `x-public-access-token: {token}` (not `Authorization: Bearer`)

### To Do
- Upload Parquet files to R2
- Build query layer (DuckDB WASM or DataFusion WASM)
- Fetch additional quarters/years
- Add SQLite/DataFusion query endpoint

## Project Structure

```
.
├── ingest/                        # Rust ingestion tool
│   ├── Cargo.toml                 # DataFusion, reqwest, object_store
│   └── src/main.rs                # CLI: Fannie API → CSV → Parquet → R2
├── test-data/
│   ├── sample.csv                 # 5-row test CSV (pipe-delimited, no headers)
│   └── sample.parquet             # Converted Parquet output
├── scripts/
│   └── setup-env.sh               # Reads secrets/datadynamics.yaml → .env
├── .pi/skills/                    # Cloudflare agent skills
├── secrets/                       # (gitignored) Fannie Mae credentials
├── .env                           # (gitignored) Environment variables
├── ARCHITECTURE.md                # Full architecture analysis
├── .mcp.json                      # Cloudflare MCP servers
└── README.md                      # This file
```

## Setup (on a new machine)

### Prerequisites
- Rust toolchain (edition 2024, tested with rustc 1.92)
- pnpm (or npm)
- Cloudflare account with R2 enabled
- Fannie Mae developer portal account with an app for the Loan Performance History API

### Credentials

Copy `.env.example` to `.env` and fill in:

```bash
cp .env.example .env
```

Required:
- `FANNIE_CLIENT_ID` and `FANNIE_CLIENT_SECRET` — from Fannie Mae developer portal app
- `R2_ACCOUNT_ID` — Cloudflare account ID (dashboard → right sidebar)

Optional (for programmatic R2 upload):
- `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY` — R2 API token (Cloudflare dashboard → R2 → API Tokens)

### Tools

```bash
# Install wrangler (already in pnpm deps)
pnpm install

# Authenticate wrangler
pnpm wrangler login

# Build the ingestion tool
cd ingest && cargo build
```

### Cloudflare Skills

The Cloudflare agent skills are in `.pi/skills/`. Pi (or any Agent Skills-compatible agent) will auto-discover them.

Pi will prompt you to trust the project when first running in this directory.

## Ingestion Tool Usage

```bash
# Convert local CSV to Parquet (pipe-delimited, no headers)
cd ingest && cargo run -- --input ../data.csv --output ../data.parquet

# Fetch from Fannie Mae API and convert
cargo run -- --year 2024 --quarter Q1 --output data.parquet

# Upload to R2 after conversion
cargo run -- --year 2024 --quarter Q1 --output data.parquet \
  --r2-bucket fannie-mae-poc \
  --r2-key 2024/Q1/data.parquet
```

## Fannie Mae API Details

| Item | Value |
|------|-------|
| API Base URL | `https://api.fanniemae.com` |
| Token Endpoint | `https://auth.pingone.com/4c2b23f9-52b1-4f8f-aa1f-1d477590770c/as/token` |
| Auth Flow | OAuth2 client credentials (Client ID + Client Secret) |
| Token Scope | `clientcredential` |
| API Docs | `devptlpub.fv7dp.etss.prod.fanniemae.com` (login required) |
| App Name | `datafusion01` |
| API Product | `SingleFamilyLphExchangeAPI` |

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/sf-loan-performance-data/years/{year}/quarters/{quarter}` | Data for year/quarter |
| GET | `/v1/sf-loan-performance-data/primary-dataset` | Full dataset |
| GET | `/v1/sf-loan-performance-data/harp-dataset` | HARP dataset |

Response contains signed S3 URLs (`s3Uri`) for actual CSV download.

### Known Auth Issue

The PingOne token endpoint returns a valid access token, but `api.fanniemae.com` returns 401 regardless of how the token is sent. The app IS subscribed to `SingleFamilyLphExchangeAPI`. Possible causes:
1. Token needs additional scopes or a specific audience claim
2. API expects a different header format
3. The developer portal app needs an additional approval step
4. The API gateway uses a different auth scheme not reflected in the OpenAPI spec

## R2 Bucket

- Bucket: `fannie-mae-poc`
- Storage class: Standard
- No egress fees
- Wrangler commands:
  ```bash
  pnpm wrangler r2 object list fannie-mae-poc --remote   # list objects
  pnpm wrangler r2 object put BUCKET/KEY --file FILE --remote  # upload
  pnpm wrangler r2 object get BUCKET/KEY --remote        # download
  ```

## Notes for Another Agent

- The OpenAPI spec is at `/home/mr/Downloads/Single-Family Loan Performance History API.json`
- The Python reference client is at `Developer-Portal-Fannie-Mae/fnmapublic-python-clients` on GitHub — it shows the exact auth flow
- Fannie Mae CSVs are **pipe-delimited** (`|`) and have **no header row** — schema is in their file layout PDF
- The sample file URL: `https://capitalmarkets.fanniemae.com/resources/file/credit-risk/xls/sf-loan-performance-data-sample.csv`
- Data is grouped by acquisition year/quarter, updated quarterly
- The PingOne environment ID in the token URL is specific to Fannie Mae's developer portal
- Playwright browser was used to inspect the developer portal — there's a `playwright-cli` skill available
