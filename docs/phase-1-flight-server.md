# Flight SQL Server

## What it does

A Rust binary (`flight-sql-server`) that:

- Exposes a Flight SQL endpoint on `localhost:50051`
- Uses [DuckLake](https://github.com/datafusion-contrib/datafusion-ducklake) for table metadata (SQLite catalog + Parquet data files)
- Executes SQL via DataFusion 54, streams results as Arrow record batches
- Writes PID to `data/.lock` (shared lock) to prevent concurrent `add-quarter` runs

```
Flight SQL :50051 ──→ DuckLake catalog (SQLite) ──→ data/datalake/*.parquet
     │
Node proxy :8765 ──→ password gate + gRPC-web ──→ browser
     │
cloudflared ──→ *.trycloudflare.com (optional)
```

## Stack

```toml
datafusion = "54"
datafusion-ducklake = { version = "0.5", features = ["metadata-sqlite", "write-sqlite"] }
datafusion-flight-sql-server = { git = "..." }  # custom fork, DF54-compatible
tokio = "1"
tonic = "0.12"
tonic-web = "0.14"
fs2 = "0.4"  # POSIX file locking
clap = "4"
```

## Startup

```bash
# Host
./flight-sql-server --data-dir ./data --parquet /path/to/seed.parquet
# --parquet is only used for first-time catalog init (if catalog.db doesn't exist)

# Docker
docker run -v ./data:/app/data -e APP_PASSWORD=demo -e TUNNEL=1 fannie-flight
```

The entrypoint starts:
1. Flight SQL server on :50051
2. Node proxy on :8765 (password gate, gRPC-web, static files)
3. Optional Cloudflare Tunnel

## DuckLake Catalog

- **Metadata**: `data/catalog.db` (SQLite)
- **Data**: `data/datalake/<uuid>.parquet` (UUID-named chunks)
- **Tables**: `main.loans` (the loans table), `information_schema.*` (snapshots, schemata, files)
- **Snapshots**: Each `add-quarter` chunk creates a snapshot (500k rows per chunk)
- **Time travel**: Query at any snapshot point

## Adding Data

Use `add-quarter` (streaming, 500k-row chunks):

```bash
./add-quarter --data-dir ./data --parquet 2024Q1.parquet --max-memory-mb 4000
# --max-memory-mb 0 = no limit
# Uses execute_stream() → append_table() per chunk — never loads all rows into memory
```

**Do not run `add-quarter` while `flight-sql-server` is running.** The file lock (`data/.lock`) enforces this.

## Node Proxy

`public/server.mjs` — a lightweight Node.js proxy that:

- Serves static files (login page, query UI)
- Password gate: `POST /login` validates against `APP_PASSWORD` env var
- gRPC-web proxy: forwards `/arrow.flight.protocol.FlightService/*` to `:50051`
- Auth: checks `auth` cookie or `x-auth-token` header on gRPC-web requests
- CORS: permissive for local/dev use

## Query Performance

~751M rows, 32 quarters. Typical queries:

| Query | Rows | Time |
|-------|------|------|
| `count(*)` | 1 | ~500ms |
| `WHERE property_state='PA' LIMIT 10` | 10 | ~80ms |
| `avg(original_upb) GROUP BY year` | 8 | ~2s |

## File Locking

`src/lock.rs` — prevents concurrent `flight-sql-server` + `add-quarter`:

- **Shared lock** (`flock` LOCK_SH): flight-server holds while running
- **Exclusive lock** (`flock` LOCK_EX): add-quarter acquires before writing
- **Crash-safe**: OS releases on process exit
- **Error messages**: Shows which PID holds the conflicting lock

```rust
// flight-server
let _lock = Lock::read(&data_dir)?;  // shared, allows other readers

// add-quarter
let _lock = Lock::write(&data_dir)?;  // exclusive, blocks readers
```
