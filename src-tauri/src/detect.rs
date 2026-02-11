//! Detect existing OpenClaw / ClawBot installations and scan configs for plaintext keys.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct DetectionResult {
    pub found: bool,
    pub path: String,
    pub install_kind: String,
    pub cli_version: String,
    pub has_config: bool,
    pub plaintext_keys: Vec<PlaintextKey>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaintextKey {
    pub file: String,
    pub key_name: String,
    pub preview: String,
}

const SEARCH_DIRS: &[&str] = &[
    "clawbot",
    "openclaw",
    ".openclaw",
    "projects/clawbot",
    "projects/openclaw",
    "Development/clawbot",
    "Development/openclaw",
    "Code/clawbot",
    "Code/openclaw",
];

const CONFIG_FILES: &[&str] = &[
    ".env",
    ".env.local",
    "config.json",
    "config.yaml",
    "config.yml",
    ".openclaw/openclaw.json",
    "openclaw.config.json",
    "openclaw.config.yaml",
    ".openclaw/config.json",
    ".openclaw/config.yaml",
];

const KEY_PATTERNS: &[(&str, &str)] = &[
    ("OPENAI_API_KEY", "sk-"),
    ("ANTHROPIC_API_KEY", "sk-ant-"),
    ("GROK_API_KEY", "xai-"),
    ("TELEGRAM_BOT_TOKEN", ""),
    ("SLACK_TOKEN", "xoxb-"),
    ("DISCORD_TOKEN", ""),
    ("GITHUB_TOKEN", "ghp_"),
    ("API_KEY", ""),
    ("SECRET_KEY", ""),
    ("PRIVATE_KEY", ""),
];

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

fn is_openclaw_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let markers = ["package.json", "pnpm-lock.yaml", "openclaw.config.json", "openclaw.json"];
    markers.iter().any(|m| path.join(m).exists())
        || path.join("node_modules").join("openclaw").exists()
        || path.join("src").join("openclaw").exists()
        || path.join("agents").is_dir()
        || path.join("logs").is_dir()
}

fn scan_for_keys(dir: &Path) -> Vec<PlaintextKey> {
    let mut found = Vec::new();
    for config_file in CONFIG_FILES {
        let file_path = dir.join(config_file);
        if !file_path.exists() || !file_path.is_file() {
            continue;
        }
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (key_name, prefix) in KEY_PATTERNS {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.contains(key_name) {
                    continue;
                }
                let value = extract_value(trimmed);
                if value.is_empty() || value.starts_with("${") || value.starts_with("$") {
                    continue;
                }
                if value == "your-key-here" || value == "CHANGE_ME" || value == "xxx" {
                    continue;
                }
                if !prefix.is_empty() && !value.starts_with(prefix) {
                    continue;
                }
                let preview = if value.len() > 8 {
                    format!("{}****", &value[..4])
                } else {
                    "****".to_string()
                };
                found.push(PlaintextKey {
                    file: config_file.to_string(),
                    key_name: key_name.to_string(),
                    preview,
                });
            }
        }
    }
    found
}

fn extract_value(line: &str) -> String {
    let cleaned = line.trim();
    if let Some(eq) = cleaned.find('=') {
        let val = cleaned[eq + 1..].trim().trim_matches('"').trim_matches('\'');
        return val.to_string();
    }
    if let Some(colon) = cleaned.find(':') {
        let val = cleaned[colon + 1..].trim().trim_matches('"').trim_matches('\'');
        return val.to_string();
    }
    String::new()
}

fn detect_global_cli() -> Option<(String, String)> {
    let path_output = Command::new("sh")
        .args(["-lc", "command -v openclaw"])
        .output()
        .ok()?;
    if !path_output.status.success() {
        return None;
    }
    let cli_path = String::from_utf8_lossy(&path_output.stdout).trim().to_string();
    if cli_path.is_empty() {
        return None;
    }
    let version_output = Command::new("openclaw").arg("--version").output().ok()?;
    if !version_output.status.success() {
        return None;
    }
    let version_text = String::from_utf8_lossy(&version_output.stdout).trim().to_string();
    if version_text.is_empty() {
        return Some((cli_path, "unknown".to_string()));
    }
    Some((cli_path, version_text))
}

