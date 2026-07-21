# Flight SQL Server — Fannie Mae Loan Data

## Quick start

```bash
cd flight-server
./start.sh          # debug build (slower queries)
./start.sh --release  # release build (fast)
```

Opens `http://localhost:8765` + auto-exposes over Tailscale.

## Architecture

```
Browser (localhost:8765)                Rust server (localhost:50051)
  │                                        │
  │  gRPC-web Flight SQL                   │  DuckLake catalog (SQLite)
  │ ────────────────────────────────────▶  │  ↓
  │  Arrow RecordBatches                   │  Parquet files (data/datalake/)
  │ ◀────────────────────────────────────  │
```

## Structure

```
flight-server/
├── start.sh              # Launches both servers, Ctrl+C to stop
├── Cargo.toml            # Rust deps: datafusion 54, ducklake 0.5, flight-sql-server
├── src/
│   ├── main.rs           # Flight SQL server + DuckLake catalog init
│   └── bin/
│       └── test_client.rs  # Quick integration test
├── data/
│   ├── 2024Q1.parquet    # Source data (not in repo — copy from test-data/)
│   ├── catalog.db        # DuckLake SQLite metadata (not in repo)
│   └── datalake/         # DuckLake-managed Parquet files (not in repo)
└── public/
    ├── index.html        # Browser query UI (dark theme)
    ├── src/app.js        # Flight SQL client (@sparrowflight/js)
    ├── server.mjs        # Dev server + gRPC-web proxy (port 8765 → 50051)
    ├── build.js          # esbuild bundler
    ├── package.json
    └── dist/bundle.js    # Bundled client (not in repo)
```

## First run

1. Copy a Parquet file to `data/2024Q1.parquet`
2. `./start.sh` — on first run it ingests the Parquet into DuckLake (writes uncompressed copy to `data/datalake/`). Subsequent runs skip ingestion.

## Key decisions

- **datafusion-ducklake** with SQLite metadata — no external database needed
- **@sparrowflight/js** for browser Flight SQL — browser-native, no WASM
- **Node dev server proxies gRPC-web** to the Rust server — single URL, no CORS pain
- **Tailscale auto-expose** in start.sh — access from any device on the tailnet
- **DuckLake copies data on ingest** — the source Parquet stays untouched; DuckLake manages its own copy. 39MB → 88MB (uncompressed). For production, use S3/R2 object store.
