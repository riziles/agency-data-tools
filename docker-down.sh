#!/usr/bin/env bash
# Stop the Fannie Mae Flight SQL server and dashboard
set -e

docker rm -f fannie-flight 2>/dev/null && echo "Server stopped." || echo "Server not running."

# Clean up any standalone Flight SQL / Node proxy processes
fuser -k 50051/tcp 2>/dev/null || true
fuser -k 8765/tcp 2>/dev/null || true

# Kill Vite dev server and all related processes
pkill -9 -f "vite.*5173" 2>/dev/null || true
# Also kill anything still on the port
fuser -k 5173/tcp 2>/dev/null || true

echo "Dashboard stopped."