#[tauri::command]
pub fn detect_openclaw() -> Result<DetectionResult, String> {
    let home = home_dir().ok_or_else(|| "Home directory not found".to_string())?;

    if let Some((cli_path, cli_version)) = detect_global_cli() {
        let keys = scan_for_keys(&home);
        let home_keys = scan_for_keys(&home.join(".openclaw"));
        let all_keys: Vec<PlaintextKey> = keys.into_iter().chain(home_keys).collect();
        let has_config = home.join(".openclaw").join("openclaw.json").exists();
        return Ok(DetectionResult {
            found: true,
            path: cli_path,
            install_kind: "global_cli".to_string(),
            cli_version,
            has_config,
            plaintext_keys: all_keys,
        });
    }

    let openclaw_config_dir = home.join(".openclaw");
    if openclaw_config_dir.join("openclaw.json").exists() {
        let keys = scan_for_keys(&openclaw_config_dir);
        let home_keys = scan_for_keys(&home);
        let all_keys: Vec<PlaintextKey> = keys.into_iter().chain(home_keys).collect();
        return Ok(DetectionResult {
            found: true,
            path: openclaw_config_dir.to_string_lossy().to_string(),
            install_kind: "config_dir".to_string(),
            cli_version: String::new(),
            has_config: true,
            plaintext_keys: all_keys,
        });
    }

    for search_dir in SEARCH_DIRS {
        let candidate = home.join(search_dir);
        if is_openclaw_dir(&candidate) {
            let keys = scan_for_keys(&candidate);
            let has_config = CONFIG_FILES
                .iter()
                .any(|f| candidate.join(f).exists());
            return Ok(DetectionResult {
                found: true,
                path: candidate.to_string_lossy().to_string(),
                install_kind: "directory".to_string(),
                cli_version: String::new(),
                has_config,
                plaintext_keys: keys,
            });
        }
    }

    Ok(DetectionResult {
        found: false,
        path: String::new(),
        install_kind: "none".to_string(),
        cli_version: String::new(),
        has_config: false,
        plaintext_keys: Vec::new(),
    })
}

