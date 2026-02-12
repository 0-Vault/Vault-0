# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take the security of Vault-0 seriously. If you discover a security vulnerability, please do **not** open a public issue.

Instead, please report it privately via email to: **security@vault0.org** (or replace with your actual contact method).

We will acknowledge your report within 48 hours and provide an estimated timeline for a fix.

## Security Model

Vault-0 is designed to protect API keys and sensitive data for AI agents.

- **Storage:** Secrets are encrypted using AES-256-GCM with a key derived from your master passphrase using Argon2id.
- **Memory:** Secrets are decrypted in memory only when needed and injected into the target process environment.
- **Network:** The local proxy (127.0.0.1:3840) intercepts outbound traffic to enforce policies. It does not inspect traffic content beyond what is necessary for policy enforcement (e.g., headers, body for redaction).
- **Updates:** We recommend always running the latest version to ensure you have the latest security patches.

## Known Limitations

- **Ephemeral .env:** During the launch process, secrets exist in plaintext in `~/.openclaw/.env` for approximately 2 seconds before being overwritten. This is a trade-off to support the OpenClaw daemon without modifying its source code.
- **Memory Dump:** A sophisticated attacker with root access who can dump the RAM of the running process could potentially retrieve decrypted keys.
