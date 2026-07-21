import { createServer, request } from "node:http";
import { readFile } from "node:fs/promises";
import { join, extname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const PORT = 8765;
const FLIGHT_PORT = 50051;

const MIME = {
  ".html": "text/html",
  ".js": "application/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
};

createServer(async (req, res) => {
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
    res.writeHead(404);
    res.end("404");
  }
}).listen(PORT, () => {
  console.log(`Serving on http://localhost:${PORT}`);
});