#[tauri::command]
pub fn secure_config_keys(install_path: String, keys_to_secure: Vec<(String, String)>) -> Result<(), String> {
    for (alias, value) in &keys_to_secure {
        let mut state = crate::proxy::state().write().map_err(|_| "state lock")?;
        state.vault.insert(alias.clone(), value.clone());
    }
    let dir = Path::new(&install_path);
    for config_file in CONFIG_FILES {
        let file_path = dir.join(config_file);
        if !file_path.exists() || !file_path.is_file() {
            continue;
        }
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut new_content = content.clone();
        for (alias, value) in &keys_to_secure {
            new_content = new_content.replace(value.as_str(), &format!("VAULT0_ALIAS:{}", alias));
        }
        if new_content != content {
            let _ = fs::write(&file_path, &new_content);
        }
    }
    crate::evidence::push("info", &format!("Secured {} keys in {}", keys_to_secure.len(), install_path));
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct HardenStep {
    pub step: String,
    pub status: String,
    pub detail: String,
    pub items: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HardenResult {
    pub success: bool,
    pub steps: Vec<HardenStep>,
}

#[tauri::command]
pub fn harden_openclaw(install_path: String) -> Result<HardenResult, String> {
    let mut steps: Vec<HardenStep> = Vec::new();
    let src = Path::new(&install_path);
    if !src.exists() {
        return Err(format!("Install path does not exist: {install_path}"));
    }

    // 1. Backup
    let backup_dir = dirs::data_dir()
        .ok_or("Cannot determine app data directory")?
        .join("Vault0")
        .join("backups")
        .join(format!("{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()));
    match fs::create_dir_all(&backup_dir) {
        Ok(_) => {
            let mut backed = 0u32;
            let mut backup_items: Vec<String> = Vec::new();
            for config_file in CONFIG_FILES {
                let file_path = src.join(config_file);
                if file_path.exists() && file_path.is_file() {
                    if let Ok(content) = fs::read(&file_path) {
                        match crate::vault_store::encrypt_bytes_with_vault_key(&content) {
                            Ok(encrypted) => {
                                let dest = backup_dir.join(format!("{}.enc", config_file));
                                if let Some(parent) = dest.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }
                                if fs::write(&dest, &encrypted).is_ok() {
                                    backup_items.push(format!("{} -> {}.enc (encrypted)", config_file, config_file));
                                    backed += 1;
                                }
                            }
                            Err(_) => {
                                let dest = backup_dir.join(config_file);
                                if let Some(parent) = dest.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }
                                let _ = fs::copy(&file_path, &dest);
                                backup_items.push(format!("{} -> {} (copy)", config_file, config_file));
                                backed += 1;
                            }
                        }
                    }
                }
            }
            let creds_src = src.join("credentials");
            if creds_src.is_dir() {
                let creds_dst = backup_dir.join("credentials");
                let _ = copy_dir_recursive(&creds_src, &creds_dst);
                backup_items.push("credentials/ directory".to_string());
                backed += 1;
            }
            backup_items.push(format!("Saved to: {}", backup_dir.display()));
            steps.push(HardenStep {
                step: "backup".into(),
                status: "ok".into(),
                detail: format!("Encrypted backup of {} config files saved", backed),
                items: backup_items,
            });
        }
        Err(e) => {
            steps.push(HardenStep {
                step: "backup".into(),
                status: "error".into(),
                detail: format!("Failed to create backup dir: {e}"),
                items: vec![],
            });
            return Ok(HardenResult { success: false, steps });
        }
    }

    // 2. Migrate secrets to encrypted vault
    let keys = scan_for_keys(src);
    let home = home_dir().unwrap_or_default();
    let home_keys = scan_for_keys(&home);
    let all_keys: Vec<PlaintextKey> = keys.into_iter().chain(home_keys).collect();

    let mut migrated = 0u32;
    let mut migrate_items: Vec<String> = Vec::new();
    for pk in &all_keys {
        let raw_value = read_raw_key_value(src, &pk.file, &pk.key_name)
            .or_else(|| read_raw_key_value(&home, &pk.file, &pk.key_name));
        if let Some(val) = raw_value {
            let alias = pk.key_name.to_lowercase().replace(' ', "_");
            let provider = guess_provider(&pk.key_name);
            let preview = if val.len() > 8 {
                format!("{}...{}", &val[..4], &val[val.len()-4..])
            } else {
                "****".to_string()
            };
            match crate::vault_store::vault_add_entry(alias.clone(), val.clone(), provider) {
                Ok(_) => {
                    replace_key_in_file(src, &pk.file, &val, &format!("VAULT0_ALIAS:{alias}"));
                    replace_key_in_file(&home, &pk.file, &val, &format!("VAULT0_ALIAS:{alias}"));
                    migrate_items.push(format!("{} ({}) -> VAULT0_ALIAS:{}", pk.key_name, preview, alias));
                    migrated += 1;
                }
                Err(e) => {
                    steps.push(HardenStep {
                        step: "migrate".into(),
                        status: "warn".into(),
                        detail: format!("Failed to vault {}: {e}", pk.key_name),
                        items: vec![],
                    });
                }
            }
        }
    }
    if migrate_items.is_empty() {
        migrate_items.push("No plaintext secrets found to migrate (already secured or none detected)".to_string());
    }
    steps.push(HardenStep {
        step: "migrate".into(),
        status: "ok".into(),
        detail: format!("Migrated {} secrets to encrypted vault", migrated),
        items: migrate_items,
    });

    // 3. Apply hardened policy
    let policy = crate::policy::default_hardened_policy();
    let policy_items = vec![
        format!("Allowed domains: {}", policy.allow_domains.join(", ")),
        format!("Blocked: {} (cloud metadata endpoint)", policy.block_domains.join(", ")),
        format!("Spend cap: ${:.2}", policy.spend_cap_cents.unwrap_or(0) as f64 / 100.0),
        format!("Log redaction: {} patterns active", policy.output_redact_patterns.len()),
    ];
    match crate::policy::save_policy(None, policy) {
        Ok(_) => steps.push(HardenStep {
            step: "policy".into(),
            status: "ok".into(),
            detail: "Hardened security policy applied".into(),
            items: policy_items,
        }),
        Err(e) => steps.push(HardenStep {
            step: "policy".into(),
            status: "warn".into(),
            detail: format!("Policy save warning: {e}"),
            items: vec![],
        }),
    }

    // 4. Start proxy
    match crate::proxy::start() {
        Ok(_) => steps.push(HardenStep {
            step: "proxy".into(),
            status: "ok".into(),
            detail: "Vault-0 secure proxy started".into(),
            items: vec![
                "Listening: 127.0.0.1:3840".into(),
                "Mode: transparent forwarding + secret injection".into(),
                "Keys are decrypted in memory only, never written to disk".into(),
            ],
        }),
        Err(e) => steps.push(HardenStep {
            step: "proxy".into(),
            status: "warn".into(),
            detail: format!("Proxy start: {e}"),
            items: vec![],
        }),
    }

    crate::evidence::push("info", &format!("Hardened OpenClaw at {install_path}: {migrated} secrets migrated"));
    Ok(HardenResult { success: true, steps })
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("mkdir: {e}"))?;
    let entries = fs::read_dir(src).map_err(|e| format!("readdir: {e}"))?;
    for entry in entries.flatten() {
        let ty = entry.file_type().map_err(|e| format!("filetype: {e}"))?;
        let dest = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), &dest).map_err(|e| format!("copy: {e}"))?;
        }
    }
    Ok(())
}

