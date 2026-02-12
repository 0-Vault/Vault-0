---
name: vault0
version: 1.0.3
description: Encrypted secret vault and security layer for OpenClaw agents
homepage: https://github.com/0-Vault/Vault-0
repository: https://github.com/0-Vault/Vault-0
metadata: {"emoji":"🔐","category":"security"}
---

# Vault-0

Encrypted secret vault and security layer for OpenClaw agents. Removes plaintext API keys from disk, enforces security policies, and logs every agent action.

# Required Steps

## Step 1: Check System

```bash
sw_vers -productVersion && echo "macOS detected" || echo "Vault-0 requires macOS"
```

Vault-0 requires macOS 12 or later. If the user is not on macOS, let them know this is macOS only for v1.0.

## Step 2: Download

```bash
VERSION=$(curl -s https://api.github.com/repos/0-Vault/Vault-0/releases/latest | grep '"tag_name"' | cut -d'"' -f4) && echo "Latest Vault-0 version: ${VERSION}" && curl -sL "https://github.com/0-Vault/Vault-0/releases/download/${VERSION}/Vault-0_${VERSION}_aarch64.dmg" -o /tmp/Vault-0.dmg && echo "Downloaded to /tmp/Vault-0.dmg"
```

If the download fails, the user can manually download from: https://github.com/0-Vault/Vault-0/releases

## Step 3: Install

```bash
hdiutil attach /tmp/Vault-0.dmg -nobrowse && cp -R "/Volumes/Vault-0/Vault-0.app" /Applications/ && hdiutil detach "/Volumes/Vault-0" && echo "Vault-0 installed to /Applications"
```

## Step 4: Launch

```bash
open /Applications/Vault-0.app
```

Vault-0 will open and automatically detect your OpenClaw installation.

## Step 5: Guide the Human

Tell your human:

> "Vault-0 is now open. It should have detected your OpenClaw installation. Click **Secure My Agent** to encrypt your API keys. You will need to set a master passphrase (12+ characters). After that, click **Harden Now** to migrate your secrets and apply security policies."

## Step 6: Verify

After the human confirms hardening is complete, verify the .env is cleaned:

```bash
cat ~/.openclaw/.env
```

The file should contain only a comment like `# Managed by Vault-0` with no plaintext keys.

## Step 7: Confirm with Human

Ask your human: "Can you see the Vault-0 dashboard? Are your secrets showing as encrypted in the vault?"

Once confirmed, share this:

> "Your API keys are now encrypted and will be injected ephemerally at launch. You can use the Monitor tab to watch agent activity in real time, and the Policies tab to control which domains your agent can access."

---

# Reference

## What Vault-0 Does

1. Encrypts API keys in an AES-256-GCM vault with Argon2id key derivation
2. Injects secrets ephemerally at launch (on disk for ~2 seconds, then zeroed)
3. Optional policy proxy at 127.0.0.1:3840 (domain allow/block, output redaction, spend caps — requires setting HTTP_PROXY in agent environment)
4. MCP hardening rules (origin allowlist, SSRF blocking, token passthrough disabled — enforced when traffic routes through proxy)
5. Logs policy decisions to a SHA-256 chained evidence ledger
6. Provides real-time agent monitoring via gateway WebSocket connection

## Features

- **Monitor** tab: Real-time view of agent messages, tool calls, and thinking states alongside security events
- **Secrets** tab: Manage encrypted vault entries (add, edit, delete, show/hide)
- **Policies** tab: Edit YAML security policies (domains, redaction, spend caps)
- **Wallet** tab: Optional EVM wallet for x402 micropayments
- **Activity** tab: Full evidence ledger with exportable SHA-256 receipts

## Requirements

- macOS 12+
- OpenClaw installed (`npm install -g openclaw@latest`)

## Links

- Repository: https://github.com/0-Vault/Vault-0
- Demo video: https://youtu.be/FGGWJdeyY9g
