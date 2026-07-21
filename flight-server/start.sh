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

# ── pick binary ──
if [ "$1" = "--release" ]; then
  BIN="$SCRIPT_DIR/target/release/flight-sql-server"
  if [ ! -f "$BIN" ]; then
    echo "Building release binary..."
    cd "$SCRIPT_DIR" && cargo build --release 2>&1
  fi
else
  BIN="$SCRIPT_DIR/target/debug/flight-sql-server"
  if [ ! -f "$BIN" ]; then
    echo "Building debug binary..."
    cd "$SCRIPT_DIR" && cargo build 2>&1
  fi
fi

# ── Flight SQL server ──
echo "Starting Flight SQL server (${BIN##*/}) on port 50051..."
"$BIN" --data-dir "$SCRIPT_DIR/data" --parquet "$SCRIPT_DIR/data/2024Q1.parquet" &
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
