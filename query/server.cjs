const http = require("http");
const fs = require("fs");
const path = require("path");
const PORT = 8765;
const ROOT = __dirname;
const MIME = { ".html": "text/html", ".js": "application/javascript", ".wasm": "application/wasm", ".parquet": "application/vnd.apache.parquet", ".css": "text/css" };
http.createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  let filePath = path.join(ROOT, url.pathname === "/" ? "/index.html" : url.pathname);
  fs.readFile(filePath, (err, data) => {
    if (err) { res.writeHead(404); res.end("Not found"); return; }
    const ext = path.extname(filePath);
    res.writeHead(200, { "Content-Type": MIME[ext] || "application/octet-stream", "Access-Control-Allow-Origin": "*", "Accept-Ranges": "bytes", "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Embedder-Policy": "require-corp", "Cache-Control": "no-store" });
    res.end(data);
  });
}).listen(PORT, () => console.log(`Serving on http://localhost:${PORT}`));
