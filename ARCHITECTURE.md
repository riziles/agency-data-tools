# Architecture

## What We Built

A local-first analytics pipeline that runs entirely on a single machine:

```
Fannie Mae API ──→ Rust ingest (DF 54) ──→ Parquet files on disk
                                              │
                                    add-quarter (streaming)
                                              │
                                    DuckLake catalog (SQLite)
                                              │
                                    Flight SQL server :50051
                                              │
                                    Node proxy :8765 (password gate)
                                              │
                          ┌───────────────────┼───────────────────┐
                          ▼                                       ▼
                    Browser query UI                     Cloudflare Tunnel
                    (localhost:8765)                    (*.trycloudflare.com)
```

## Why Local-First?

The original plan (below) targeted Cloudflare R2 + WASM for $0/month. That approach works for demos but has limitations:

| Concern | WASM + R2 | Flight SQL + local |
|---------|-----------|-------------------|
| Query speed | 4.8s (40MB download, no pruning) | ~500ms (server-side DataFusion) |
| Data scale | 1 file at a time (wasm-bindgen limits) | 32 quarters, 751M rows |
| SQL support | Limited (WASM subset) | Full DataFusion SQL |
| Deployment | GitHub Pages (static) | Docker + Cloudflare Tunnel |
| Cost | $0 (R2 free tier) | $0 (your machine) |

Both approaches are valid for different use cases. The **WASM demo** lives on the `wasm-query` branch and works on GitHub Pages. The **Flight SQL server** on `flight-sql-server` branch handles the full dataset.

## Data Flow

### 1. Ingest (`ingest/`)

```
Fannie Mae API
  │  OAuth2 (PingOne)
  │  GET /v1/sf-loan-performance-data/years/{year}/quarters/{quarter}
  ▼
Signed S3 URL (valid ~1 hour)
  │
  ▼
ZIP download (~50 MB – 1.8 GB)
  │  Pipe-delimited CSV, no header, 113 columns
  ▼
CSV → Parquet (DataFusion 54)
  │  33:1 compression on disk
  ▼
test-data/{year}{quarter}.parquet
```

### 2. Load to DuckLake (`add-quarter`)

```
Source Parquet
  │  execute_stream() — never loads all rows into memory
  │  500k-row chunks
  ▼
append_table() per chunk → UUID-named Parquet in data/datalake/
  │  SQLite catalog tracks which files belong to main.loans
  ▼
Snapshot created per chunk (time travel points)
```

### 3. Query

```
Browser SQL
  │  gRPC-web (tonic-web)
  │  Password gate (cookie or x-auth-token header)
  ▼
Node proxy :8765
  │  Forwards to Flight SQL :50051
  ▼
DataFusion 54 + DuckLake
  │  Reads catalog → finds relevant Parquet files
  │  Pushes down predicates, reads only needed columns
  ▼
Arrow RecordBatches → browser
  │  @sparrowflight/js deserializes
  ▼
HTML table rendered
```

## Disk Layout

```
~140 GB total
├── flight-server/data/     16 GB   DuckLake catalog + datalake chunks
├── test-data/               7 GB   Source Parquet backups
├── zip-backup/fannie_*.zip   17 GB   Downloaded ZIP caches
```

## Key Numbers

| Metric | Value |
|--------|-------|
| Total rows | ~751,000,000 |
| Quarters | 32 (2018 Q1 – 2025 Q4) |
| DuckLake snapshots | ~1,485 |
| DuckLake data size | 16 GB |
| Source Parquet total | 7 GB |
| ZIP cache total | 17 GB |
| Ingest memory (peak) | ~860 MB per chunk |
| Query latency | ~500ms (aggregate), ~80ms (filtered) |
| Docker image | 804 MB (no data baked in) |
| Image build time | ~7 minutes |

## Original Architecture Analysis

The initial plan (below) targeted a different architecture: R2 storage + WASM query engine, deployed to GitHub Pages. We pivoted to a local Flight SQL server for production-scale data, but kept the WASM approach as a demo on the `wasm-query` branch.

### Why the pivot?

1. **Data volume**: Individual quarters reach 80M rows (1.8 GB ZIPs). WASM can't efficiently handle >100M rows.
2. **Full SQL**: DataFusion Flight SQL supports window functions, CTEs, subqueries — not available in WASM.
3. **DuckLake**: Snapshots, time travel, compaction, maintenance commands — requires a proper catalog, not just `read_parquet()`.
4. **Multi-file**: 32 quarters × hundreds of chunks requires a catalog to track them all.
5. **wasm-bindgen hangs**: Intermittent hangs on CI-like machines (works on dev machines), making WASM deployment unreliable.

### When to use WASM + R2

- **Demo/POC**: Single quarter, pre-filtered, deployed to GitHub Pages for $0
- **Small data**: Subset of 10K–100K rows for interactive exploration
- **Embedded analytics**: Where you can't run a server

The `wasm-query` branch has a working WASM demo with smart fetch (byte-range row group download) and column projection.
