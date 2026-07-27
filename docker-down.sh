#!/usr/bin/env bash
# Stop the Fannie Mae Flight SQL server and dashboard
set -e
docker rm -f fannie-flight 2>/dev/null && echo "Server stopped." || echo "Server not running."

# Kill dashboard dev server if running
DASHBOARD_PID=$(pgrep -f "vite dev.*dashboard" 2>/dev/null || true)
if [ -n "$DASHBOARD_PID" ]; then
  kill $DASHBOARD_PID 2>/dev/null && echo "Dashboard stopped." || true
fi
