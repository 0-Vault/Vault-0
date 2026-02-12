//! WebSocket client for the OpenClaw gateway. Streams real-time agent events
//! (messages, tool calls, thinking states) into a ring buffer that the frontend polls.

use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

const EVENT_CAP: usize = 500;
const DEFAULT_PORT: u16 = 18789;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct GatewayEvent {
    pub ts: String,
    pub kind: String,
    pub session_id: String,
    pub platform: String,
    pub summary: String,
    pub payload: String,
}

#[derive(Debug, Serialize)]
pub struct GatewayStatus {
    pub connected: bool,
    pub event_count: usize,
    pub gateway_url: String,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

static CONNECTED: AtomicBool = AtomicBool::new(false);
static SHOULD_RUN: AtomicBool = AtomicBool::new(false);
static EVENTS: Lazy<RwLock<VecDeque<GatewayEvent>>> =
    Lazy::new(|| RwLock::new(VecDeque::new()));
static GATEWAY_URL: Lazy<RwLock<String>> =
    Lazy::new(|| RwLock::new(String::new()));

fn push_event(evt: GatewayEvent) {
    if let Ok(mut g) = EVENTS.write() {
        g.push_back(evt);
        while g.len() > EVENT_CAP {
            g.pop_front();
        }
    }
}

fn now_ts() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}.{:03}", d.as_secs(), d.subsec_millis()))
        .unwrap_or_else(|_| "0.000".into())
}

// ---------------------------------------------------------------------------
// Config helpers (reads ~/.openclaw/openclaw.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OcConfig {
    #[serde(default)]
    gateway: Option<OcGateway>,
}

#[derive(Debug, Deserialize)]
struct OcGateway {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    auth: Option<OcAuth>,
}

#[derive(Debug, Deserialize)]
struct OcAuth {
    #[serde(default)]
    token: Option<String>,
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn read_gateway_config() -> (u16, Option<String>) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return (DEFAULT_PORT, None),
    };
    let path = home.join(".openclaw").join("openclaw.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (DEFAULT_PORT, None),
    };
    // Strip // comments for JSON5 compat
    let stripped: String = content
        .lines()
        .map(|l| if l.trim().starts_with("//") { "" } else { l })
        .collect::<Vec<&str>>()
        .join("\n");
    let config: OcConfig = serde_json::from_str(&stripped)
        .or_else(|_| serde_json::from_str(&content))
        .unwrap_or(OcConfig { gateway: None });
    let port = config.gateway.as_ref().map(|g| g.port).unwrap_or(DEFAULT_PORT);
    let token = config
        .gateway
        .as_ref()
        .and_then(|g| g.auth.as_ref())
        .and_then(|a| a.token.clone());
    (port, token)
}

// ---------------------------------------------------------------------------
// WebSocket loop
// ---------------------------------------------------------------------------

async fn ws_loop() {
    let (port, token) = read_gateway_config();
    let url = format!("ws://127.0.0.1:{}", port);
    if let Ok(mut g) = GATEWAY_URL.write() {
        *g = url.clone();
    }

    info!("Gateway WS connecting to {}", url);

    let ws_url = if let Some(ref t) = token {
        format!("{}/?token={}", url, t)
    } else {
        url.clone()
    };

    let ws_stream = match tokio_tungstenite::connect_async(&ws_url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            error!("Gateway WS connect failed: {}", e);
            CONNECTED.store(false, Ordering::Relaxed);
            return;
        }
    };

    CONNECTED.store(true, Ordering::Relaxed);
    info!("Gateway WS connected");

    let (mut _write, mut read) = ws_stream.split();

    // If token provided, send auth message (some gateways require it as a frame)
    if let Some(ref t) = token {
        let auth_msg = serde_json::json!({"type": "auth", "token": t});
        let _ = _write.send(Message::Text(auth_msg.to_string())).await;
    }

    while SHOULD_RUN.load(Ordering::Relaxed) {
        match tokio::time::timeout(std::time::Duration::from_secs(30), read.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                parse_and_store(&text);
            }
            Ok(Some(Ok(Message::Ping(data)))) => {
                let _ = _write.send(Message::Pong(data)).await;
            }
            Ok(Some(Ok(Message::Close(_)))) => {
                warn!("Gateway WS closed by server");
                break;
            }
            Ok(Some(Err(e))) => {
                error!("Gateway WS read error: {}", e);
                break;
            }
            Ok(None) => {
                warn!("Gateway WS stream ended");
                break;
            }
            Err(_) => {
                // Timeout — send ping to keep alive
                let _ = _write.send(Message::Ping(vec![])).await;
            }
            _ => {}
        }
    }

    CONNECTED.store(false, Ordering::Relaxed);
    info!("Gateway WS disconnected");
}

