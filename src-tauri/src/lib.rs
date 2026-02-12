#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod detect;
mod evidence;
mod gateway_ws;
mod launcher;
mod mcp_guard;
mod openclaw_health;
mod policy;
mod proxy;
mod vault_store;
mod wallet;
mod x402;

use tracing::info;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Vault-0 proxy is ready.", name)
}

#[tauri::command]
fn get_proxy_status() -> Result<bool, String> {
    Ok(proxy::is_running())
}

#[tauri::command]
fn start_proxy() -> Result<(), String> {
    proxy::start().map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_proxy() -> Result<(), String> {
    proxy::stop().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_secret(alias: String, value: String) -> Result<(), String> {
    let mut state = proxy::state().write().map_err(|_| "state lock")?;
    state.vault.insert(alias, value);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("vault0_desktop=info".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_pty::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_proxy_status,
            start_proxy,
            stop_proxy,
            evidence::get_evidence_log,
            evidence::get_evidence_stats,
            evidence::export_receipt,
            policy::load_policy,
            policy::save_policy,
            set_secret,
            x402::get_wallet_balance,
            x402::get_payment_history,
            x402::get_pending_402,
            launcher::launch_agent,
            wallet::create_wallet,
            wallet::import_wallet,
            wallet::get_wallet_info,
            wallet::export_seed,
            detect::detect_openclaw,
            detect::secure_config_keys,
            detect::harden_openclaw,
            detect::launch_secure_agent,
            detect::scan_for_new_secrets,
            openclaw_health::check_openclaw_readiness,
            openclaw_health::check_gateway_health,
            vault_store::vault_exists,
            vault_store::vault_create,
            vault_store::vault_unlock,
            vault_store::vault_lock,
            vault_store::vault_is_unlocked,
            vault_store::vault_add_entry,
            vault_store::vault_list_entries,
            vault_store::vault_get_secret,
            vault_store::vault_delete_entry,
            vault_store::vault_delete_file,
            gateway_ws::gateway_connect,
            gateway_ws::gateway_disconnect,
            gateway_ws::gateway_status,
            gateway_ws::get_gateway_events,
        ])
        .setup(|_app| {
            info!("Vault-0 starting");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
