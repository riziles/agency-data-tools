# Phase 2: Browser Flight SQL Client (local)

## What it does

A browser app that:
- Connects to the local Flight SQL server at `localhost:50051`
- Replaces the WASM query engine with gRPC-web Flight SQL calls
- Keeps the same dark theme, SQL editor, and result table from the current app
- Results arrive as Arrow record batches (no JSON overhead)

## Dependencies

```html
<!-- Arrow JS for in-browser data handling -->
<script type="module">
import { tableFromIPC, tableToIPC } from "https://unpkg.com/apache-arrow@latest/+esm";
</script>
```

```json
{
  "dependencies": {
    "@sparrowjs/flight": "^0.2.0"
  }
}
```

Two options for the Flight SQL client:
- **`@sparrowjs/flight`** (npm) — dedicated browser Flight SQL client over gRPC-web
- **`@lancedb/arrow-flight-sql-client`** (npm) — simpler API, less mature

For Phase 2, start with `@sparrowjs/flight` as it's designed for browser use.

## Key changes from current app

### Before (WASM)
```js
import { WrenEngine } from "https://unpkg.com/@wrenai/wren-core-wasm@0.4.1/dist/index.js";

const engine = await WrenEngine.init();
await engine.registerParquet("loans", buf);  // 7s wait
await engine.loadMDL(MDL, { source: "" });

const rows = await engine.query("SELECT * FROM \"Loans\" LIMIT 10");
```

### After (Flight SQL)
```js
import { connect } from "@sparrowjs/flight";

const client = await connect("http://localhost:50051");

const rows = await client.query('SELECT * FROM "Loans" LIMIT 10');
// rows is an Arrow Table — no JSON.parse(), just column access
```

**Instant page load** — no WASM download, no Parquet download.

## Result rendering with Arrow

```js
function renderResults(arrowTable) {
    const schema = arrowTable.schema;
    const keys = schema.fields.map(f => f.name);

    // Build header
    const thead = document.createElement("thead");
    const hr = document.createElement("tr");
    keys.forEach(k => { const th = document.createElement("th"); th.textContent = k; hr.appendChild(th); });
    thead.appendChild(hr);

    // Build body from Arrow batches
    const tbody = document.createElement("tbody");
    for (let i = 0; i < arrowTable.numRows; i++) {
        const tr = document.createElement("tr");
        keys.forEach(k => {
            const td = document.createElement("td");
            td.textContent = arrowTable.getChild(k)?.get(i) ?? "";
            tr.appendChild(td);
        });
        tbody.appendChild(tr);
    }

    resultTable.innerHTML = "";
    resultTable.appendChild(thead);
    resultTable.appendChild(tbody);
    rowCountEl.textContent = arrowTable.numRows.toLocaleString() + " rows";
}
```

## Schema discovery

Instead of hardcoding the 28-column MDL, the Flight SQL server can return table schemas:

```js
// Get available tables
const flights = await client.listFlights();
// flights contains schema info per table

// Run schema-informed queries
const rows = await client.query(sql);
// rows.schema.fields tells you column names and types
```

This eliminates the need to maintain column lists in both the server and client.

## gRPC-web proxy concern

Browsers can't do raw gRPC (HTTP/2). Flight SQL clients use **gRPC-web** which works over HTTP/1.1. The Flight SQL server needs to support gRPC-web, or a proxy is needed.

Options:
1. **tonic with gRPC-web** — `tonic-web` crate wraps a tonic server to accept gRPC-web
2. **Envoy proxy** — overkill for local dev
3. **Use `@sparrowjs/flight`** directly — it handles gRPC-web transport internally

For Phase 2, use `tonic` + `tonic-web` on the server side to accept gRPC-web requests directly from the browser.

## File structure

```
query/
├── index.html        # Rewrite: Flight SQL client instead of WASM
├── package.json      # If using npm for sparrowjs
└── server.cjs        # Dev server (serves static files only now)
```

The HTML file loads `@sparrowjs/flight` via CDN or bundled JS. The dev server only serves static files — no more 117MB WASM binary.

## Test workflow

```bash
# Terminal 1: Start Flight SQL server
cd flight-server && cargo run

# Terminal 2: Start static file server
cd query && node server.cjs

# Browser: Open http://localhost:8765
# Type a query, hit Run — results appear instantly (no 7s wait)
```
