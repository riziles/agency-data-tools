#!/usr/bin/env bash
# Start the Fannie Mae Flight SQL server in Docker
# Usage: ./docker-up.sh [password] [--tunnel]
#   password  - APP_PASSWORD (default: demo)
#   --tunnel  - enable Cloudflare Tunnel for public access

set -e
cd "$(dirname "$0")"

PASSWORD="${APP_PASSWORD:-demo}"
TUNNEL=0

for arg in "$@"; do
  case "$arg" in
    --tunnel|-t) TUNNEL=1 ;;
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

docker run -d --name fannie-flight \
  -p 8765:8765 \
  -v "$DATA_DIR:/app/data" \
  -e APP_PASSWORD="$PASSWORD" \
  -e TUNNEL="$TUNNEL" \
  fannie-flight:latest

sleep 3
echo ""
echo "Ready: http://localhost:8765"
docker logs fannie-flight 2>&1 | grep -E "Serving|Tunnel|snapshot" || true
