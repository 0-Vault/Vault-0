//! Encrypted vault for agent secrets.
//! Master passphrase -> Argon2id KDF -> AES-256-GCM encrypted file.
//! File: ~/Library/Application Support/Vault0/vault.enc

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString;
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use tracing::info;

const VAULT_DIR: &str = "Vault0";
const VAULT_FILE: &str = "vault.enc";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub alias: String,
    pub provider: String,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultHeader {
    salt_hex: String,
    argon2_m: u32,
    argon2_t: u32,
    argon2_p: u32,
    nonce_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultFile {
    header: VaultHeader,
    ciphertext_hex: String,
}

struct VaultState {
    entries: Vec<VaultEntry>,
    derived_key: [u8; KEY_LEN],
    unlocked: bool,
}

static VAULT: Lazy<RwLock<Option<VaultState>>> = Lazy::new(|| RwLock::new(None));

fn vault_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or("Cannot determine app data directory")?;
    Ok(base.join(VAULT_DIR))
}

fn vault_path() -> Result<PathBuf, String> {
    Ok(vault_dir()?.join(VAULT_FILE))
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    let argon2 = Argon2::default();
    let salt_str = SaltString::encode_b64(salt).map_err(|e| format!("salt encode: {e}"))?;
    let hash = argon2
        .hash_password(passphrase.as_bytes(), &salt_str)
        .map_err(|e| format!("argon2 hash: {e}"))?;
    let output = hash.hash.ok_or("argon2 produced no hash output")?;
    let bytes = output.as_bytes();
    if bytes.len() < KEY_LEN {
        return Err("argon2 output too short".into());
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&bytes[..KEY_LEN]);
    Ok(key)
}

