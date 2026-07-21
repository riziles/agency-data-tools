# Phase 1: DataFusion Flight SQL Server (local)

## What it does

A Rust binary that:
- Exposes a Flight SQL endpoint on `localhost:50051`
- Reads Parquet files from R2 via S3-compatible API (range requests)
- Executes SQL queries using DataFusion and streams results as Arrow record batches

```
Browser                   Rust binary              R2
  │                          │                      │
  │  gRPC-web Flight SQL     │  S3 GetObject (range)│
  │ ──────────────────────▶  │ ────────────────────▶│
  │                          │                      │
  │  Arrow RecordBatches     │  Parquet byte ranges │
  │ ◀──────────────────────  │ ◀──────────────────── │
```

## Dependencies

```toml
[dependencies]
datafusion = "53"
datafusion-flight-sql-server = "53"
object_store = { version = "0.13", features = ["aws"] }
tokio = { version = "1", features = ["full"] }
tonic = "0.12"
clap = { version = "4", features = ["derive"] }
```

## Key implementation points

### 1. R2 as S3-compatible object store

R2 supports the S3 API. Configure the object store with the public endpoint:

```rust
use object_store::aws::AmazonS3Builder;

let r2 = AmazonS3Builder::new()
    .with_bucket_name("fannie-mae-poc")
    .with_endpoint("https://pub-a0dfcedd4df848e5bfca15cca210aea5.r2.dev")
    .with_allow_http(true)
    .with_region("auto")
    .with_skip_signature(true)  // public bucket, no auth
    .build()?;
```

No credentials needed for the public dev endpoint.

### 2. Register Parquet tables

```rust
let ctx = SessionContext::new();
ctx.runtime_env(RuntimeEnv::default().with_object_store_registry(registry));

// Register each quarter/file as a table
ctx.register_listing_table(
    "loans",
    "s3://2024/Q1/loans.parquet",
    ListingTableOptions::new(Arc::new(ParquetFormat::default())),
    None,  // No partition columns
).await?;
```

DataFusion reads only what's needed via range requests — no full file download.

### 3. Implement FlightSqlService

```rust
#[tonic::async_trait]
impl FlightSqlService for FannieMaeFlightServer {
    // Tables are auto-detected from SessionContext
    // Queries are forwarded to ctx.sql(sql).collect().await
    // Results streamed as Arrow FlightData batches
}
```

The `datafusion-flight-sql-server` crate provides the `FlightSqlService` trait. Most methods are boilerplate — the core `do_get()` method runs SQL and streams results.

### 4. MDL semantic layer

Register tables with the same 28-column schema used in the browser app:

| Category | Columns |
|---|---|
| Identifiers | loan_id, monthly_reporting_period |
| Loan terms | original_interest_rate, current_interest_rate, original_upb, current_actual_upb, original_loan_term |
| Dates | origination_date, first_payment_date, maturity_date, loan_age |
| Risk metrics | original_ltv, original_cltv, dti, borrower_credit_score_at_origination, co_borrower_credit_score_at_origination |
| Property | property_state, property_type, number_of_units, occupancy_status, msa, zip_code_short |
| Borrower | first_time_home_buyer_indicator, number_of_borrowers, loan_purpose |
| Servicing | channel, seller_name, servicer_name |

## Test with grpcurl

```bash
cargo run -- --bind 127.0.0.1:50051

# List tables
grpcurl -plaintext localhost:50051 \
  arrow.flight.protocol.FlightService/ListFlights

# Run a query
grpcurl -plaintext -d '{"query":"SELECT count(*) FROM \"Loans\""}' \
  localhost:50051 \
  arrow.flight.protocol.FlightService/GetFlightInfo
```

## File structure

```
flight-server/
├── Cargo.toml
└── src/
    └── main.rs          # Server setup, R2 config, table registration
```

Single binary, single file. Keep it minimal for now.
