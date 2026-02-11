# Vault-0 Security Model

1. **Secretless execution** – Credentials are stored in the local vault and injected by the proxy at request time. Agent code never receives plaintext secrets.
2. **Policy engine** – Outbound requests are checked against allow/block domain lists, method, and (optionally) spend caps. Response bodies can be redacted by configurable patterns (e.g. API key patterns).
3. **MCP hardening** – For requests classified as MCP: origin allowlist is enforced, SSRF (private/internal IP) targets are blocked, and token passthrough is disabled so the proxy does not forward client-supplied tokens to upstream.
4. **Evidence ledger** – Every allowed, blocked, and 402 event is appended to a tamper-evident log. Export produces entries with SHA-256 hashes for verification.

## Threat model

- Local first: vault and logs stay on the machine. No cloud dependency in v1.0.
- Proxy is the single egress for agent HTTP traffic when configured (e.g. HTTP_PROXY). Bypass requires the agent to ignore proxy settings.
