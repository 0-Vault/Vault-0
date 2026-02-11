use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct ReadinessProbeResult {
    pub ready: bool,
    pub source: String,
    pub install_path: String,
    pub status_command_ok: bool,
    pub status_command_output: String,
    pub http_ok: bool,
    pub http_url: String,
    pub http_status: u16,
    pub diagnostics: Vec<String>,
}

fn resolve_install_path(path: Option<String>) -> Result<String, String> {
    if let Some(p) = path {
        let p = p.trim();
        if !p.is_empty() && Path::new(p).exists() {
            return Ok(p.to_string());
        }
    }

    let home = dirs::home_dir().ok_or_else(|| "Home directory not found".to_string())?;
    let candidates = vec![home.join("openclaw"), home.join("clawbot")];
    for c in candidates {
        if c.exists() && c.is_dir() {
            return Ok(c.to_string_lossy().to_string());
        }
    }
    Err("OpenClaw install path not found (tried ~/openclaw and ~/clawbot)".to_string())
}

fn run_status_command(install_path: &str) -> (bool, String, Vec<String>) {
    let mut diagnostics = Vec::new();
    let cmd = format!(
        "cd \"{}\" && npx -y pnpm@10.23.0 run openclaw status",
        install_path.replace('"', "\\\"")
    );
    diagnostics.push(format!("Running status command: {}", cmd));

    let output = Command::new("/bin/zsh").arg("-lc").arg(cmd).output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let combined = if stderr.trim().is_empty() {
                stdout.clone()
            } else {
                format!("{}\n{}", stdout, stderr)
            };
            let lower = combined.to_lowercase();
            let healthy_markers = [
                "online",
                "running",
                "ready",
                "healthy",
                "ok",
                "connected",
            ];
            let marker_match = healthy_markers.iter().any(|m| lower.contains(m));
            let ok = out.status.success() && marker_match;
            diagnostics.push(format!(
                "Status command exit: {} marker_match:{}",
                out.status.code().unwrap_or(-1),
                marker_match
            ));
            (ok, combined, diagnostics)
        }
        Err(e) => {
            diagnostics.push(format!("Status command execution error: {}", e));
            (false, String::new(), diagnostics)
        }
    }
}

async fn run_http_probe() -> (bool, String, u16, Vec<String>) {
    let mut diagnostics = Vec::new();
    let candidates = [
        "http://127.0.0.1:3000/health",
        "http://127.0.0.1:3000/status",
        "http://127.0.0.1:8787/health",
        "http://127.0.0.1:8787/status",
        "http://127.0.0.1:8080/health",
        "http://127.0.0.1:8080/status",
    ];

    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string());
    let client = match client {
        Ok(c) => c,
        Err(e) => {
            diagnostics.push(format!("HTTP client init failed: {}", e));
            return (false, String::new(), 0, diagnostics);
        }
    };

    for url in candidates {
        diagnostics.push(format!("HTTP probe: {}", url));
        match client.get(url).send().await {
            Ok(resp) => {
                let code = resp.status().as_u16();
                if resp.status().is_success() {
                    return (true, url.to_string(), code, diagnostics);
                }
                diagnostics.push(format!("HTTP non-success {} at {}", code, url));
            }
            Err(e) => diagnostics.push(format!("HTTP error at {}: {}", url, e)),
        }
    }
    (false, String::new(), 0, diagnostics)
}

#[derive(Debug, Serialize)]
pub struct GatewayHealth {
    pub running: bool,
    pub port: u16,
    pub model: String,
    pub auth_mode: String,
    pub bind: String,
    pub config_secured: bool,
    pub unsecured_keys: Vec<String>,
    pub config_path: String,
}

#[derive(Debug, Deserialize)]
struct OpenClawConfig {
    #[serde(default)]
    gateway: Option<GatewaySection>,
    #[serde(default)]
    agents: Option<AgentsSection>,
}

#[derive(Debug, Deserialize)]
struct GatewaySection {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    bind: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    auth: Option<AuthSection>,
}

#[derive(Debug, Deserialize)]
struct AuthSection {
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentsSection {
    #[serde(default)]
    defaults: Option<AgentDefaults>,
}

#[derive(Debug, Deserialize)]
struct AgentDefaults {
    #[serde(default)]
    model: Option<ModelSection>,
}

#[derive(Debug, Deserialize)]
struct ModelSection {
    #[serde(default)]
    primary: Option<String>,
}

fn default_port() -> u16 { 18789 }

fn openclaw_config_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let path = home.join(".openclaw").join("openclaw.json");
    if path.exists() { Some(path) } else { None }
}

