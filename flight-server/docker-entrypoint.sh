#!/usr/bin/env bash
set -e

echo "=== Fannie Mae Flight SQL ==="

# ── Start Flight SQL server ──
echo "[1/2] Starting Flight SQL on :50051..."
/app/flight-sql-server --data-dir /app/data --bind 0.0.0.0:50051 &
FLIGHT_PID=$!

# Wait until Flight SQL is accepting connections
echo -n "      Waiting"
for i in $(seq 1 30); do
  if curl -s -o /dev/null --max-time 1 http://127.0.0.1:50051/ 2>/dev/null; then
    echo " ready"
    break
  fi
  sleep 0.5
  echo -n "."
done

# ── Start SvelteKit + proxy server ──
echo "[2/2] Starting server on :8765 (password: ${APP_PASSWORD})..."
cd /app
APP_PASSWORD="${APP_PASSWORD}" node server.mjs &
PROXY_PID=$!

# ── Optional Cloudflare Tunnel ──
if [ "${TUNNEL}" = "1" ]; then
  echo "[+] Starting Cloudflare Tunnel..."
  cloudflared tunnel --url http://localhost:8765 > /tmp/tunnel-web.log 2>&1 &
  TUNNEL_PID=$!

  # Separate tunnel for native gRPC (Python/ADBC) — gets its own URL
  cloudflared tunnel --url http://localhost:50051 > /tmp/tunnel-grpc.log 2>&1 &

  # Poll for the web URL
  TUNNEL_URL=""
  TUNNEL_GRPC=""
  for i in $(seq 1 30); do
    TUNNEL_URL=$(grep -oP 'https://[^.]*\.trycloudflare\.com' /tmp/tunnel-web.log 2>/dev/null | head -1)
    TUNNEL_GRPC=$(grep -oP 'https://[^.]*\.trycloudflare\.com' /tmp/tunnel-grpc.log 2>/dev/null | head -1)
    if [ -n "$TUNNEL_URL" ] && [ -n "$TUNNEL_GRPC" ]; then break; fi
    sleep 0.5
  done
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  App:     http://localhost:8765"
echo "  Auth:    password = ${APP_PASSWORD}"
if [ -n "${TUNNEL_URL:-}" ]; then
  echo "  Web:     $TUNNEL_URL"
fi
if [ -n "${TUNNEL_GRPC:-}" ]; then
  echo "  gRPC:    $TUNNEL_GRPC  (Python/ADBC)"
fi
if [ -z "${TUNNEL_URL:-}" ] && [ "${TUNNEL}" = "1" ]; then
  echo "  Tunnel:  starting (check docker logs)"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ── Wait for any child to exit ──
wait -n $FLIGHT_PID $PROXY_PID ${TUNNEL_PID:-}
