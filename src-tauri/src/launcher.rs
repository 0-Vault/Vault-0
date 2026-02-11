use crate::evidence;
use std::collections::HashMap;
use std::process::Command;

const PROXY_ADDR: &str = "http://127.0.0.1:3840";

/// Launch an agent script with HTTP_PROXY / HTTPS_PROXY set to the Vault-0 proxy.
#[tauri::command]
pub fn launch_agent(script_path: String) -> Result<String, String> {
    if !crate::proxy::is_running() {
        return Err("Proxy must be running before launching an agent.".to_string());
    }

    let path = std::path::Path::new(&script_path);
    if !path.exists() {
        return Err(format!("Script not found: {}", script_path));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (program, args): (&str, Vec<&str>) = match ext.as_str() {
        "py" => ("python3", vec![&script_path]),
        "js" | "mjs" => ("node", vec![&script_path]),
        "ts" => ("npx", vec!["tsx", &script_path]),
        "sh" => ("sh", vec![&script_path]),
        _ => return Err(format!("Unsupported file type: .{}", ext)),
    };

    let mut env: HashMap<String, String> = std::env::vars().collect();
    env.insert("HTTP_PROXY".to_string(), PROXY_ADDR.to_string());
    env.insert("HTTPS_PROXY".to_string(), PROXY_ADDR.to_string());
    env.insert("http_proxy".to_string(), PROXY_ADDR.to_string());
    env.insert("https_proxy".to_string(), PROXY_ADDR.to_string());

    let child = Command::new(program)
        .args(&args)
        .envs(&env)
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;

    let pid = child.id();
    evidence::push(
        "info",
        &format!("Launched agent {} (pid {}) via {}", script_path, pid, program),
    );

    Ok(format!("Agent launched (pid {})", pid))
}
