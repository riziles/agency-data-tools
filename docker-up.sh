#!/usr/bin/env bash
# Start the Fannie Mae Flight SQL server
# Usage: ./docker-up.sh [password] [--tunnel] [--dashboard]
#   password    - APP_PASSWORD (default: demo)
#   --tunnel    - enable Cloudflare Tunnel for public access
#   --dashboard - also start the SvelteKit dashboard on :5173

set -e
cd "$(dirname "$0")"

PASSWORD="${APP_PASSWORD:-demo}"
TUNNEL=0
DASHBOARD=0

for arg in "$@"; do
  case "$arg" in
    --tunnel|-t) TUNNEL=1 ;;
    --dashboard|-d) DASHBOARD=1 ;;
    *) PASSWORD="$arg" ;;
  esac
done

DATA_DIR="$(pwd)/flight-server/data"
if [ ! -f "$DATA_DIR/catalog.db" ]; then
  echo "⚠️  No catalog.db found at $DATA_DIR"
  echo "   Run add-quarter first to load some data, or create an empty catalog."
  exit 1
fi

echo "Starting fannie-flight on http://localhost:8765 (password: $PASSWORD)"
if [ "$TUNNEL" = "1" ]; then
  echo "Cloudflare Tunnel: enabled"
fi

docker rm -f fannie-flight 2>/dev/null || true

PUBLIC_DIR="$(pwd)/flight-server/public"

docker run -d --name fannie-flight \
  -p 8765:8765 \
  -v "$DATA_DIR:/app/data" \
  -v "$PUBLIC_DIR:/app/public" \
  -e APP_PASSWORD="$PASSWORD" \
  -e TUNNEL="$TUNNEL" \
  fannie-flight:latest

sleep 3
echo ""
echo "Ready: http://localhost:8765"
docker logs fannie-flight 2>&1 | grep -E "Serving|Tunnel|snapshot" || true

# ── Dashboard (build & serve via proxy, or dev mode) ──
if [ "$DASHBOARD" = "1" ]; then
  if [ ! -d "$(pwd)/dashboard/node_modules" ]; then
    echo ""
    echo "📦 Installing dashboard dependencies..."
    (cd dashboard && pnpm install)
  fi

  # Build static and copy into the proxy's public dir so it's served on :8765
  echo ""
  echo "🏗  Building dashboard..."
  (cd dashboard && pnpm build)
  rm -rf flight-server/public/_app flight-server/public/index.html
  cp -r dashboard/build/* flight-server/public/
  echo "   Copied dashboard build → flight-server/public/"

  echo ""
  echo "🎛  Starting dashboard dev on http://localhost:5173"
  cd dashboard && pnpm dev &
  DASHBOARD_PID=$!
  echo "   Dashboard PID: $DASHBOARD_PID"
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  App:       http://localhost:8765"
  echo "  Dashboard: http://localhost:5173 (dev)"
  echo "             http://localhost:8765 (static, via tunnel)"
  echo "  Auth:      password = $PASSWORD"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
fi
