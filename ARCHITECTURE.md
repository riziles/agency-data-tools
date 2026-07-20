# Fannie Mae Loan Performance Data — Cloudflare Architecture

## Verdict: Not Crazy, But Needs a Tweaked Approach

Your thesis is sound. Storing the data as Parquet in R2 and querying it locally via DataFusion WASM is a smart, low-cost architecture. A few adjustments needed for the ingestion pipeline.

---

## The Data

**Fannie Mae Single-Family Loan Performance Data**

| Item | Details |
|------|---------|
| URL | https://capitalmarkets.fanniemae.com/credit-risk-transfer/single-family-credit-risk-transfer/fannie-mae-single-family-loan-performance-data |
| Access | Free registration + Terms & Conditions acceptance |
| Format | CSV inside ZIP files (**no column headers** — use separate file layout PDF) |
| Datasets | **Acquisition** (static origination data) + **Monthly Performance** (dynamic loan status history) |
| History | ~2000 to present, quarterly releases |
| Size | Multi-GB per quarter — full dataset is **hundreds of GB** |
| Grouping | Files grouped by acquisition year/quarter |
| Updates | Released quarterly — re-download every quarter |
| POC | Sample file available for testing (see "Sample File" on the data page) |
| Tools | Fannie Mae provides R code for downloading both Primary and HARP datasets |

---

## R2 Storage Costs

| Item | Cost |
|------|------|
| **Storage (Standard)** | **$0.015 / GB-month** |
| **Storage (Infrequent Access)** | $0.01 / GB-month |
| **Egress (data transfer to Internet)** | **$0.00 — FREE** |
| Class A Operations (writes) | $4.50 / million requests |
| Class B Operations (reads) | $0.36 / million requests |

**Free tier**: 10 GB-month storage, 1M writes, 10M reads/month.

**Example**: Storing 100 GB of Parquet files costs **~$1.50/month**, and querying it costs $0 in bandwidth. This is the killer feature of R2.

---

## Worker Limitation (Critical)

A single Cloudflare Worker **cannot** download multi-GB CSVs and convert them to Parquet:

| Limit | Free Plan | Paid Plan |
|-------|-----------|-----------|
| Memory | 128 MB | 128 MB |
| CPU time | 10 ms | 30s default (up to 5 min) |
| Startup time | 1s | 1s |
| Worker size | 3 MB | 10 MB |

A 3 GB CSV won't fit in 128 MB of memory, and CSV→Parquet conversion of large files would exceed CPU limits.

**Solution**: Offload ingestion to a pay-per-use compute service (see below).

---

## DataFusion + WASM (This Works)

DataFusion compiles to WASM and runs in the browser. Proven projects:

- **datafusion-wasm-playground** — live demo: https://datafusion-contrib.github.io/datafusion-wasm-playground/
- **CREATE EXTERNAL TABLE** syntax works via HTTP range requests:

```sql
CREATE EXTERNAL TABLE loans
STORED AS PARQUET
LOCATION 'https://r2-bucket.example.com/data/2024/Q1/loans.parquet';
```

- R2 supports HTTP range requests natively, so DataFusion reads only the row groups it needs
- **tiny-parquet** (306 KB WASM) can write Parquet from Workers (small datasets) or from the browser

---

## DuckDB + R2 (Official Support)

DuckDB has **official, documented support** for querying Cloudflare R2 directly:

