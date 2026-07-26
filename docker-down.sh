#!/usr/bin/env bash
# Stop the Fannie Mae Flight SQL server
set -e
docker rm -f fannie-flight 2>/dev/null && echo "Stopped." || echo "Not running."
