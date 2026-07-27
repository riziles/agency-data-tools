import { createServer, request } from "node:http";

const PORT = 8765;
const FLIGHT_PORT = 50051;
const SVELTE_PORT = 3000;
const PASSWORD = process.env.APP_PASSWORD || "demo";

// ── Start SvelteKit ──
const { server: svelteServer } = await import("./dashboard/build/index.js");

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
  const cookies = parseCookies(req.headers.cookie);
  if (cookies.auth) return true;
  if (req.headers["x-auth-token"] === PASSWORD) return true;
  const auth = req.headers["authorization"] || "";
  if (auth === `Bearer ${PASSWORD}`) return true;
  return false;
}

function serveLogin(res, error) {
  const errHtml = error
    ? `<p style="color:#f44336;margin-bottom:1rem">${error}</p>`
    : "";
  res.writeHead(error ? 401 : 200, { "Content-Type": "text/html" });
  res.end(`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Fannie Mae — Login</title>
<style>
:root{--bg:#1a1a2e;--surface:#16213e;--primary:#e94560;--text:#eaeaea;--muted:#888;--border:#333}
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
  <p class="desc">751M rows &middot; 32 quarters &middot; DataFusion Flight SQL</p>
  ${errHtml}
  <form method="POST" action="/login">
    <input type="password" name="password" placeholder="Password" autofocus>
    <button type="submit">Sign In</button>
  </form>
</div>
</body>
</html>`);
}

function proxyToTarget(targetPort, req, res) {
  const opts = {
    hostname: "127.0.0.1",
    port: targetPort,
    path: req.url,
    method: req.method,
    headers: { ...req.headers, host: `127.0.0.1:${targetPort}` },
  };
  const proxy = request(opts, (targetRes) => {
    res.writeHead(targetRes.statusCode, {
      ...targetRes.headers,
      "access-control-allow-origin": "*",
      "access-control-allow-headers": "*",
      "access-control-allow-methods": "*",
    });
    targetRes.pipe(res);
  });
  proxy.on("error", () => {
    res.writeHead(502);
    res.end("Backend unreachable");
  });
  req.pipe(proxy);
}

function isGrpc(req) {
  const ct = req.headers["content-type"] || "";
  return (
    ct.includes("grpc-web") ||
    ct.includes("application/proto") ||
    (req.url || "").startsWith("/arrow.flight.")
  );
}

// ── Main proxy server ──
createServer(async (req, res) => {
  // CORS preflight
  if (req.method === "OPTIONS") {
    res.writeHead(204, {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Headers": "*",
      "Access-Control-Allow-Methods": "*",
    });
    res.end();
    return;
  }

  // Login
  if (req.method === "POST" && req.url === "/login") {
    let body = "";
    req.on("data", (c) => (body += c));
    req.on("end", () => {
      const params = new URLSearchParams(body);
      if (params.get("password") === PASSWORD) {
        res.writeHead(302, {
          Location: "/",
          "Set-Cookie": "auth=1; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        });
        res.end();
      } else {
        serveLogin(res, "Wrong password");
      }
    });
    return;
  }

  // Auth gate
  if (!isAuthenticated(req)) {
    const url = req.url || "/";
    if (url === "/" || url.startsWith("/?")) {
      serveLogin(res);
    } else {
      res.writeHead(401);
      res.end("Unauthorized");
    }
    return;
  }

  // gRPC-web → Flight SQL
  if (isGrpc(req)) {
    // Log the query
    const ts = new Date().toISOString();
    const ip = req.headers["x-forwarded-for"] || req.socket.remoteAddress || "?";
    console.log(`[${ts}] query from ${ip}`);
    proxyToTarget(FLIGHT_PORT, req, res);
    return;
  }

  // Everything else → SvelteKit
  proxyToTarget(SVELTE_PORT, req, res);
}).listen(PORT, () => {
  console.log(`Serving on http://localhost:${PORT}  (password: ${PASSWORD})`);
  console.log(`SvelteKit → :${SVELTE_PORT}  |  Flight SQL → :${FLIGHT_PORT}`);
});
