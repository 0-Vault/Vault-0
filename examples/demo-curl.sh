#!/usr/bin/env sh
# Demo: send a request through Vault-0 proxy (must be running).
# 1. Start Vault-0 app and click Start proxy.
# 2. Run: ./examples/demo-curl.sh

set -e
PROXY="${VAULT0_PROXY:-http://127.0.0.1:3840}"

echo "Request through Vault-0 proxy (blocked domain will appear in Evidence)..."
curl -s -x "$PROXY" -o /dev/null -w "%{http_code}" "https://internal.company.local/" || true
echo ""

echo "Request to allowed domain (if you added openai secret, this would inject it)..."
curl -s -x "$PROXY" -o /dev/null -w "%{http_code}\n" "https://api.openai.com/v1/models" || true

echo "Check the Dashboard live feed and Evidence page for the log."