fn parse_and_store(raw: &str) {
    let json: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => {
            push_event(GatewayEvent {
                ts: now_ts(),
                kind: "unknown".into(),
                session_id: String::new(),
                platform: String::new(),
                summary: raw.chars().take(120).collect(),
                payload: raw.to_string(),
            });
            return;
        }
    };

    let kind = json
        .get("type")
        .or_else(|| json.get("event"))
        .or_else(|| json.get("kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let session_id = json
        .get("sessionId")
        .or_else(|| json.get("session_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let platform = json
        .get("platform")
        .or_else(|| json.get("channel"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let summary = build_summary(kind, &json);

    let mapped_kind = match kind {
        "message" | "message_received" | "incoming" => "message_in",
        "response" | "message_sent" | "outgoing" | "reply" => "message_out",
        "tool_call" | "tool_use" | "function_call" => "tool_call",
        "tool_result" | "tool_response" | "function_result" => "tool_result",
        "thinking" | "thought" | "reasoning" => "thinking",
        "error" | "failure" => "error",
        other => other,
    };

    push_event(GatewayEvent {
        ts: now_ts(),
        kind: mapped_kind.to_string(),
        session_id,
        platform,
        summary,
        payload: raw.to_string(),
    });
}

fn build_summary(kind: &str, json: &serde_json::Value) -> String {
    match kind {
        "message" | "message_received" | "incoming" => {
            let text = json
                .get("text")
                .or_else(|| json.get("content"))
                .or_else(|| json.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if text.len() > 100 {
                format!("{}...", &text[..100])
            } else {
                text.to_string()
            }
        }
        "response" | "message_sent" | "outgoing" | "reply" => {
            let text = json
                .get("text")
                .or_else(|| json.get("content"))
                .or_else(|| json.get("response"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if text.len() > 100 {
                format!("{}...", &text[..100])
            } else {
                text.to_string()
            }
        }
        "tool_call" | "tool_use" | "function_call" => {
            let name = json
                .get("name")
                .or_else(|| json.get("tool"))
                .or_else(|| json.get("function"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Tool: {}", name)
        }
        "tool_result" | "tool_response" | "function_result" => {
            let name = json
                .get("name")
                .or_else(|| json.get("tool"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Result: {}", name)
        }
        "thinking" | "thought" | "reasoning" => {
            let text = json
                .get("text")
                .or_else(|| json.get("thought"))
                .and_then(|v| v.as_str())
                .unwrap_or("...");
            if text.len() > 80 {
                format!("{}...", &text[..80])
            } else {
                text.to_string()
            }
        }
        _ => {
            let s = json.to_string();
            if s.len() > 120 {
                format!("{}...", &s[..120])
            } else {
                s
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn gateway_connect() -> Result<String, String> {
    if CONNECTED.load(Ordering::Relaxed) {
        return Ok("Already connected".into());
    }
    SHOULD_RUN.store(true, Ordering::Relaxed);
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("gateway ws runtime");
        rt.block_on(async {
            loop {
                ws_loop().await;
                if !SHOULD_RUN.load(Ordering::Relaxed) {
                    break;
                }
                // Reconnect after 3 seconds if still supposed to run
                info!("Gateway WS reconnecting in 3s...");
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        });
    });
    Ok("Connecting".into())
}

#[tauri::command]
pub fn gateway_disconnect() -> Result<String, String> {
    SHOULD_RUN.store(false, Ordering::Relaxed);
    CONNECTED.store(false, Ordering::Relaxed);
    Ok("Disconnected".into())
}

#[tauri::command]
pub fn gateway_status() -> Result<GatewayStatus, String> {
    let event_count = EVENTS.read().map(|g| g.len()).unwrap_or(0);
    let gateway_url = GATEWAY_URL
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Ok(GatewayStatus {
        connected: CONNECTED.load(Ordering::Relaxed),
        event_count,
        gateway_url,
    })
}

#[tauri::command]
pub fn get_gateway_events() -> Result<Vec<GatewayEvent>, String> {
    let g = EVENTS.read().map_err(|_| "lock")?;
    Ok(g.iter().cloned().collect())
}
