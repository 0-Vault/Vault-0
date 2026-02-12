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

/// Protocol/system events filtered from the user-facing event list.
const SKIP_EVENTS: &[&str] = &[
    "connect.challenge", "connect.response", "connect.success", "connect.error",
    "hello-ok", "auth", "ping", "pong", "heartbeat", "health", "tick",
];

fn is_skip_event(event_type: &str) -> bool {
    SKIP_EVENTS.iter().any(|p| event_type == *p)
}

/// Build the `connect` request frame matching the OpenClaw gateway protocol.
/// Crabwalk reference: src/integrations/openclaw/protocol.ts → createConnectParams
fn build_connect_request(token: &Option<String>) -> serde_json::Value {
    let auth = token.as_ref().map(|t| serde_json::json!({"token": t}));
    serde_json::json!({
        "type": "req",
        "id": format!("connect-{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)),
        "method": "connect",
        "params": {
            "minProtocol": 3,
            "maxProtocol": 3,
            "client": {
                "id": "cli",
                "version": "0.1.0",
                "platform": "linux",
                "mode": "cli"
            },
            "role": "operator",
            "scopes": ["operator.read"],
            "caps": [],
            "commands": [],
            "permissions": {},
            "locale": "en-US",
            "userAgent": "vault0-monitor/1.0.3",
            "auth": auth
        }
    })
}

async fn ws_loop() {
    let (port, token) = read_gateway_config();
    let url = format!("ws://127.0.0.1:{}", port);
    if let Ok(mut g) = GATEWAY_URL.write() {
        *g = url.clone();
    }

    info!("Gateway WS connecting to {}", url);

    let ws_stream = match tokio_tungstenite::connect_async(&url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            error!("Gateway WS connect failed: {}", e);
            CONNECTED.store(false, Ordering::Relaxed);
            return;
        }
    };

    info!("Gateway WS TCP connected, waiting for challenge");

    let (mut write, mut read) = ws_stream.split();
    let mut authenticated = false;

    while SHOULD_RUN.load(Ordering::Relaxed) {
        match tokio::time::timeout(std::time::Duration::from_secs(30), read.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let json: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => {
                        if authenticated { parse_and_store(&text); }
                        continue;
                    }
                };

                // Determine frame type: OpenClaw uses {"type":"event","event":"..."} for events
                // and {"type":"hello-ok"} for auth success
                let frame_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
                let event_name = json.get("event").and_then(|v| v.as_str()).unwrap_or("");

                // Step 1: Gateway sends connect.challenge → we respond with connect request
                if frame_type == "event" && event_name == "connect.challenge" {
                    info!("Gateway challenge received, sending connect request");
                    let connect_req = build_connect_request(&token);
                    let _ = write.send(Message::Text(connect_req.to_string())).await;
                    continue;
                }

                // Step 2: Gateway responds with hello-ok → we're authenticated
                if frame_type == "hello-ok" {
                    authenticated = true;
                    CONNECTED.store(true, Ordering::Relaxed);
                    let protocol = json.get("protocol").and_then(|v| v.as_u64()).unwrap_or(0);
                    info!("Gateway WS authenticated (protocol {})", protocol);
                    continue;
                }

                // Response frame (type: "res") — result of our connect request
                if frame_type == "res" {
                    let ok = json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                    if ok {
                        authenticated = true;
                        CONNECTED.store(true, Ordering::Relaxed);
                        info!("Gateway WS connect response OK");
                        continue;
                    } else {
                        let msg = json.pointer("/error/message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown error");
                        let code = json.pointer("/error/code")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        error!("Gateway connect rejected: {} ({})", msg, code);
                        push_event(GatewayEvent {
                            ts: now_ts(),
                            kind: "error".into(),
                            session_id: String::new(),
                            platform: String::new(),
                            summary: format!("Connect rejected: {}", msg),
                            payload: text.clone(),
                        });
                        // Stop reconnecting on auth rejection
                        SHOULD_RUN.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                // Auth error
                if (frame_type == "error" || event_name == "connect.error") && !authenticated {
                    let msg = json.get("message")
                        .or_else(|| json.pointer("/payload/message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    error!("Gateway auth failed: {}", msg);
                    push_event(GatewayEvent {
                        ts: now_ts(),
                        kind: "error".into(),
                        session_id: String::new(),
                        platform: String::new(),
                        summary: format!("Auth failed: {}", msg),
                        payload: text.clone(),
                    });
                    SHOULD_RUN.store(false, Ordering::Relaxed);
                    break;
                }

                // Skip system/protocol events
                if is_skip_event(event_name) || is_skip_event(frame_type) {
                    continue;
                }

                // Real agent event
                if !authenticated {
                    // Got a real event before hello-ok — treat as implicit auth
                    authenticated = true;
                    CONNECTED.store(true, Ordering::Relaxed);
                    info!("Gateway WS connected (implicit auth)");
                }
                parse_and_store_v2(frame_type, event_name, &json, &text);
            }
            Ok(Some(Ok(Message::Ping(data)))) => {
                let _ = write.send(Message::Pong(data)).await;
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
                let _ = write.send(Message::Ping(vec![])).await;
            }
            _ => {}
        }
    }

    CONNECTED.store(false, Ordering::Relaxed);
    info!("Gateway WS disconnected");
}

/// Parse OpenClaw gateway events using the real protocol shapes.
/// Reference: crabwalk/src/integrations/openclaw/parser.ts
fn parse_and_store_v2(
    frame_type: &str,
    event_name: &str,
    json: &serde_json::Value,
    raw: &str,
) {
    let payload = json.get("payload").unwrap_or(json);

    let session_id = payload
        .get("sessionKey")
        .or_else(|| payload.get("sessionId"))
        .or_else(|| payload.get("session_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let platform = payload
        .get("platform")
        .or_else(|| payload.get("channel"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match event_name {
        // Chat events: delta (streaming), final, aborted, error
        "chat" => {
            let state = payload.get("state").and_then(|v| v.as_str()).unwrap_or("");
            let kind = match state {
                "delta" => "thinking",
                "final" => "message_out",
                "aborted" => "error",
                "error" => "error",
                _ => "message_out",
            };
            let summary = extract_chat_content(payload, state);
            push_event(GatewayEvent {
                ts: now_ts(), kind: kind.into(), session_id, platform, summary, payload: raw.into(),
            });
        }
        // Agent events: lifecycle, assistant stream, tool_use, tool_result
        "agent" => {
            let stream = payload.get("stream").and_then(|v| v.as_str()).unwrap_or("");
            let data = payload.get("data").unwrap_or(payload);
            let data_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");

            let (kind, summary) = match (stream, data_type) {
                ("lifecycle", _) => {
                    let phase = data.get("phase").and_then(|v| v.as_str()).unwrap_or("");
                    match phase {
                        "start" => ("thinking", "Run started".to_string()),
                        "end" => ("message_out", "Run completed".to_string()),
                        _ => ("thinking", format!("Lifecycle: {}", phase)),
                    }
                }
                (_, "tool_use") => {
                    let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    ("tool_call", format!("Tool: {}", name))
                }
                (_, "tool_result") => {
                    let content = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    let preview = truncate(content, 100);
                    ("tool_result", format!("Result: {}", preview))
                }
                ("assistant", _) => {
                    let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    ("thinking", truncate(text, 100))
                }
                _ => {
                    ("thinking", truncate(&data.to_string(), 100))
                }
            };
            push_event(GatewayEvent {
                ts: now_ts(), kind: kind.into(), session_id, platform, summary, payload: raw.into(),
            });
        }
        // Exec events
        "exec.started" => {
            let cmd = payload.get("command").and_then(|v| v.as_str()).unwrap_or("");
            push_event(GatewayEvent {
                ts: now_ts(), kind: "tool_call".into(), session_id, platform,
                summary: format!("Exec: {}", truncate(cmd, 80)),
                payload: raw.into(),
            });
        }
        "exec.output" => {
            let output = payload.get("output").and_then(|v| v.as_str()).unwrap_or("");
            let stream = payload.get("stream").and_then(|v| v.as_str()).unwrap_or("stdout");
            push_event(GatewayEvent {
                ts: now_ts(), kind: "tool_result".into(), session_id, platform,
                summary: format!("[{}] {}", stream, truncate(output, 80)),
                payload: raw.into(),
            });
        }
        "exec.completed" => {
            let exit_code = payload.get("exitCode").and_then(|v| v.as_i64()).unwrap_or(-1);
            let duration = payload.get("durationMs").and_then(|v| v.as_u64()).unwrap_or(0);
            push_event(GatewayEvent {
                ts: now_ts(), kind: "tool_result".into(), session_id, platform,
                summary: format!("Exec done (exit {}, {}ms)", exit_code, duration),
                payload: raw.into(),
            });
        }
        // Fallback for any other event
        _ => {
            push_event(GatewayEvent {
                ts: now_ts(),
                kind: frame_type.to_string(),
                session_id, platform,
                summary: truncate(&json.to_string(), 120),
                payload: raw.into(),
            });
        }
    }
}

fn extract_chat_content(payload: &serde_json::Value, state: &str) -> String {
    // Try message.content[].text first (standard shape)
    if let Some(msg) = payload.get("message") {
        if let Some(content) = msg.get("content") {
            if let Some(arr) = content.as_array() {
                let texts: Vec<&str> = arr.iter()
                    .filter_map(|b| {
                        let btype = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        match btype {
                            "text" => b.get("text").and_then(|v| v.as_str()),
                            "tool_use" => b.get("name").and_then(|v| v.as_str()),
                            _ => None,
                        }
                    })
                    .collect();
                if !texts.is_empty() {
                    return truncate(&texts.join(""), 120);
                }
            }
            if let Some(s) = content.as_str() {
                return truncate(s, 120);
            }
        }
        if let Some(s) = msg.get("text").and_then(|v| v.as_str()) {
            return truncate(s, 120);
        }
        if let Some(s) = msg.as_str() {
            return truncate(s, 120);
        }
    }
    if let Some(err) = payload.get("errorMessage").and_then(|v| v.as_str()) {
        return truncate(err, 120);
    }
    match state {
        "delta" => "Thinking...".into(),
        "final" => "Response complete".into(),
        "aborted" => "Aborted".into(),
        "error" => "Error".into(),
        _ => state.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

/// Legacy parser kept for non-gateway events (e.g. from evidence log)
fn parse_and_store(raw: &str) {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) {
        let frame_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let event_name = json.get("event").and_then(|v| v.as_str()).unwrap_or("");
        parse_and_store_v2(frame_type, event_name, &json, raw);
    } else {
        push_event(GatewayEvent {
            ts: now_ts(),
            kind: "unknown".into(),
            session_id: String::new(),
            platform: String::new(),
            summary: truncate(raw, 120),
            payload: raw.to_string(),
        });
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

#[tauri::command]
pub fn gateway_clear_events() -> Result<String, String> {
    if let Ok(mut g) = EVENTS.write() {
        g.clear();
    }
    Ok("Cleared".into())
}