- DuckDB's `httpfs` extension speaks the S3 API, which R2 implements
- Can read/write Parquet files directly: `SELECT * FROM read_parquet('s3://bucket/file.parquet')`
- Uses HTTP range requests — reads only the row groups it needs, not entire files
- DuckDB **also compiles to WASM** and runs in the browser
- Reference project: [`casperm/duckdb-query-r2`](https://github.com/casperm/duckdb-query-r2) — browser-based SQL querying R2 via DuckDB WASM

R2 secret setup in DuckDB:
```sql
CREATE SECRET (
  TYPE r2,
  KEY_ID 'your-access-key-id',
  SECRET 'your-secret-access-key',
  ACCOUNT_ID 'your-account-id'
);
SELECT * FROM read_parquet('s3://my-bucket/data/2024/Q1/loans.parquet');
```

This is a strong alternative to DataFusion WASM for the query layer.

## WASM Query Engine Options

There are multiple proven options for running queries in the browser against R2 Parquet files:

| Option | Engine | Notes |
|--------|--------|-------|
| **DuckDB WASM** | DuckDB | Official R2 docs. Most mature analytics. SQLite-compatible CLI too. |
| **DataFusion WASM** | DataFusion | `datafusion-wasm-playground` — works today. S3 + HTTP support. |
| **wren-core-wasm** | DataFusion | Production-grade wrapper. Semantic layer support. Parquet/CSV/JSON. |
| **Axon** | DataFusion | Delta Lake in browser, falls back to native for large queries. |
| **Micromegas** | DataFusion | "Data warehouse in browser" — DataFusion WASM + Arrow IPC. |

All of them read Parquet via HTTP range requests, meaning they download only the row groups needed for your query — not the whole file.

## DataFusion-DuckLake Note

[DataFusion-DuckLake](https://github.com/datafusion-contrib/datafusion-ducklake) is a Rust-native crate that adds DuckLake catalog support. It does **not** compile to WASM easily. For a POC, plain DataFusion or DuckDB with Parquet files in R2 is a simpler path.

---

## Recommended Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    INGESTION PIPELINE                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Fannie Mae CSV.zip ──► GitHub Actions / Cheap VPS ──► R2   │
│       (multi-GB)         (convert to Parquet)   Parquet     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    QUERY LAYER                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Browser / CLI ──► DataFusion WASM ── HTTP range reqs ──► R2│
│  (your machine)      (free compute)      ($0 egress)        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Component 1: Ingestion (Pay-Per-Use Compute)

Since Workers are too constrained for multi-GB CSV→Parquet conversion, use a pay-per-use compute service instead. Key requirements:
- Download large CSV.zips (multi-GB)
- Convert CSV to Parquet (CPU/memory intensive)
- Upload to R2 via S3 API

#### Options Comparison

Since you want **Rust**, here are the pay-per-use options that work well with compiled binaries:

| Service | Rust Support | Model | Free Tier | Limits |
|---------|-------------|-------|-----------|--------|
| **[Modal](https://modal.com)** | **✅ Rust SDK (`modal-rs`) + custom Docker images** | Pay per second | **$30/month free compute** | 64GB RAM, 8 vCPUs |
| **[Google Cloud Run Jobs](https://cloud.google.com/run)** | **✅ Package in Docker** | Pay per second | 240K vCPU-seconds/month | 60 min timeout, 64GB RAM |
| **GitHub Actions** | **✅ Pre-installed Rust toolchain** | Pay per minute | 2,000 min/month (free) | 6hr timeout, 4 vCPUs, 14GB RAM |
| **[Fly.io Machines](https://fly.io/machines)** | **✅ Package in Docker** | Pay per second | None | Per-second billing on containers |
| **AWS Lambda** | **✅ Custom runtime + provided.al2023** | Pay per request + GB-second | 1M requests + 400K GB-seconds/month free | 15 min timeout, 10GB RAM |
| **[Railway](https://railway.app)** | **✅ Package in Docker** | Pay per second | $5 credit/month | 12GB RAM, 8 vCPUs |

**Recommendation**:
- **POC**: **GitHub Actions** — Rust is pre-installed, just `cargo run --release` in a workflow step. Free tier is generous.
- **Production**: **Modal** (Rust SDK + Docker, $30 free compute/month) or **Google Cloud Run Jobs** (pure Docker, no subscription).

#### Ingestion Pipeline (Rust)

You can use **DataFusion** itself for the CSV→Parquet conversion — keeps the stack consistent:

```rust
// ingest/src/main.rs
use datafusion::prelude::*;
use object_store::aws::AmazonS3Builder;
use std::sync::Arc;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = SessionContext::new();

    // 1. Read CSV (Fannie Mae CSVs have no headers, so provide schema)
    let schema = Arc::new(Schema::new(vec![
        Field::new("loan_id", DataType::Utf8, false),
        Field::new("origination_date", DataType::Utf8, false),
        Field::new("original_upb", DataType::Float64, false),
        Field::new("original_interest_rate", DataType::Float64, false),
        Field::new("original_loan_term", DataType::Int32, false),
        // ... etc from the file layout PDF
    ]));

    let csv_options = CsvReadOptions::new()
        .has_header(false)
        .schema(&schema);

    let df = ctx.read_csv("loan_data.csv", csv_options).await?;

    // 2. Write as Parquet locally
    df.write_parquet("loan_data.parquet", DataFrameWriteOptions::new(), None).await?;

    // 3. Upload to R2 via object_store (S3 API)
    let r2 = AmazonS3Builder::new()
        .with_endpoint("https://<account-id>.r2.cloudflarestorage.com")
        .with_access_key_id("your-key")
        .with_secret_access_key("your-secret")
        .with_region("auto")
        .with_bucket_name("fannie-mae-data")
        .build()?;

    ctx.runtime_env()
        .register_object_store(
            &Url::parse("s3://fannie-mae-data")?,
            Arc::new(r2),
        );

    // Or just use the aws-sdk-s3 crate to upload the file
    Ok(())
}
```

Dependencies: `datafusion` (CSV→Parquet, same engine as WASM query layer), `object-store` (with `aws` feature for R2 S3 API).

### Component 2: Storage (Cloudflare R2)

- R2 bucket with Parquet files
- Partition layout: `data/{year}/{quarter}/*.parquet`
- Cost: ~$0.015/GB-month. A 50 GB dataset costs ~$0.75/month
- $0 egress — querying costs nothing in bandwidth

### Component 3: Query (DuckDB WASM or DataFusion WASM)

- Runs **locally** in browser or as CLI — **free compute**
- Queries Parquet files via HTTP range requests (reads only needed row groups)
- **DuckDB WASM** has official R2 docs — easiest path for the POC
- **DataFusion WASM** is more flexible if you want custom processing pipelines
- DuckDB CLI also works great locally for power users: `SELECT * FROM read_parquet('s3://...')`

---

## POC Plan

| Step | What | Compute | Infra Cost |
|------|------|---------|------------|
| 1 | Register for Fannie Mae data access, download sample file | — | Free |
| 2 | Write ingestion script: CSV → Parquet → upload to R2 | Modal (free credit) or GitHub Actions (free tier) | ~$0 |
| 3 | Build simple HTML page with DuckDB WASM querying R2 | Browser (local) | Free |
| 4 | Run `SELECT COUNT(*), AVG(credit_score)` against the data | Browser (local) | ~$0 (no egress from R2) |

Approximate total POC cost: **<$1/month** (mostly R2 storage for the sample file)

---

## Tools Summary

| Tool | Purpose | Cost Model |
|------|---------|------------|
| Cloudflare R2 | Object storage for Parquet files | ~$0.015/GB-month, $0 egress |
| **Modal** | Ingestion (CSV → Parquet) | **$0.0000131/core/sec, $30/month free credit** |
| Google Cloud Run Jobs | Ingestion (container-based) | Pay per second, 240K vCPU-seconds free/month |
| GitHub Actions | Ingestion (POC) | Free tier (2,000 min/month) |
| DuckDB WASM / DataFusion WASM | Query engine in browser/CLI | Free |
| DuckDB CLI | Local analytics with direct R2 access | Free |
| Python + PyArrow | CSV → Parquet conversion | Free |
| Wrangler CLI | Manage R2, Workers, config | Free |
