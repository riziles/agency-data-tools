#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

WASM="www/wasm_query_bg.wasm"

# ── Build if needed ──
if [ ! -f "$WASM" ]; then
  echo "Building DataFusion WASM (one time, ~2 min)..."
  export PATH="$HOME/.local/bin:$PATH"
  export LD_LIBRARY_PATH="$HOME/.local/lib"
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
    cargo build --target wasm32-unknown-unknown --release
  wasm-bindgen --target web --out-dir www \
    target/wasm32-unknown-unknown/release/wasm_query.wasm
fi

# ── Kill any existing server on 8765 ──
fuser -k 8765/tcp 2>/dev/null || true
sleep 1

# ── Serve ──
echo "Serving on http://localhost:8765"
echo "Press Ctrl+C to stop"
cd www
python3 -m http.server 8765 --bind 127.0.0.1
