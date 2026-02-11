use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::proxy;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub allow_domains: Vec<String>,
    pub block_domains: Vec<String>,
    pub spend_cap_cents: Option<u64>,
    pub output_redact_patterns: Vec<String>,
    #[serde(default)]
    pub auto_settle_402: bool,
}

#[tauri::command]
pub fn load_policy(path: Option<String>) -> Result<Policy, String> {
    let path = path.or_else(|| Some(default_policy_path()));
    let path = path.as_deref().unwrap_or("");
    if path.is_empty() || !Path::new(path).exists() {
        return Ok(Policy::default());
    }
    let s = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let policy: Policy = serde_yaml::from_str(&s).map_err(|e| e.to_string())?;
    {
        let mut state = proxy::state().write().map_err(|_| "state lock")?;
        state.policy = policy.clone();
    }
    Ok(policy)
}

#[tauri::command]
pub fn save_policy(path: Option<String>, policy: Policy) -> Result<(), String> {
    let path = path.or_else(|| Some(default_policy_path()));
    let path = path.as_deref().unwrap_or("");
    if path.is_empty() {
        let mut state = proxy::state().write().map_err(|_| "state lock")?;
        state.policy = policy;
        return Ok(());
    }
    let s = serde_yaml::to_string(&policy).map_err(|e| e.to_string())?;
    fs::write(path, s).map_err(|e| e.to_string())?;
    let mut state = proxy::state().write().map_err(|_| "state lock")?;
    state.policy = policy;
    Ok(())
}

pub fn default_hardened_policy() -> Policy {
    Policy {
        allow_domains: vec![
            "api.openai.com".into(),
            "api.anthropic.com".into(),
            "api.x.ai".into(),
            "generativelanguage.googleapis.com".into(),
        ],
        block_domains: vec![
            "169.254.169.254".into(),
        ],
        spend_cap_cents: Some(1000),
        output_redact_patterns: vec![
            "sk-[a-zA-Z0-9]{20,}".into(),
            "Bearer [a-zA-Z0-9._-]+".into(),
        ],
        auto_settle_402: false,
    }
}

fn default_policy_path() -> String {
    dirs::config_dir()
        .map(|p| p.join("vault0").join("policy.yaml"))
        .and_then(|p| {
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            p.into_os_string().into_string().ok()
        })
        .unwrap_or_else(|| "policy.yaml".to_string())
}
