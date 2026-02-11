# Vault-0 v1.0 Scope Lock

Frozen for 12-week launch window. Do not add scope without explicit plan update.

## In Scope (Eight Core Modules)

1. **Secretless Proxy** — Alias-based credential injection; zero plaintext in agent runtime.
2. **Policy Engine** — Allow/deny domains, method controls, spend caps, output redaction rules.
3. **MCP Hardening** — Allowlisted MCP servers, token audience checks, SSRF blocking, no token passthrough.
4. **x402 Payments** — 402 detection, EIP-3009 signing (alloy-signer-local), policy-gated auto-settlement with X-PAYMENT header.
5. **Evidence Ledger** — Tamper-evident event hashing, exportable receipts.
6. **Agent Wallet** — BIP-39 mnemonic via alloy, macOS Keychain storage, real EVM addresses. Private key isolated in Rust process.
7. **Embedded Terminal** — xterm.js + tauri-plugin-pty. Auto-install OpenClaw, run ClawBot wizard, launch bots.
8. **Desktop UX** — Welcome flow with OpenClaw detection, helper-only wizard bridge (real OpenClaw helper in PTY), readiness-gated launch, live session trace dashboard, wallet sidebar, share proof.

## Target Platform

macOS only (v1.0). Apple Silicon and Intel supported. Keychain integration requires macOS 12+.

## Primary User Flow

1. Launch — detect existing OpenClaw or fresh install via embedded terminal.
2. Wizard bridge — collect secure inputs, run real OpenClaw helper, inject deterministic answers in PTY.
3. Run — bot launches through proxy; readiness gate must pass (status command then HTTP probe) before dashboard transition.
4. Share — proof receipt with event counts, wallet address, post to X.

## Explicit Non-Goals (v1.0)

1. No cloud sync, team workspaces, or multi-user auth.
2. No Windows or Linux support.
3. No enterprise compliance pack.
4. No advanced autonomous multi-agent orchestration.
5. No on-chain transaction submission (signing only; settlement deferred to x402 facilitators).

## Cut Criteria

If timeline slips, preserve in order: proxy + policy + evidence + dashboard. Defer terminal auto-install polish, share proof, and optional wallet.
