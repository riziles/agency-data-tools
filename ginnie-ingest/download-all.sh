#!/usr/bin/env bash
# Download Ginnie Mae Loan Performance files via Playwright (needs live browser session)
# then convert each to Parquet using ginnie-ingest.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INGEST="$SCRIPT_DIR/ginnie-ingest/target/debug/ginnie-ingest"
OUTDIR="$SCRIPT_DIR/test-data"

mkdir -p "$OUTDIR"

# All available quarterly files (YYYYMM format)
# Source: https://www.ginniemae.gov/disclosure/disclosure-data/disclosure-data-history/
QUARTERS=(
  "202306"  # 2023 Q2
  "202309"  # 2023 Q3
  "202312"  # 2023 Q4
  "202403"  # 2024 Q1
  "202406"  # 2024 Q2
  "202409"  # 2024 Q3
  "202412"  # 2024 Q4
  "202503"  # 2025 Q1
  "202506"  # 2025 Q2
  "202509"  # 2025 Q3
  "202512"  # 2025 Q4
  "202603"  # 2026 Q1
)

download_via_playwright() {
  local YYYYMM="$1"
  local ZIPFILE="LoanPerf_${YYYYMM}.zip"
  local DEST="$SCRIPT_DIR/$ZIPFILE"
  
  if [ -f "$DEST" ]; then
    echo "  [cached] $ZIPFILE"
    return 0
  fi
  
  echo "  Downloading $ZIPFILE via browser..."
  
  # Use playwright-cli eval to fetch and save via downloads
  # We'll use CDP to set download path first
  playwright-cli --raw eval "
    (async () => {
      // Trigger download via fetch + blob + anchor click
      const url = 'https://www.ginniemae.gov/disclosure-api/api/download?dlfile=data_history_cons%2F${ZIPFILE}';
      const res = await fetch(url, { credentials: 'include' });
      if (!res.ok) return 'HTTP ' + res.status;
      
      const blob = await res.blob();
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = '${ZIPFILE}';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(a.href), 5000);
      
      return 'downloaded ' + blob.size + ' bytes';
    })()
  " 2>&1
  
  echo "  Waiting for browser download to complete..."
  # File will go to ~/Downloads — move it
  sleep 10
  if [ -f "$HOME/Downloads/$ZIPFILE" ]; then
    mv "$HOME/Downloads/$ZIPFILE" "$DEST"
    echo "  Saved $ZIPFILE ($(du -h "$DEST" | cut -f1))"
  else
    echo "  ERROR: download not found in ~/Downloads/"
    return 1
  fi
}

for YYYYMM in "${QUARTERS[@]}"; do
  YEAR="${YYYYMM:0:4}"
  MONTH="${YYYYMM:4:2}"
  case "$MONTH" in
    03) Q="Q1" ;;
    06) Q="Q2" ;;
    09) Q="Q3" ;;
    12) Q="Q4" ;;
  esac
  
  PARQUET="$OUTDIR/ginnie_${YEAR}${Q}.parquet"
  if [ -f "$PARQUET" ]; then
    echo "━━━ ${YEAR} ${Q} — already done, skipping ━━━"
    continue
  fi
  
  echo ""
  echo "━━━ ${YEAR} ${Q} ━━━"
  
  download_via_playwright "$YYYYMM" || {
    echo "Download failed, stopping."
    exit 1
  }
  
  # Wait between downloads
  echo "  Waiting 120s..."
  sleep 120
  
  # Convert
  echo "  Converting to Parquet..."
  "$INGEST" --year "$YEAR" --quarter "$Q" --output "$PARQUET" -c /dev/null 2>&1 || {
    echo "Conversion failed for ${YEAR} ${Q}"
    exit 1
  }
  
  # Clean up the 7+ GB extracted text file
  rm -f "LoanPerf_${YYYYMM}.txt" "LoanPerf_${YYYYMM}.lp.csv"
  
  echo "  ✓ ${YEAR} ${Q} → $PARQUET ($(du -h "$PARQUET" | cut -f1))"
done

echo ""
echo "All done!"
ls -lh "$OUTDIR"/ginnie_*.parquet