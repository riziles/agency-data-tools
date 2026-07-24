#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

WASM="pkg/wasm_query_bg.wasm"

# ── Build if needed ──
if [ ! -f "$WASM" ]; then
  echo "Building DataFusion WASM (~3 min compile + ~5 min bindgen)..."
  echo "  This only runs once. Subsequent runs use cached build."
  export PATH="$HOME/.local/bin:$PATH"
  export LD_LIBRARY_PATH="$HOME/.local/lib"
  
  # Build the WASM binary
  echo "  [1/2] cargo build..."
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
    cargo build --target wasm32-unknown-unknown --release
  
  # Generate JS bindings (slow on large WASM, be patient)
  echo "  [2/2] wasm-pack..."
  RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
    wasm-pack build --target web --no-opt
  echo "  Build complete!"
fi

# Sync to www
cp pkg/wasm_query.js www/
cp pkg/wasm_query_bg.wasm www/

# ── Kill any existing server on 8765 ──
fuser -k 8765/tcp 2>/dev/null || true
sleep 1

# ── Serve ──
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Open http://localhost:8765"
echo "  Press Ctrl+C to stop"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cd www
python3 -m http.server 8765 --bind 127.0.0.1
