use crate::evidence;
use crate::mcp_guard;
use crate::policy::Policy;
use base64::Engine;
use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use thiserror::Error;
use tracing::info;

static RUNNING: AtomicBool = AtomicBool::new(false);

pub struct ProxyState {
    pub vault: HashMap<String, String>,
    pub policy: Policy,
}

static STATE: Lazy<RwLock<ProxyState>> = Lazy::new(|| {
    RwLock::new(ProxyState {
        vault: HashMap::new(),
        policy: Policy::default(),
    })
});

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("proxy already running")]
    AlreadyRunning,
    #[error("proxy not running")]
    NotRunning,
    #[error("bind failed: {0}")]
    Bind(String),
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

pub fn state() -> &'static RwLock<ProxyState> {
    &STATE
}

pub fn start() -> Result<(), ProxyError> {
    if RUNNING.swap(true, Ordering::Relaxed) {
        return Err(ProxyError::AlreadyRunning);
    }
    let addr = SocketAddr::from_str("127.0.0.1:3840").map_err(|e| ProxyError::Bind(e.to_string()))?;
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("proxy runtime");
        rt.block_on(async {
            let app = axum::Router::new()
                .route("/", axum::routing::any(proxy_handler))
                .route("/*path", axum::routing::any(proxy_handler));
            let listener = tokio::net::TcpListener::bind(addr).await.expect("proxy bind");
            info!("Vault-0 proxy listening on {}", addr);
            axum::serve(listener, app).await.expect("proxy serve");
        });
    });
    Ok(())
}

pub fn stop() -> Result<(), ProxyError> {
    if !RUNNING.swap(false, Ordering::Relaxed) {
        return Err(ProxyError::NotRunning);
    }
    Ok(())
}

