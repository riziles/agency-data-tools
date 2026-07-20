#!/bin/bash
# Read secrets from YAML and create .env file
# Usage: ./scripts/setup-env.sh

set -e

SECRETS_FILE="secrets/datadynamics.yaml"
ENV_FILE=".env"

if [ ! -f "$SECRETS_FILE" ]; then
    echo "Error: $SECRETS_FILE not found"
    exit 1
fi

# Extract values from YAML using grep/awk (no yaml parser needed for flat format)
CLIENT_ID=$(grep 'Client ID:' "$SECRETS_FILE" | awk '{print $3}')
CLIENT_SECRET=$(grep 'Client Secret:' "$SECRETS_FILE" | awk '{print $3}')

if [ -z "$CLIENT_ID" ] || [ -z "$CLIENT_SECRET" ]; then
    echo "Error: Could not extract credentials from $SECRETS_FILE"
    exit 1
fi

cat > "$ENV_FILE" << EOF
# Fannie Mae Developer Portal credentials
FANNIE_CLIENT_ID=$CLIENT_ID
FANNIE_CLIENT_SECRET=$CLIENT_SECRET

# Cloudflare R2 (generate via: pnpm wrangler r2 bucket list)
# Then create an R2 API token:
# pnpm wrangler r2 bucket create fannie-mae-poc
R2_ACCOUNT_ID=1c4f9d44ccb174100f3d15ee3528f166
R2_ACCESS_KEY_ID=
R2_SECRET_ACCESS_KEY=
EOF

echo "✅ Created $ENV_FILE with Fannie Mae credentials"
echo ""
echo "⚠️  R2 credentials not set. To generate them:"
echo "   1. Go to https://dash.cloudflare.com/?to=/:account/r2/api-tokens"
echo "   2. Create an API token with read/write access to 'fannie-mae-poc'"
echo "   3. Add the values to .env"
