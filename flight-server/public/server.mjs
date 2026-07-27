import { createServer, request } from "node:http";
import { readFile } from "node:fs/promises";
import { join, extname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const PORT = 8765;
const FLIGHT_PORT = 50051;
const PASSWORD = process.env.APP_PASSWORD || "demo";

const MIME = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
};

function parseCookies(header) {
  const c = {};
  if (!header) return c;
  header.split(";").forEach((p) => {
    const eq = p.indexOf("=");
    if (eq > 0) c[p.slice(0, eq).trim()] = p.slice(eq + 1).trim();
  });
  return c;
}

function isAuthenticated(req) {
  // Cookie
  const cookies = parseCookies(req.headers.cookie);
  if (cookies.auth) return true;
  // Header-based fallback (for gRPC-web clients that don't send cookies)
  if (req.headers["x-auth-token"] === PASSWORD) return true;
  const auth = req.headers["authorization"] || "";
  if (auth === `Bearer ${PASSWORD}`) return true;
  return false;
}

function serveLogin(res, error) {
  const errHtml = error ? `<p style="color:#f44336;margin-bottom:1rem">${error}</p>` : "";
  res.writeHead(error ? 401 : 200, { "Content-Type": "text/html" });
  res.end(`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Fannie Mae — Login</title>
<style>
:root{--bg:#1a1a2e;--surface:#16213e;--primary:#e94560;--text:#eaeaea;--muted:#888;--border:#333;--error:#f44336}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:system-ui,sans-serif;background:var(--bg);color:var(--text);display:flex;align-items:center;justify-content:center;min-height:100vh}
.box{background:var(--surface);padding:2.5rem;border-radius:12px;text-align:center;min-width:340px;border:1px solid var(--border)}
h1{font-size:1.5rem;margin-bottom:0.5rem}
.desc{color:var(--muted);margin-bottom:1.5rem;font-size:0.9rem}
input{width:100%;background:var(--bg);color:var(--text);border:1px solid var(--border);border-radius:6px;padding:0.75rem 1rem;font-size:1rem;margin-bottom:1rem}
input:focus{outline:none;border-color:var(--primary)}
button{width:100%;background:var(--primary);color:white;border:none;border-radius:6px;padding:0.75rem;font-size:1rem;cursor:pointer;font-weight:600}
button:hover{opacity:0.85}
</style>
</head>
<body>
<div class="box">
  <h1>🏠 Fannie Mae Loan Data</h1>
  <p class="desc">2024 Q1 &middot; 3,989,404 loans &middot; DataFusion Flight SQL</p>
  ${errHtml}
  <form method="POST" action="/login">
    <input type="password" name="password" placeholder="Password" autofocus>
    <button type="submit">Sign In</button>
  </form>
</div>
</body>
</html>`);
}

createServer(async (req, res) => {
  // ── Handle CORS preflight ──
  if (req.method === "OPTIONS") {
    res.writeHead(204, {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Headers": "*",
      "Access-Control-Allow-Methods": "*",
    });
    res.end();
    return;
  }

  // ── Login endpoint ──
  if (req.method === "POST" && req.url === "/login") {
    let body = "";
    req.on("data", (c) => (body += c));
    req.on("end", () => {
      const params = new URLSearchParams(body);
      if (params.get("password") === PASSWORD) {
        res.writeHead(302, {
          Location: "/?token=" + encodeURIComponent(PASSWORD),
          "Set-Cookie": "auth=1; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        });
        res.end();
      } else {
        serveLogin(res, "Wrong password");
      }
    });
    return;
  }

  // ── Auth gate ──
  if (!isAuthenticated(req)) {
    const url = new URL(req.url || "/", `http://localhost:${PORT}`);
    if (url.pathname === "/" || url.pathname === "/index.html") {
      serveLogin(res);
    } else {
      res.writeHead(401);
      res.end("Unauthorized");
    }
    return;
  }

  // ── Authenticated zone ──
  const ct = req.headers["content-type"] || "";

  // Proxy gRPC-web / Flight SQL requests to the Rust server
  if (
    ct.includes("grpc-web") ||
    ct.includes("application/proto") ||
    (req.url || "").startsWith("/arrow.flight.")
  ) {
    const opts = {
      hostname: "127.0.0.1",
      port: FLIGHT_PORT,
      path: req.url,
      method: req.method,
      headers: { ...req.headers, host: `127.0.0.1:${FLIGHT_PORT}` },
    };
    const proxy = request(opts, (flightRes) => {
      res.writeHead(flightRes.statusCode, {
        ...flightRes.headers,
        "access-control-allow-origin": "*",
        "access-control-allow-headers": "*",
        "access-control-allow-methods": "*",
      });
      flightRes.pipe(res);
    });
    proxy.on("error", () => {
      res.writeHead(502);
      res.end("Flight SQL unreachable");
    });
    req.pipe(proxy);
    return;
  }

  // Serve static files
  const url = new URL(req.url, `http://localhost:${PORT}`);
  let path = url.pathname === "/" ? "/index.html" : url.pathname;
  const filePath = join(__dirname, path);

  try {
    const data = await readFile(filePath);
    const ext = extname(filePath);
    res.writeHead(200, {
      "Content-Type": MIME[ext] || "application/octet-stream",
      "Access-Control-Allow-Origin": "*",
      "Cache-Control": "no-store",
    });
    res.end(data);
  } catch {
    // SPA fallback: serve index.html for client-side routes (/query, /view, /docs)
    try {
      const data = await readFile(join(__dirname, "index.html"));
      res.writeHead(200, { "Content-Type": "text/html", "Access-Control-Allow-Origin": "*" });
      res.end(data);
    } catch {
      res.writeHead(404);
      res.end("404");
    }
  }
}).listen(PORT, () => {
  console.log(`Serving on http://localhost:${PORT}  (password: ${PASSWORD})`);
});
