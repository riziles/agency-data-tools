#!/usr/bin/env bash
# Batch download + convert Fannie Mae quarters 2000-2017 (72 quarters)
# Run from project root. Uses cached ZIPs/CSVs to avoid re-download.
set -e
cd "$(dirname "$0")"

INGEST="cargo run --manifest-path ingest/Cargo.toml --"
mkdir -p zip-backup test-data

source .env 2>/dev/null || true

refresh_token() {
  TOKEN=$(curl -s -u "${FANNIE_CLIENT_ID}:${FANNIE_CLIENT_SECRET}" \
    "https://auth.pingone.com/4c2b23f9-52b1-4f8f-aa1f-1d477590770c/as/token" \
    -d "grant_type=client_credentials" \
    -H "Content-Type: application/x-www-form-urlencoded" | python3 -c "import sys,json; print(json.load(sys.stdin)['access_token'])")
  export FANNIE_TOKEN="$TOKEN"
}
refresh_token

COUNT=0
for year in $(seq 2000 2017); do
  for q in Q1 Q2 Q3 Q4; do
    OUTPUT="test-data/${year}${q}.parquet"
    if [ -f "$OUTPUT" ]; then
      echo "[skip] ${year} ${q} — already done"
      continue
    fi
    
    echo ""
    echo "━━━ ${year} ${q} ━━━"
    $INGEST --year "$year" --quarter "$q" --output "$OUTPUT" 2>&1
    
    # Clean up the huge CSV
    rm -f "fannie_${year}_${q}.csv"
    
    echo "  ✓ ${year} ${q} → $OUTPUT"
    echo "  Waiting 120s..."
    sleep 120
    
    COUNT=$((COUNT + 1))
    # Refresh token every 6 quarters (token lasts 1 hour)
    if [ $((COUNT % 6)) -eq 0 ]; then
      refresh_token
    fi
  done
done

echo ""
echo "All done! $(ls test-data/[12]*.parquet | wc -l) quarters total."