#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PID_FILE="$SCRIPT_DIR/.server-pids"

cleanup() {
  echo ""
  if [ -f "$PID_FILE" ]; then
    echo "Stopping servers..."
    while read pid; do
      kill "$pid" 2>/dev/null && echo "  Killed $pid"
    done < "$PID_FILE"
    rm -f "$PID_FILE"
  fi
  echo "Done."
}
trap cleanup EXIT INT TERM

# ── Flight SQL server ──
echo "Starting Flight SQL server (port 50051)..."
cd "$SCRIPT_DIR"
cargo run -- --data-dir ./data --parquet ./data/2024Q1.parquet &
echo $! >> "$PID_FILE"

# ── Wait for Flight SQL to be ready ──
echo -n "Waiting for Flight SQL..."
for i in $(seq 1 30); do
  if curl -s -o /dev/null -w "%{http_code}" --max-time 1 http://127.0.0.1:50051/ 2>/dev/null | grep -q .; then
    echo " ready"
    break
  fi
  sleep 1
  echo -n "."
done

# ── Dev server ──
echo "Starting dev server (port 8765)..."
cd "$SCRIPT_DIR/public"
node server.mjs &
echo $! >> "$PID_FILE"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Open http://localhost:8765"
echo "  Press Ctrl+C to stop both servers"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

wait
