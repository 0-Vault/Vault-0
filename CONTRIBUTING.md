# Contributing to Vault-0

## Getting Started

1. Fork the repo and clone your fork:

```bash
git clone https://github.com/<your-username>/Vault-0.git
cd Vault-0
```

2. Install dependencies:

```bash
npm install
```

3. Run in development mode:

```bash
cargo tauri dev
```

Requires Rust 1.75+, Node 18+, and Xcode Command Line Tools on macOS.

## Making Changes

1. Create a branch from `main`:

```bash
git checkout -b your-branch-name
```

2. Keep files under 600 lines. Split into modules if needed.
3. Run `cargo clippy` and `cargo fmt` before committing.
4. Test your changes locally with `cargo tauri dev`.

## Pull Requests

1. One PR per feature or fix. Keep scope small.
2. Write a clear title describing the change (e.g., "Add per-request spend tracking to proxy handler").
3. Include a brief description of what changed and why.
4. Link any related issue (e.g., "Closes #12").

## What to Work On

Check the [open issues](https://github.com/0-Vault/Vault-0/issues) for tasks labeled `good first issue` or `help wanted`. Current areas that need contribution:

1. **Per-request spend tracking** — enforce `spend_cap_cents` across all proxied requests, not just x402 settlements.
2. **Wallet balance fetching** — replace the stubbed `get_wallet_balance()` with real RPC calls to fetch on-chain USDC balance.
3. **Payment history persistence** — store settled x402 payments to disk and return them from `get_payment_history()`.
4. **Linux support** — replace macOS Keychain calls with a cross-platform secret storage backend.
5. **Additional agent frameworks** — extend the detection/migration flow beyond OpenClaw.

## Reporting Issues

Open an issue with:

1. What you expected to happen.
2. What actually happened.
3. Steps to reproduce.
4. macOS version and architecture (Apple Silicon or Intel).

## Code of Conduct

Be respectful. No harassment, no trolling. Focus on the code.
