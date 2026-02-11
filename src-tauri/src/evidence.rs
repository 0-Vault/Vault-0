use once_cell::sync::Lazy;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::sync::RwLock;

const LOG_CAP: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts: String,
    pub kind: String,
    pub msg: String,
}

static LOG: Lazy<RwLock<VecDeque<LogEntry>>> = Lazy::new(|| RwLock::new(VecDeque::new()));

pub fn push(kind: &str, msg: &str) {
    let ts = chrono_ts();
    let entry = LogEntry {
        ts: ts.clone(),
        kind: kind.to_string(),
        msg: msg.to_string(),
    };
    if let Ok(mut g) = LOG.write() {
        g.push_back(entry);
        while g.len() > LOG_CAP {
            g.pop_front();
        }
    }
}

fn chrono_ts() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let millis = d.subsec_millis();
            format!("{}.{:03}", secs, millis)
        })
        .unwrap_or_else(|_| "0.000".to_string())
}

#[tauri::command]
pub fn get_evidence_log() -> Result<Vec<LogEntry>, String> {
    let g = LOG.read().map_err(|_| "lock")?;
    Ok(g.iter().cloned().collect())
}

#[derive(Debug, serde::Serialize)]
pub struct EvidenceStats {
    pub total: usize,
    pub allowed: usize,
    pub blocked: usize,
    pub payment: usize,
}

#[tauri::command]
pub fn get_evidence_stats() -> Result<EvidenceStats, String> {
    let g = LOG.read().map_err(|_| "lock")?;
    let mut allowed = 0;
    let mut blocked = 0;
    let mut payment = 0;
    for e in g.iter() {
        match e.kind.as_str() {
            "allowed" => allowed += 1,
            "blocked" => blocked += 1,
            "payment" => payment += 1,
            _ => {}
        }
    }
    Ok(EvidenceStats {
        total: g.len(),
        allowed,
        blocked,
        payment,
    })
}

#[derive(Debug, Serialize)]
pub struct ReceiptEntry {
    pub ts: String,
    pub kind: String,
    pub msg: String,
    pub hash: String,
}

fn hash_entry(ts: &str, kind: &str, msg: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ts.as_bytes());
    hasher.update(kind.as_bytes());
    hasher.update(msg.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
pub fn export_receipt(entries: Vec<(String, String, String)>) -> Result<Vec<ReceiptEntry>, String> {
    let out: Vec<ReceiptEntry> = entries
        .into_iter()
        .map(|(ts, kind, msg)| {
            let hash = hash_entry(&ts, &kind, &msg);
            ReceiptEntry { ts, kind, msg, hash }
        })
        .collect();
    Ok(out)
}