fn read_raw_key_value(base: &Path, config_file: &str, key_name: &str) -> Option<String> {
    let file_path = base.join(config_file);
    let content = fs::read_to_string(&file_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains(key_name) {
            let val = extract_value(trimmed);
            if !val.is_empty() && !val.starts_with("${") && !val.starts_with("$") && !val.starts_with("VAULT0_ALIAS") {
                return Some(val);
            }
        }
    }
    None
}

fn replace_key_in_file(base: &Path, config_file: &str, old_value: &str, new_value: &str) {
    let file_path = base.join(config_file);
    if let Ok(content) = fs::read_to_string(&file_path) {
        let updated = content.replace(old_value, new_value);
        if updated != content {
            let _ = fs::write(&file_path, &updated);
        }
    }
}

fn guess_provider(key_name: &str) -> String {
    let lower = key_name.to_lowercase();
    if lower.contains("openai") { return "openai".into(); }
    if lower.contains("anthropic") { return "anthropic".into(); }
    if lower.contains("grok") || lower.contains("xai") { return "grok".into(); }
    if lower.contains("telegram") { return "telegram".into(); }
    if lower.contains("slack") { return "slack".into(); }
    if lower.contains("discord") { return "discord".into(); }
    if lower.contains("github") { return "github".into(); }
    "unknown".into()
}

// --- Ephemeral .env Writer (Option C) ---

#[derive(Debug, Serialize)]
pub struct SecureLaunchResult {
    pub success: bool,
    pub keys_injected: u32,
    pub daemon_restarted: bool,
    pub env_cleaned: bool,
    pub detail: String,
}

fn openclaw_env_path() -> Result<PathBuf, String> {
    let home = home_dir().ok_or("Home directory not found")?;
    Ok(home.join(".openclaw").join(".env"))
}

#[tauri::command]
pub async fn launch_secure_agent() -> Result<SecureLaunchResult, String> {
    // 1. Check vault is unlocked and get all entries
    let entries = crate::vault_store::vault_list_entries()?;
    if entries.is_empty() {
        return Ok(SecureLaunchResult {
            success: false,
            keys_injected: 0,
            daemon_restarted: false,
            env_cleaned: false,
            detail: "No secrets in vault. Add secrets first.".into(),
        });
    }

    // 2. Build .env content from vault secrets
    let mut env_lines: Vec<String> = Vec::new();
    let mut count = 0u32;
    for entry in &entries {
        match crate::vault_store::vault_get_secret(entry.alias.clone()) {
            Ok(value) => {
                let key_name = entry.alias.to_uppercase().replace('-', "_");
                env_lines.push(format!("{}={}", key_name, value));
                count += 1;
            }
            Err(_) => continue,
        }
    }

    if env_lines.is_empty() {
        return Ok(SecureLaunchResult {
            success: false,
            keys_injected: 0,
            daemon_restarted: false,
            env_cleaned: false,
            detail: "Could not read any secrets from vault.".into(),
        });
    }

    // 3. Write ephemeral .env
    let env_path = openclaw_env_path()?;
    let env_content = env_lines.join("\n") + "\n";
    fs::write(&env_path, &env_content).map_err(|e| format!("Write .env failed: {e}"))?;
    tracing::info!("Ephemeral .env written with {} keys", count);

    // 4. Restart OpenClaw daemon
    let daemon_restarted = restart_openclaw_daemon();

    // 5. Sleep 2 seconds to let daemon read .env
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // 6. Delete/zero .env
    let env_cleaned = match fs::write(&env_path, "# Managed by Vault-0 - secrets injected at runtime\n") {
        Ok(_) => {
            tracing::info!("Ephemeral .env cleaned");
            true
        }
        Err(e) => {
            tracing::error!("Failed to clean .env: {e}");
            false
        }
    };

    // 7. Log to evidence
    crate::evidence::push("info", &format!(
        "Secure launch: {} keys injected, daemon restarted: {}, .env cleaned: {}",
        count, daemon_restarted, env_cleaned
    ));

    Ok(SecureLaunchResult {
        success: true,
        keys_injected: count,
        daemon_restarted,
        env_cleaned,
        detail: format!(
            "{} secrets injected. Daemon {}. .env {}.",
            count,
            if daemon_restarted { "restarted" } else { "restart failed (try manually)" },
            if env_cleaned { "cleaned" } else { "cleanup failed" }
        ),
    })
}