pub fn encrypt_bytes_with_vault_key(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let guard = VAULT.read().map_err(|_| "vault lock")?;
    let state = guard.as_ref().ok_or("Vault is locked")?;
    let cipher = Aes256Gcm::new_from_slice(&state.derived_key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom(&mut nonce_bytes).map_err(|e| format!("nonce gen: {e}"))?;
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce_bytes), plaintext).map_err(|e| format!("encrypt: {e}"))?;
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn encrypt_entries(entries: &[VaultEntry], key: &[u8; KEY_LEN]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let plaintext = serde_json::to_vec(entries).map_err(|e| format!("serialize: {e}"))?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom(&mut nonce_bytes).map_err(|e| format!("nonce gen: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|e| format!("encrypt: {e}"))?;
    Ok((nonce_bytes.to_vec(), ciphertext))
}

fn decrypt_entries(ciphertext: &[u8], nonce: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<VaultEntry>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce = Nonce::from_slice(nonce);
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| "Decryption failed. Wrong passphrase?".to_string())?;
    let entries: Vec<VaultEntry> = serde_json::from_slice(&plaintext).map_err(|e| format!("deserialize: {e}"))?;
    Ok(entries)
}

fn write_vault_file(salt: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<(), String> {
    let dir = vault_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let file = VaultFile {
        header: VaultHeader {
            salt_hex: hex::encode(salt),
            argon2_m: 65536,
            argon2_t: 3,
            argon2_p: 1,
            nonce_hex: hex::encode(nonce),
        },
        ciphertext_hex: hex::encode(ciphertext),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("serialize file: {e}"))?;
    let path = vault_path()?;
    fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
    info!("Vault file written to {}", path.display());
    Ok(())
}

fn read_vault_file() -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    let path = vault_path()?;
    let json = fs::read_to_string(&path).map_err(|e| format!("read vault: {e}"))?;
    let file: VaultFile = serde_json::from_str(&json).map_err(|e| format!("parse vault: {e}"))?;
    let salt = hex::decode(&file.header.salt_hex).map_err(|e| format!("decode salt: {e}"))?;
    let nonce = hex::decode(&file.header.nonce_hex).map_err(|e| format!("decode nonce: {e}"))?;
    let ciphertext = hex::decode(&file.ciphertext_hex).map_err(|e| format!("decode ciphertext: {e}"))?;
    Ok((salt, nonce, ciphertext))
}

#[tauri::command]
pub fn vault_exists() -> bool {
    vault_path().map(|p| p.exists()).unwrap_or(false)
}

#[tauri::command]
pub fn vault_create(passphrase: String) -> Result<(), String> {
    if passphrase.len() < 12 {
        return Err("Passphrase must be at least 12 characters".into());
    }
    let mut salt = [0u8; 16];
    getrandom(&mut salt).map_err(|e| format!("salt gen: {e}"))?;
    let key = derive_key(&passphrase, &salt)?;
    let entries: Vec<VaultEntry> = Vec::new();
    let (nonce, ciphertext) = encrypt_entries(&entries, &key)?;
    write_vault_file(&salt, &nonce, &ciphertext)?;
    let mut guard = VAULT.write().map_err(|_| "vault lock")?;
    *guard = Some(VaultState {
        entries,
        derived_key: key,
        unlocked: true,
    });
    info!("Vault created and unlocked");
    Ok(())
}

#[tauri::command]
pub fn vault_unlock(passphrase: String) -> Result<(), String> {
    let (salt, nonce, ciphertext) = read_vault_file()?;
    let key = derive_key(&passphrase, &salt)?;
    let entries = decrypt_entries(&ciphertext, &nonce, &key)?;
    let mut guard = VAULT.write().map_err(|_| "vault lock")?;
    *guard = Some(VaultState {
        entries,
        derived_key: key,
        unlocked: true,
    });
    info!("Vault unlocked ({} entries)", guard.as_ref().unwrap().entries.len());
    Ok(())
}

#[tauri::command]
pub fn vault_lock() -> Result<(), String> {
    let mut guard = VAULT.write().map_err(|_| "vault lock")?;
    *guard = None;
    info!("Vault locked");
    Ok(())
}

#[tauri::command]
pub fn vault_is_unlocked() -> bool {
    VAULT.read().map(|g| g.as_ref().map(|v| v.unlocked).unwrap_or(false)).unwrap_or(false)
}

#[tauri::command]
pub fn vault_add_entry(alias: String, value: String, provider: String) -> Result<(), String> {
    let mut guard = VAULT.write().map_err(|_| "vault lock")?;
    let state = guard.as_mut().ok_or("Vault is locked")?;
    state.entries.retain(|e| e.alias != alias);
    state.entries.push(VaultEntry {
        alias,
        provider,
        value,
        created_at: chrono_now(),
    });
    let (nonce, ciphertext) = encrypt_entries(&state.entries, &state.derived_key)?;
    let (salt, _, _) = read_vault_file()?;
    write_vault_file(&salt, &nonce, &ciphertext)?;
    Ok(())
}

#[derive(Serialize)]
pub struct VaultEntryInfo {
    pub alias: String,
    pub provider: String,
    pub preview: String,
    pub created_at: String,
}

#[tauri::command]
pub fn vault_list_entries() -> Result<Vec<VaultEntryInfo>, String> {
    let guard = VAULT.read().map_err(|_| "vault lock")?;
    let state = guard.as_ref().ok_or("Vault is locked")?;
    Ok(state.entries.iter().map(|e| {
        let preview = if e.value.len() > 6 {
            format!("{}...{}", &e.value[..3], &e.value[e.value.len()-3..])
        } else {
            "****".to_string()
        };
        VaultEntryInfo {
            alias: e.alias.clone(),
            provider: e.provider.clone(),
            preview,
            created_at: e.created_at.clone(),
        }
    }).collect())
}

#[tauri::command]
pub fn vault_get_secret(alias: String) -> Result<String, String> {
    let guard = VAULT.read().map_err(|_| "vault lock")?;
    let state = guard.as_ref().ok_or("Vault is locked")?;
    state.entries.iter().find(|e| e.alias == alias)
        .map(|e| e.value.clone())
        .ok_or(format!("No entry with alias '{alias}'"))
}

#[tauri::command]
pub fn vault_delete_entry(alias: String) -> Result<(), String> {
    let mut guard = VAULT.write().map_err(|_| "vault lock")?;
    let state = guard.as_mut().ok_or("Vault is locked")?;
    state.entries.retain(|e| e.alias != alias);
    let (nonce, ciphertext) = encrypt_entries(&state.entries, &state.derived_key)?;
    let (salt, _, _) = read_vault_file()?;
    write_vault_file(&salt, &nonce, &ciphertext)?;
    Ok(())
}

#[tauri::command]
pub fn vault_delete_file() -> Result<(), String> {
    let path = vault_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("delete vault: {e}"))?;
        info!("Vault file deleted: {}", path.display());
    }
    let mut guard = VAULT.write().map_err(|_| "vault lock")?;
    *guard = None;
    Ok(())
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

