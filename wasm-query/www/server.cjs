const http = require('http'), fs = require('fs'), path = require('path');
http.createServer((req, res) => {
  let u = req.url.split('?')[0], fp = path.join(__dirname, u === '/' ? 'index.html' : u);
  const m = { '.html': 'text/html', '.js': 'application/javascript', '.wasm': 'application/wasm' };
  try { const d = fs.readFileSync(fp); res.writeHead(200, { 'Content-Type': m[path.extname(fp)] || 'application/octet-stream', 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' }); res.end(d); }
  catch(e) { res.writeHead(404); res.end('404'); }
}).listen(8765, () => console.log('listening'));
