use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub amount_cents: u64,
    pub recipient: String,
    pub network: String,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPayment {
    pub id: String,
    pub intent: PaymentIntent,
    pub ts: i64,
}

static PENDING: Lazy<RwLock<VecDeque<PendingPayment>>> = Lazy::new(|| RwLock::new(VecDeque::new()));

/// Detect 402 from response headers (x402 PAYMENT-REQUIRED).
pub fn parse_402_required(headers: &[(String, String)], body: &[u8]) -> Option<PaymentIntent> {
    let has_402 = headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("payment-required") || v.contains("402"));
    if !has_402 {
        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(body) {
            if parsed.get("payment_required").and_then(|v| v.as_bool()).unwrap_or(false) {
                return Some(PaymentIntent {
                    amount_cents: parsed
                        .get("amount_cents")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    recipient: parsed
                        .get("recipient")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    network: parsed
                        .get("network")
                        .and_then(|v| v.as_str())
                        .unwrap_or("base")
                        .to_string(),
                    resource: parsed.get("resource").and_then(|v| v.as_str()).map(String::from),
                });
            }
        }
        return None;
    }
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("payment-required") {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(v) {
                return Some(PaymentIntent {
                    amount_cents: parsed
                        .get("amount_cents")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    recipient: parsed
                        .get("recipient")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    network: parsed
                        .get("network")
                        .and_then(|v| v.as_str())
                        .unwrap_or("base")
                        .to_string(),
                    resource: parsed.get("resource").and_then(|v| v.as_str()).map(String::from),
                });
            }
        }
    }
    Some(PaymentIntent {
        amount_cents: 0,
        recipient: String::new(),
        network: "base".to_string(),
        resource: None,
    })
}

pub fn record_pending(intent: PaymentIntent) -> String {
    let id = format!("pay_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let pending = PendingPayment {
        id: id.clone(),
        intent,
        ts,
    };
    if let Ok(mut g) = PENDING.write() {
        g.push_back(pending);
        if g.len() > 100 {
            g.pop_front();
        }
    }
    id
}

#[tauri::command]
pub fn get_wallet_balance() -> Result<WalletBalance, String> {
    Ok(WalletBalance {
        balance_cents: 0,
        network: "base".to_string(),
        address: "0x0000...0000".to_string(),
    })
}

#[tauri::command]
pub fn get_payment_history() -> Result<Vec<PaymentRecord>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub fn get_pending_402() -> Result<Vec<PendingPayment>, String> {
    let g = PENDING.read().map_err(|_| "lock")?;
    Ok(g.iter().cloned().collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletBalance {
    pub balance_cents: u64,
    pub network: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub id: String,
    pub amount_cents: u64,
    pub recipient: String,
    pub ts: i64,
}
