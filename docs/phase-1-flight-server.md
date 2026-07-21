# Phase 1: DataFusion Flight SQL Server (local)

## Stack

- **DataFusion** — query engine
- **datafusion-ducklake** — table format + catalog (SQLite metadata, Parquet data)
- **Arrow Flight SQL** — wire protocol for browser client
- **tonic-web** — gRPC-web support so the browser can connect

## What it does

A Rust binary that:
- Exposes a Flight SQL endpoint on `localhost:50051`
- Stores table metadata in a local SQLite database (DuckLake catalog)
- Reads Parquet files from local disk
- Executes SQL queries using DataFusion → DuckLake and streams results as Arrow record batches

```
Browser                          Rust binary              Local disk
  │                                │                        │
  │  gRPC-web Flight SQL           │  DuckLake catalog      │
  │ ────────────────────────────▶  │  (SQLite metadata)     │
  │                                │  ↓                     │
  │                                │  DataFusion query      │
  │  Arrow RecordBatches           │  ↓                     │
  │ ◀────────────────────────────  │  Parquet files         │
```

## Dependencies

```toml
[dependencies]
datafusion = "53"
datafusion-ducklake = { version = "0.5", features = ["metadata-sqlite", "write-sqlite"] }
datafusion-flight-sql-server = "53"
tokio = { version = "1", features = ["full"] }
tonic = "0.12"
tonic-web = "0.12"
clap = { version = "4", features = ["derive"] }
```

Why `datafusion-ducklake` instead of raw DataFusion?

| Raw DataFusion | datafusion-ducklake |
|---|---|
| `ctx.register_parquet("t", "file.parquet")` | SQL `ADD FILES 'data/*.parquet' INTO TABLE loans` |
| No metadata tracking | Catalog tracks which files belong to which table |
| No schema enforcement | Schema validated on ingest |
| Just read | Read + write + maintenance (compaction, orphan cleanup) |
| No time travel | Snapshots |

## Key implementation points

### 1. DuckLake catalog with SQLite

SQLite is embedded — no server process needed:

```rust
use datafusion::prelude::*;
use datafusion_ducklake::{DuckLakeCatalog, SqliteMetadataProvider};

let provider = SqliteMetadataProvider::new("sqlite://catalog.db").await?;

let catalog = DuckLakeCatalog::new(provider)?;
let ctx = SessionContext::new();
ctx.register_catalog("ducklake", Arc::new(catalog));
```

### 2. Ingest Parquet into DuckLake

Once the catalog is registered, add our existing Parquet file:

```sql
-- Run inside the DuckLake catalog context
ADD FILES 'data/2024Q1.parquet' INTO TABLE ducklake.main.loans;
```

Or programmatically via DataFusion:

```rust
ctx.sql("ADD FILES 'data/2024Q1.parquet' INTO TABLE ducklake.main.loans").await?;
```

DuckLake reads the Parquet schema, creates the table metadata in SQLite, and the data stays in place (no copy).

### 3. Flight SQL service

```rust
use datafusion_flight_sql_server::FlightSqlService;

struct FannieMaeServer {
    ctx: SessionContext,
}

#[tonic::async_trait]
impl FlightSqlService for FannieMaeServer {
    // Tables auto-detected from SessionContext
    // Queries forwarded to ctx.sql(sql).collect().await
    // Results streamed as Arrow FlightData
}
```

### 4. gRPC-web for browser access

```rust
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;

let svc = FlightServiceServer::new(service);

tonic::transport::Server::builder()
    .accept_http1(true)
    .layer(GrpcWebLayer::new())
    .layer(CorsLayer::permissive())
    .add_service(svc)
    .serve(addr)
    .await?;
```

## File structure

```
flight-server/
├── Cargo.toml
├── data/
│   └── 2024Q1.parquet          # Copy from ingest/test-data/ (39 MB)
└── src/
    └── main.rs                 # Single file: catalog, ingest, serve

# Runtime state (gitignored):
#   catalog.db                  # SQLite metadata (auto-created)
```

## Test with grpcurl

```bash
cargo run -- --bind 127.0.0.1:50051 --data-dir ./data

# First run: ingest the Parquet file
grpcurl -plaintext -d '{"query":"ADD FILES '"'"'data/2024Q1.parquet'"'"' INTO TABLE ducklake.main.loans"}' \
  localhost:50051 \
  arrow.flight.protocol.FlightService/GetFlightInfo

# Query it
grpcurl -plaintext -d '{"query":"SELECT count(*) FROM ducklake.main.loans"}' \
  localhost:50051 \
  arrow.flight.protocol.FlightService/GetFlightInfo
```
