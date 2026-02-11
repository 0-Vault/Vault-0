# Vault-0 Quickstart

1. Download and install the app (or run `npm run tauri dev` from the repo).
2. Complete onboarding: add secrets as aliases, pick a policy template (e.g. OpenClaw Safe).
3. Use the dashboard to monitor agent activity; launch agents from the app for auto-proxy injection.
4. Export evidence receipts from the Evidence page when needed.

## Proxy

Route agent HTTP traffic through the local proxy so credentials are injected at runtime and never appear in agent memory. Configure `HTTP_PROXY` / `HTTPS_PROXY` to point at the Vault-0 proxy (default port TBD in implementation).
