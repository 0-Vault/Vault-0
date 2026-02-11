# Vault-0 Launch Thread

> **Format:** Twitter/X thread. Pin tweet 1. Attach demo video to tweet 1.

---

## Tweet 1 (Video Tweet)

> **[Attach: demo video showing guided setup, proxy injection, dashboard]**

Introducing **Vault-0** -- the open-source security layer for @OpenClaw agents.

Zero plaintext API keys. Policy-controlled actions. Native x402 payments. Tamper-evident audit trail.

Your ClawBot shouldn't touch raw secrets. Now it doesn't have to.

Thread

---

## Tweet 2

The problem: every AI agent framework stores API keys in `.env` files or config YAML. One leaked log, one bad plugin, one prompt injection -- and your keys are gone.

Vault-0 eliminates this entirely. Agents never see your secrets. Period.

---

## Tweet 3

How it works:

1. Secrets live in an AES-256-GCM encrypted vault (Argon2id key derivation)
2. A local proxy at `127.0.0.1:3840` injects credentials per-request
3. Your ClawBot uses aliases like `VAULT0_ALIAS:openai` -- never real keys
4. The proxy handles the swap at the network layer, in Rust

---

## Tweet 4

Policy engine gives you granular control over what your agent can do:

1. Domain allow/block lists
2. HTTP method restrictions
3. Spend caps per session
4. Output redaction (API keys never leak into logs)
5. Auto-settle rules for x402 payments

YAML-based. Auditable. No surprises.

---

## Tweet 5

Built-in x402 payment support.

When an API returns HTTP 402 Payment Required, Vault-0 handles it natively:

1. Parses the 402 response (amount, recipient, network)
2. Signs via EIP-3009 with your on-device EVM wallet
3. Auto-settles if policy allows
4. Logs everything to the evidence ledger

Machine-to-machine payments, handled.

---

## Tweet 6

MCP hardening out of the box:

1. Allowlisted MCP server origins only
2. SSRF mitigation (private/internal IPs blocked)
3. Token passthrough disabled -- the proxy never forwards your client tokens to MCP servers

If you're running MCP tools with your ClawBot, this matters.

---

## Tweet 7

The evidence ledger is a SHA-256 chained log of every proxied request, policy decision, and payment.

Export tamper-evident receipts. Know exactly what your agent did, when, and why it was allowed.

Full accountability for autonomous agents.

---

## Tweet 8

The guided setup flow:

1. Launch Vault-0 -- it detects your OpenClaw installation automatically
2. Scans for plaintext keys in existing configs
3. Migrates them into the encrypted vault
4. Hardens your ClawBot with secure aliases and policies
5. Launches through the proxy with a single click

Zero-to-secure in under 2 minutes.

---

## Tweet 9

Tech stack for the curious:

1. **Tauri 2** + **Svelte** + **Tailwind** (frontend)
2. **Axum** proxy, **alloy** EVM signer, **argon2/aes-gcm** vault (Rust backend)
3. **macOS Keychain** for wallet storage -- private key never touches the webview
4. Embedded terminal via xterm.js + tauri-pty

Native performance. No Electron. ~15MB binary.

---

## Tweet 10

Vault-0 is open source and built for the OpenClaw ecosystem.

If you're running ClawBot agents and want to stop leaving API keys in plaintext, this is for you.

Star the repo. Try it. Break it. PRs welcome.

[REPO LINK]

---

## Suggested Hashtags

`#OpenClaw` `#ClawBot` `#AI` `#Agents` `#OpenSource` `#Tauri` `#Rust` `#x402` `#InfoSec` `#Web3`