async fn proxy_handler(req: Request) -> Response {
    let uri = req.uri().clone();
    let host_header = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let path = uri.path();
    let host = uri
        .host()
        .map(|h| h.to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| host_header.split(':').next().unwrap_or("").to_string());

    let (allowed, deny_reason) = {
        let guard = STATE.read().expect("state read");
        let policy = &guard.policy;
        let allow = policy.allow_domains.is_empty()
            || policy.allow_domains.iter().any(|d| host.ends_with(d.as_str()));
        let block = policy.block_domains.iter().any(|d| host.ends_with(d.as_str()));
        if block {
            (false, Some("domain blocked by policy".to_string()))
        } else if !policy.allow_domains.is_empty() && !allow {
            (false, Some("domain not in allow list".to_string()))
        } else {
            (true, None)
        }
    };

    if !allowed {
        let msg = format!("Vault-0 policy denied: {}", deny_reason.unwrap_or_default());
        evidence::push("blocked", &msg);
        return (StatusCode::FORBIDDEN, msg).into_response();
    }

    if mcp_guard::is_mcp_request(&host, path) {
        if !mcp_guard::origin_allowed(&host) {
            evidence::push("blocked", "MCP server not in allowlist");
            return (
                StatusCode::FORBIDDEN,
                "MCP server not in allowlist".to_string(),
            )
                .into_response();
        }
        if mcp_guard::would_be_ssrf(uri.authority().map(|a| a.as_str()).unwrap_or("")) {
            evidence::push("blocked", "MCP SSRF: private/internal target blocked");
            return (
                StatusCode::FORBIDDEN,
                "MCP SSRF: private/internal target blocked".to_string(),
            )
                .into_response();
        }
        if mcp_guard::token_passthrough_disabled() && req.headers().contains_key("authorization") {
            evidence::push("blocked", "Token passthrough disabled for MCP");
            return (
                StatusCode::BAD_REQUEST,
                "Token passthrough disabled for MCP".to_string(),
            )
                .into_response();
        }
    }

    let (method, headers, body) = (req.method().clone(), req.headers().clone(), req.into_body());
    let target_url = build_full_uri(&uri, &host);
    let inject_key = alias_for_host(&host);

    let (auth_header, redact_patterns) = {
        let state_guard = STATE.read().expect("state read");
        let auth = inject_key.as_ref().and_then(|alias| state_guard.vault.get(alias.as_str()).cloned());
        let redact = state_guard.policy.output_redact_patterns.clone();
        (auth, redact)
    };

    let mut out_headers = reqwest::header::HeaderMap::new();
    for (k, v) in headers.iter() {
        if k.as_str().eq_ignore_ascii_case("authorization") && auth_header.is_some() {
            continue;
        }
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes()) {
            if let Ok(value) = reqwest::header::HeaderValue::from_bytes(v.as_bytes()) {
                out_headers.insert(name, value);
            }
        }
    }
    if let Some(ref key) = auth_header {
        out_headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key))
                .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("Bearer")),
        );
    }

    let client = reqwest::Client::builder().build().unwrap_or_default();
    const BODY_LIMIT: usize = 10 * 1024 * 1024;
    let body_bytes = axum::body::to_bytes(body, BODY_LIMIT).await.unwrap_or_default();
    let req_builder = client.request(method.clone(), &target_url).headers(out_headers.clone());
    let upstream = if body_bytes.is_empty() {
        req_builder.send().await
    } else {
        req_builder.body(body_bytes.to_vec()).send().await
    };

    match upstream {
        Ok(resp) => {
            let status = resp.status();
            let headers_vec: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let bytes = resp.bytes().await.unwrap_or_default();
            if status.as_u16() == 402 {
                if let Some(intent) = crate::x402::parse_402_required(&headers_vec, &bytes) {
                    let id = crate::x402::record_pending(intent.clone());
                    evidence::push(
                        "payment",
                        &format!("402 pending {} cents -> {} [{}]", intent.amount_cents, intent.recipient, id),
                    );

                    let should_auto_settle = {
                        let guard = STATE.read().expect("state read");
                        let p = &guard.policy;
                        p.auto_settle_402
                            && (p.spend_cap_cents.is_none() || intent.amount_cents <= p.spend_cap_cents.unwrap_or(0))
                    };

                    if should_auto_settle {
                        if let Ok(wallet_info) = crate::wallet::get_wallet_info() {
                            if wallet_info.has_wallet {
                                if let Ok(sig) = crate::wallet::sign_x402_payment(
                                    intent.amount_cents,
                                    intent.recipient.clone(),
                                    intent.network.clone(),
                                )
                                .await
                                {
                                    let payload = base64::engine::general_purpose::STANDARD.encode(
                                        serde_json::json!({
                                            "scheme": "evm-eip3009",
                                            "signature": sig,
                                            "amount_cents": intent.amount_cents,
                                            "recipient": intent.recipient,
                                            "network": intent.network,
                                        })
                                        .to_string()
                                        .as_bytes(),
                                    );
                                    let mut retry_headers = out_headers.clone();
                                    retry_headers.insert(
                                        reqwest::header::HeaderName::from_static("x-payment"),
                                        reqwest::header::HeaderValue::from_str(&payload).unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
                                    );
                                    let retry_builder = client
                                        .request(method.clone(), &target_url)
                                        .headers(retry_headers);
                                    let retry_resp = if body_bytes.is_empty() {
                                        retry_builder.send().await
                                    } else {
                                        retry_builder.body(body_bytes.to_vec()).send().await
                                    };
                                    if let Ok(retry) = retry_resp {
                                        let retry_status = retry.status();
                                        if retry_status.is_success() {
                                            evidence::push(
                                                "payment",
                                                &format!("402 settled {} cents -> {}", intent.amount_cents, intent.recipient),
                                            );
                                            let retry_headers_vec: Vec<(String, String)> = retry
                                                .headers()
                                                .iter()
                                                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                                                .collect();
                                            let retry_bytes = retry.bytes().await.unwrap_or_default();
                                            let retry_filtered = redact_body(&retry_bytes, &redact_patterns);
                                            let mut retry_builder = Response::builder().status(retry_status);
                                            for (k, v) in &retry_headers_vec {
                                                if let (Ok(name), Ok(value)) = (
                                                    axum::http::HeaderName::from_bytes(k.as_bytes()),
                                                    axum::http::HeaderValue::from_str(v),
                                                ) {
                                                    retry_builder = retry_builder.header(name, value);
                                                }
                                            }
                                            return retry_builder
                                                .body(Body::from(retry_filtered))
                                                .unwrap_or_else(|_| Response::new(Body::from("internal error")));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                evidence::push("allowed", &format!("{} {}", method, target_url));
            }
            let filtered = redact_body(&bytes, &redact_patterns);
            let mut resp_builder = Response::builder().status(status);
            for (k, v) in &headers_vec {
                if let (Ok(name), Ok(value)) = (
                    axum::http::HeaderName::from_bytes(k.as_bytes()),
                    axum::http::HeaderValue::from_str(v),
                ) {
                    resp_builder = resp_builder.header(name, value);
                }
            }
            resp_builder
                .body(Body::from(filtered))
                .unwrap_or_else(|_| Response::new(Body::from("internal error")))
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("Upstream error: {}", e),
        )
            .into_response(),
    }
}

fn build_full_uri(uri: &Uri, host: &str) -> String {
    if let Some(s) = uri.path().strip_prefix("https://").or_else(|| uri.path().strip_prefix("http://")) {
        if s.contains('/') || s.contains('?') {
            let scheme = if uri.path().starts_with("https") { "https" } else { "http" };
            return format!("{}://{}", scheme, s);
        }
    }
    let scheme = uri.scheme_str().unwrap_or("https");
    let path = uri.path();
    let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
    let port = uri.port_u16();
    if host.is_empty() {
        return format!("{}://localhost{}{}", scheme, path, query);
    }
    if let Some(p) = port {
        format!("{}://{}:{}{}{}", scheme, host, p, path, query)
    } else {
        format!("{}://{}{}{}", scheme, host, path, query)
    }
}

fn alias_for_host(host: &str) -> Option<String> {
    let alias = match host {
        h if h.contains("openai.com") => "openai",
        h if h.contains("anthropic.com") => "anthropic",
        _ => return None,
    };
    Some(alias.to_string())
}

fn redact_body(body: &[u8], patterns: &[String]) -> Vec<u8> {
    let mut text = match std::str::from_utf8(body) {
        Ok(t) => t.to_string(),
        Err(_) => return body.to_vec(),
    };
    for pat in patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            text = re.replace_all(&text, "[REDACTED]").to_string();
        }
    }
    text.into_bytes()
}
