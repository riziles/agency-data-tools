# Browser Query Client

## What it does

A single-page browser app that connects to the Flight SQL server via gRPC-web through the Node proxy. Delivers a dark-themed SQL editor with instant results — ~500ms for aggregate queries over 751M rows.

## Stack

- **`@sparrowflight/js`** — browser Flight SQL client (CDN, no npm)
- **esbuild** — bundles `app.js` into `build.js`
- **Node proxy** (`server.mjs`) — password gate, gRPC-web forwarding, static files
- **Arrow JS** — in-browser IPC deserialization (bundled in sparrowflight)

No WASM downloads. No Parquet downloads. The page loads instantly — raw HTML/CSS/JS.

## Key Files

```
public/
├── index.html           # Login page + query UI (dark theme)
├── build.js             # Bundled JS (@sparrowflight/js + app code)
├── server.mjs           # Node proxy
└── src/
    └── app.js           # Main app: login, query, results table
```

## How It Works

```
Browser                          Node proxy :8765                   Flight SQL :50051
  │                                 │                                  │
  │  GET /                          │                                  │
  │ ────────────────────────────▶   │  auth? → login page or app      │
  │ ◀────────────────────────────   │                                  │
  │                                 │                                  │
  │  POST /login                    │                                  │
  │ ────────────────────────────▶   │  validate APP_PASSWORD           │
  │ ◀──── Set-Cookie: auth=token    │                                  │
  │                                 │                                  │
  │  POST /arrow.flight.protocol.   │                                  │
  │    FlightService/DoPut          │                                  │
  │  Cookie: auth=token             │                                  │
  │ ────────────────────────────▶   │  gRPC-web → gRPC ────────────▶  │
  │                                 │                                  │  DataFusion
  │  Arrow RecordBatches            │                                  │  DuckLake
  │ ◀────────────────────────────   │ ◀─────────────────────────────── │
```

## Auth Flow

Two mechanisms (covers both static files and gRPC-web):

1. **Cookie** (`auth`) — set on successful login, checked by proxy for static file access
2. **Header** (`x-auth-token`) — fallback for gRPC-web requests where cookies may not be sent natively

The login page redirects to `?token=demo` on success. `app.js` reads `token` from URL and passes it as `x-auth-token` header on all gRPC-web calls.

```js
// app.js
const params = new URLSearchParams(window.location.search);
const token = params.get("token");

const client = await SparrowFlight.connect("http://" + window.location.host, {
  headers: { "x-auth-token": token }
});
```

## Result Rendering

Results arrive as Apache Arrow tables via `@sparrowflight/js`:

```js
const result = await client.query(sql);
// result is an Arrow Table with .schema (fields) and .getChild(name) for columns

function renderTable(arrowTable) {
  const fields = arrowTable.schema.fields.map(f => f.name);

  // Header
  let html = "<thead><tr>";
  fields.forEach(f => html += `<th>${f}</th>`);
  html += "</tr></thead><tbody>";

  // Rows
  for (let i = 0; i < arrowTable.numRows; i++) {
    html += "<tr>";
    fields.forEach(f => {
      html += `<td>${arrowTable.getChild(f)?.get(i) ?? ""}</td>`;
    });
    html += "</tr>";
  }
  html += "</tbody>";
  table.innerHTML = html;
}
```

## Dev Server

```bash
cd flight-server/public
APP_PASSWORD=demo node server.mjs
# Serves on :8765
# Proxies /arrow.flight.protocol.FlightService/* → :50051
# Intercepts POST /login → validates password → sets cookie
```

## Public Access

Two options for exposing beyond localhost:

### Cloudflare Tunnel (zero config)
```bash
docker run -e TUNNEL=1 ... fannie-flight
# Provides *.trycloudflare.com URL (changes each restart)
```

### Tailscale (stable URL)
```bash
# In start.sh
tailscale serve --bg --https=8765 http://127.0.0.1:8765
# Provides https://<hostname>.ts.net
```
