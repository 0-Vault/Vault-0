# Launch assets

**Tagline:** The secure desktop dashboard for autonomous AI agents.

**One-liner:** Install in minutes, protect existing agents with zero plaintext secrets, policy-controlled actions, and native x402 payments. Live evidence log and one-click receipt export.

**Demo script (1 min):**
1. Show onboarding, add a secret alias, pick a policy.
2. Start proxy; run a request through it (e.g. `examples/demo-curl.sh`).
3. Show Dashboard live feed (allowed/blocked), then Evidence page and Export receipt.

**Screenshots to capture:** Dashboard with proxy running and event feed; Evidence page with receipt; Policies page with YAML.

**KPIs (instrumentation):** `get_evidence_stats` returns allowed/blocked/payment counts. First protected run = first non-zero allowed count. Activation = proxy started and at least one request proxied.
