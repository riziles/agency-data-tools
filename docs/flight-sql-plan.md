# Flight SQL Server — DataFusion on Cloudflare Containers

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Browser                                                     │
│  @sparrowjs/flight (gRPC-web)  ←── Arrow RecordBatches      │
│  Dark theme UI                                              │
└──────────────┬──────────────────────────────────────────────┘
               │ gRPC-web (HTTP/1.1)
               ▼
┌──────────────────────────────┐      ┌───────────────────┐
│  Cloudflare Worker (optional)│      │  Cloudflare R2     │
│  Reverse proxy + auth        │      │  Parquet files     │
│  gRPC-web → gRPC bridge      │      │  fannie-mae-poc    │
└──────────────┬───────────────┘      └─────────┬─────────┘
               │                                │ S3 API
               ▼                                ▼
┌──────────────────────────────────────────────────────────────┐
│  Cloudflare Container                                       │
│  ┌────────────────────────────────────────────────────────┐  │
│  │  DataFusion Flight SQL Server                          │  │
│  │  - Rust binary (~30 MB)                                │  │
│  │  - Full threading (no WASM issues)                     │  │
│  │  - object_store → R2 (S3-compatible)                   │  │
│  │  - Parquet via range requests (zero upfront download)  │  │
│  │  - MDL semantic layer (from wren-core)                 │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## Development phases

### Phase 1: Flight SQL server binary (local)

Build a standalone DataFusion Flight SQL server that reads Parquet from R2.

- **Rust crate**: Uses `datafusion`, `datafusion-flight-sql-server`, `object_store`
- **Config**: R2 endpoint + credentials from env vars
- **Tables**: Pre-configured Parquet paths (e.g. `s3://fannie-mae-poc/2024/Q1/loans.parquet`)
- **Schema**: Hardcoded or loaded from config file (same 28-column MDL as current app)
- **Test**: `grpcurl` or Python Flight SQL client against `localhost:50051`

```rust
// Key dependencies
datafusion = "53"
datafusion-flight-sql-server = "53"  // contains FlightSqlService trait
object_store = { version = "0.13", features = ["aws"] }
tokio = { version = "1", features = ["full"] }
tonic = "0.12"  // gRPC server
```

### Phase 2: Browser client (local)

Rewrite the query UI to use Flight SQL instead of WASM.

- **Replace WASM engine** with [`@sparrowjs/flight`](https://github.com/balicat/sparrowjs) or [`@lancedb/arrow-flight-sql-client`](https://github.com/lancedb/arrow-flight-sql-client)
- **Same dark theme**, SQL editor, result table
- **Arrow integration**: Results arrive as Arrow record batches → render to HTML table without JSON round-trip
- **Test**: Browser → `localhost:50051` via gRPC-web

```js
import { connect } from "@sparrowjs/flight";

const client = await connect("http://localhost:50051");
const rows = await client.query(`SELECT * FROM "Loans" LIMIT 10`);
// rows is an Arrow Table — zero-copy to render
```

**Key difference from WASM**: Instant page load. No 7s download of 39MB Parquet. No 117MB WASM binary. Queries stream results via gRPC-web, Arrow batches arrive incrementally.

### Phase 3: Dockerize + local Tailscale

Containerize the Flight SQL server and expose via Tailscale.

```dockerfile
FROM rust:alpine AS builder
COPY . .
RUN cargo build --release

FROM alpine:latest
COPY --from=builder /target/release/flight-sql-server /usr/local/bin/
EXPOSE 50051
CMD ["flight-sql-server"]
```

- **Tailscale**: Run container with `tailscale serve` to get a stable `.ts.net` address
- **Browser access**: `https://flight-server.tailnet-name.ts.net` — works from any device on the tailnet
- **No Cloudflare deployment needed** for local dev

### Phase 4: Cloudflare Containers (production)

Deploy the same Docker image to Cloudflare Containers.

```toml
# wrangler.toml
name = "fannie-mae-flight"
[[containers]]
name = "flight-sql"
image = "registry.example.com/fannie-flight:v1"
port = 50051
[[containers.bindings]]
type = "r2"
bucket = "fannie-mae-poc"
```

- Worker proxy handles gRPC-web → gRPC translation + auth
- Container auto-scales to zero when idle
- R2 binding provides internal S3 endpoint (no public URL needed)

## Comparison: WASM vs Flight SQL

| | Browser WASM (current) | Flight SQL (new) |
|---|---|---|
| **Page load** | 7s (download 39MB) + 117MB WASM | Instant |
| **First query** | <1s (data in memory) | ~2-5s (range requests) |
| **Subsequent queries** | <1s | ~0.5-2s (cached metadata) |
| **Max query size** | Limited by browser memory | Unlimited (streaming) |
| **SQL features** | DataFusion WASM subset | Full DataFusion |
| **Threading** | ❌ No threads (WASM) | ✅ Native threads |
| **Parquet access** | Must download entire file | Range requests (reads only what's needed) |
| **Cost** | Free (runs on client) | Paid (container + requests) |
| **Dependencies** | 117MB WASM binary | 30MB server binary |
| **Multiple quarters** | Downloads all of them | Range requests scale naturally |

## Open questions

1. **Flight SQL vs plain HTTP/JSON**: Flight SQL maximizes performance (Arrow batches), but a simple HTTP `POST /query` that returns JSON is easier to build and debug. Worth the complexity?

2. **MDL semantic layer**: Should we port the wren-core MDL analyzer to the server, or keep table definitions simple?

3. **R2 credentials**: For local dev, use public R2 URL with no auth. For production, use Cloudflare's internal S3 endpoint via container binding.

4. **Tailscale vs direct**: Tailscale works for demos, but Cloudflare Tunnel is another option that's free with CF.

## Next step

Start Phase 1: build a minimal Flight SQL server that serves `SELECT * FROM "Loans"` reading from R2. Test with `grpcurl` locally.
