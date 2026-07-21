# Phase 1: DataFusion Flight SQL Server (local)

## What it does

A Rust binary that:
- Exposes a Flight SQL endpoint on `localhost:50051`
- Reads Parquet files from the local filesystem
- Executes SQL queries using DataFusion and streams results as Arrow record batches

```
Browser                          Rust binary              Local disk
  │                                │                        │
  │  gRPC-web Flight SQL           │  read Parquet           │
  │ ────────────────────────────▶  │ ────────────────────▶  │
  │                                │                        │
  │  Arrow RecordBatches           │                        │
  │ ◀────────────────────────────  │                        │
```

## Dependencies

```toml
[dependencies]
datafusion = "53"
datafusion-flight-sql-server = "53"
tokio = { version = "1", features = ["full"] }
tonic = "0.12"
tonic-web = "0.12"    # gRPC-web support for browser
clap = { version = "4", features = ["derive"] }
```

## Key implementation points

### 1. Read Parquet from local files

DataFusion reads Parquet from the filesystem with zero config:

```rust
let ctx = SessionContext::new();

// Register a directory of Parquet files (supports glob patterns)
ctx.register_parquet("loans", "data/2024Q1.parquet", ParquetReadOptions::default()).await?;
```

The `data/` directory contains the already-converted Parquet files from the ingestion pipeline (e.g., `ingest/test-data/`).

### 2. Implement FlightSqlService

```rust
#[tonic::async_trait]
impl FlightSqlService for FannieMaeFlightServer {
    // Tables are auto-detected from SessionContext
    // Queries forwarded to ctx.sql(sql).collect().await
    // Results streamed as Arrow FlightData batches
}
```

The `datafusion-flight-sql-server` crate provides the trait. Most methods are boilerplate — `do_get()` runs SQL and streams results.

### 3. gRPC-web support

Browsers can't do raw gRPC (HTTP/2). Add `tonic-web` to accept gRPC-web over HTTP/1.1:

```rust
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;

let svc = FlightServiceServer::new(service);

let app = tonic::transport::Server::builder()
    .accept_http1(true)                       // enable HTTP/1.1
    .layer(GrpcWebLayer::new())               // gRPC-web wrapper
    .layer(CorsLayer::permissive())            // browser CORS
    .add_service(svc)
    .serve(addr);
```

### 4. Schema

Register tables with the 28-column subset used in the browser app:

| Category | Columns |
|---|---|
| Identifiers | loan_id, monthly_reporting_period |
| Loan terms | original_interest_rate, current_interest_rate, original_upb, current_actual_upb, original_loan_term |
| Dates | origination_date, first_payment_date, maturity_date, loan_age |
| Risk metrics | original_ltv, original_cltv, dti, borrower_credit_score_at_origination, co_borrower_credit_score_at_origination |
| Property | property_state, property_type, number_of_units, occupancy_status, msa, zip_code_short |
| Borrower | first_time_home_buyer_indicator, number_of_borrowers, loan_purpose |
| Servicing | channel, seller_name, servicer_name |

## File structure

```
flight-server/
├── Cargo.toml
├── data/                    # Symlink or copy to ingest/test-data/
│   └── 2024Q1.parquet       # 39 MB, 3,989,404 rows
└── src/
    └── main.rs              # Server setup, register tables, serve
```

Single binary. Copy the Parquet file from the existing `ingest/test-data/` directory.

## Test with grpcurl

```bash
cargo run -- --bind 127.0.0.1:50051 --data-dir ./data

# List tables
grpcurl -plaintext localhost:50051 \
  arrow.flight.protocol.FlightService/ListFlights

# Run a query
grpcurl -plaintext -d '{"query":"SELECT count(*) FROM \"Loans\""}' \
  localhost:50051 \
  arrow.flight.protocol.FlightService/GetFlightInfo
```