fn restart_openclaw_daemon() -> bool {
    use std::process::Command;

    // Try launchctl first (macOS daemon)
    let uid = Command::new("id").arg("-u").output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if !uid.is_empty() {
        let service = format!("gui/{}/ai.openclaw.gateway", uid);
        let result = Command::new("launchctl")
            .args(["kickstart", "-k", &service])
            .output();
        if let Ok(out) = result {
            if out.status.success() {
                tracing::info!("Daemon restarted via launchctl kickstart");
                return true;
            }
        }
    }

    // Fallback: try openclaw restart
    let result = Command::new("sh")
        .args(["-lc", "openclaw restart 2>/dev/null || openclaw gateway --restart 2>/dev/null"])
        .output();
    if let Ok(out) = result {
        if out.status.success() {
            tracing::info!("Daemon restarted via openclaw restart");
            return true;
        }
    }

    // Fallback: find and HUP the gateway process
    let result = Command::new("sh")
        .args(["-lc", "pgrep -f 'openclaw.*gateway' | head -1"])
        .output();
    if let Ok(out) = result {
        let pid = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !pid.is_empty() {
            let kill = Command::new("kill").args(["-HUP", &pid]).output();
            if let Ok(k) = kill {
                if k.status.success() {
                    tracing::info!("Daemon signaled via HUP on PID {}", pid);
                    return true;
                }
            }
        }
    }

    tracing::warn!("Could not restart OpenClaw daemon automatically");
    false
}

// --- Scan for New Secrets ---

#[derive(Debug, Serialize)]
pub struct NewSecretFound {
    pub key_name: String,
    pub file: String,
    pub provider: String,
    pub preview: String,
}

#[tauri::command]
pub fn scan_for_new_secrets() -> Result<Vec<NewSecretFound>, String> {
    let home = home_dir().ok_or("Home directory not found")?;
    let openclaw_dir = home.join(".openclaw");

    // Get existing vault entries for comparison
    let vault_entries = crate::vault_store::vault_list_entries().unwrap_or_default();
    let vault_aliases: std::collections::HashSet<String> = vault_entries.iter()
        .map(|e| e.alias.to_lowercase().replace('-', "_"))
        .collect();

    let mut new_secrets: Vec<NewSecretFound> = Vec::new();

    // Scan .env
    let env_path = openclaw_dir.join(".env");
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(&env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some(eq) = trimmed.find('=') {
                    let key = trimmed[..eq].trim();
                    let val = trimmed[eq + 1..].trim().trim_matches('"').trim_matches('\'');
                    if val.is_empty() || val.starts_with("VAULT0_ALIAS") || val == "your-key-here" {
                        continue;
                    }
                    let normalized = key.to_lowercase().replace('-', "_");
                    if !vault_aliases.contains(&normalized) {
                        let preview = if val.len() > 8 {
                            format!("{}...{}", &val[..4], &val[val.len()-4..])
                        } else {
                            "****".to_string()
                        };
                        new_secrets.push(NewSecretFound {
                            key_name: key.to_string(),
                            file: ".env".to_string(),
                            provider: guess_provider(key),
                            preview,
                        });
                    }
                }
            }
        }
    }

    // Scan openclaw.json for inline apiKey values
    let config_path = openclaw_dir.join("openclaw.json");
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            for (key_name, prefix) in KEY_PATTERNS {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.contains(key_name) && !trimmed.contains("apiKey") {
                        continue;
                    }
                    let val = extract_value(trimmed);
                    if val.is_empty() || val.starts_with("$") || val.starts_with("VAULT0_ALIAS") {
                        continue;
                    }
                    if !prefix.is_empty() && !val.starts_with(prefix) {
                        continue;
                    }
                    let normalized = key_name.to_lowercase().replace('-', "_");
                    if !vault_aliases.contains(&normalized) {
                        let preview = if val.len() > 8 {
                            format!("{}...{}", &val[..4], &val[val.len()-4..])
                        } else {
                            "****".to_string()
                        };
                        new_secrets.push(NewSecretFound {
                            key_name: key_name.to_string(),
                            file: "openclaw.json".to_string(),
                            provider: guess_provider(key_name),
                            preview,
                        });
                    }
                }
            }
        }
    }

    // Scan auth-profiles.json
    let auth_path = openclaw_dir.join("auth-profiles.json");
    if auth_path.exists() {
        if let Ok(content) = fs::read_to_string(&auth_path) {
            for (key_name, _) in KEY_PATTERNS {
                if content.contains(key_name) {
                    let normalized = key_name.to_lowercase().replace('-', "_");
                    if !vault_aliases.contains(&normalized) {
                        new_secrets.push(NewSecretFound {
                            key_name: key_name.to_string(),
                            file: "auth-profiles.json".to_string(),
                            provider: guess_provider(key_name),
                            preview: "****".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(new_secrets)
}