fn parse_openclaw_config(path: &Path) -> Result<OpenClawConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read config: {e}"))?;
    // openclaw.json uses JSON5 (comments, trailing commas) so we parse leniently
    serde_json::from_str::<OpenClawConfig>(&content)
        .or_else(|_| {
            // Strip comments for basic JSON5 compat
            let stripped: String = content.lines()
                .map(|l| {
                    let t = l.trim();
                    if t.starts_with("//") { "" } else { l }
                })
                .collect::<Vec<&str>>()
                .join("\n");
            serde_json::from_str::<OpenClawConfig>(&stripped)
        })
        .map_err(|e| format!("parse config: {e}"))
}

fn check_config_for_plaintext(path: &Path) -> (bool, Vec<String>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (true, vec![]),
    };
    let lower = content.to_lowercase();
    let key_prefixes = [
        ("sk-", "OpenAI key"),
        ("sk-ant-", "Anthropic key"),
        ("xai-", "Grok key"),
        ("xoxb-", "Slack token"),
        ("ghp_", "GitHub token"),
    ];
    let mut unsecured = Vec::new();
    for (prefix, label) in key_prefixes {
        if lower.contains(prefix) && !content.contains("VAULT0_ALIAS") {
            unsecured.push(label.to_string());
        }
    }
    let secured = unsecured.is_empty();
    (secured, unsecured)
}

#[tauri::command]
pub async fn check_gateway_health() -> Result<GatewayHealth, String> {
    let config_path = openclaw_config_path()
        .ok_or("OpenClaw config not found at ~/.openclaw/openclaw.json")?;

    let config = parse_openclaw_config(&config_path).unwrap_or(OpenClawConfig {
        gateway: None,
        agents: None,
    });

    let port = config.gateway.as_ref().map(|g| g.port).unwrap_or(18789);
    let bind = config.gateway.as_ref().and_then(|g| g.bind.clone()).unwrap_or("loopback".into());
    let auth_mode = config.gateway.as_ref().and_then(|g| g.auth.as_ref()).and_then(|a| a.mode.clone()).unwrap_or("none".into());
    let model = config.agents.as_ref()
        .and_then(|a| a.defaults.as_ref())
        .and_then(|d| d.model.as_ref())
        .and_then(|m| m.primary.clone())
        .unwrap_or("unknown".into());

    let (config_secured, unsecured_keys) = check_config_for_plaintext(&config_path);

    // Probe gateway
    let running = {
        let url = format!("http://127.0.0.1:{}/__openclaw__/canvas/", port);
        let client = Client::builder().timeout(Duration::from_secs(2)).build().ok();
        if let Some(c) = client {
            c.get(&url).send().await.map(|r| r.status().is_success() || r.status().as_u16() == 426).unwrap_or(false)
        } else {
            false
        }
    };

    info!("Gateway health: running={}, port={}, model={}, secured={}", running, port, model, config_secured);

    Ok(GatewayHealth {
        running,
        port,
        model,
        auth_mode,
        bind,
        config_secured,
        unsecured_keys,
        config_path: config_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn check_openclaw_readiness(path: Option<String>) -> Result<ReadinessProbeResult, String> {
    let install_path = resolve_install_path(path)?;
    info!("Readiness check for OpenClaw at {}", install_path);

    let (status_ok, status_output, mut diagnostics) = run_status_command(&install_path);
    if status_ok {
        diagnostics.push("Readiness source: status command".to_string());
        return Ok(ReadinessProbeResult {
            ready: true,
            source: "status_command".to_string(),
            install_path,
            status_command_ok: true,
            status_command_output: status_output,
            http_ok: false,
            http_url: String::new(),
            http_status: 0,
            diagnostics,
        });
    }

    let (http_ok, http_url, http_status, http_diag) = run_http_probe().await;
    diagnostics.extend(http_diag);
    if http_ok {
        diagnostics.push("Readiness source: http probe".to_string());
    } else {
        diagnostics.push("Readiness failed: no successful status command or HTTP probe".to_string());
    }

    Ok(ReadinessProbeResult {
        ready: http_ok,
        source: if http_ok {
            "http_probe".to_string()
        } else {
            "none".to_string()
        },
        install_path,
        status_command_ok: false,
        status_command_output: status_output,
        http_ok,
        http_url,
        http_status,
        diagnostics,
    })
}
