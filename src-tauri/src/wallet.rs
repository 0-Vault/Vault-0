//! macOS Keychain-backed EVM wallet using alloy-signer-local.
//! Mnemonic stored only in Keychain; metadata (address) in wallet.json.

use alloy_primitives::{Address, B256, U256};
use alloy_signer::Signer;
use alloy_signer_local::{
    coins_bip39::{English, Mnemonic},
    MnemonicBuilder, PrivateKeySigner,
};
use alloy_sol_types::{eip712_domain, sol, SolStruct};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

sol! {
    struct TransferWithAuthorization {
        address from;
        address to;
        uint256 value;
        uint256 validAfter;
        uint256 validBefore;
        bytes32 nonce;
    }
}

const KEYRING_SERVICE: &str = "vault0-wallet";
const KEYRING_USER: &str = "mnemonic";
const WALLET_DIR: &str = "vault0";
const WALLET_META: &str = "wallet.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletMeta {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletInfo {
    pub has_wallet: bool,
    pub address: String,
    pub balance_cents: u64,
    pub network: String,
}

/// One-time return of recovery phrase when creating a wallet.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWalletResult {
    pub info: WalletInfo,
    pub recovery_phrase: String,
}

fn wallet_dir() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|p| p.join(WALLET_DIR))
        .ok_or_else(|| "Config dir not found".to_string())
}

fn meta_path() -> Result<PathBuf, String> {
    Ok(wallet_dir()?.join(WALLET_META))
}

fn keychain_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())
}

fn save_mnemonic(phrase: &str) -> Result<(), String> {
    keychain_entry()?
        .set_password(phrase)
        .map_err(|e| e.to_string())
}

fn load_mnemonic() -> Result<String, String> {
    keychain_entry()?
        .get_password()
        .map_err(|e| e.to_string())
}

fn signer_from_phrase(phrase: &str) -> Result<PrivateKeySigner, String> {
    MnemonicBuilder::<English>::default()
        .phrase(phrase)
        .build()
        .map_err(|e| e.to_string())
}

fn address_string(addr: Address) -> String {
    format!("{:#x}", addr)
}

#[tauri::command]
pub fn create_wallet() -> Result<CreateWalletResult, String> {
    let dir = wallet_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let mut rng = rand::thread_rng();
    let mnemonic = Mnemonic::<English>::new_with_count(&mut rng, 12).map_err(|e| e.to_string())?;
    let phrase = mnemonic.to_phrase();

    let signer = signer_from_phrase(&phrase)?;

    save_mnemonic(&phrase)?;

    let address = address_string(signer.address());

    let meta = WalletMeta {
        address: address.clone(),
    };
    let meta_p = meta_path()?;
    fs::write(
        &meta_p,
        serde_json::to_string(&meta).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(CreateWalletResult {
        info: WalletInfo {
            has_wallet: true,
            address,
            balance_cents: 0,
            network: "base".to_string(),
        },
        recovery_phrase: phrase,
    })
}

#[tauri::command]
pub fn import_wallet(mnemonic_phrase: String) -> Result<WalletInfo, String> {
    let phrase = mnemonic_phrase.trim();
    let signer = signer_from_phrase(phrase)?;

    save_mnemonic(phrase)?;

    let address = address_string(signer.address());

    let dir = wallet_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let meta = WalletMeta {
        address: address.clone(),
    };
    fs::write(
        meta_path()?,
        serde_json::to_string(&meta).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(WalletInfo {
        has_wallet: true,
        address,
        balance_cents: 0,
        network: "base".to_string(),
    })
}

#[tauri::command]
pub fn get_wallet_info() -> Result<WalletInfo, String> {
    let meta_p = meta_path()?;
    if !meta_p.exists() {
        return Ok(WalletInfo {
            has_wallet: false,
            address: String::new(),
            balance_cents: 0,
            network: "base".to_string(),
        });
    }
    let s = fs::read_to_string(&meta_p).map_err(|e| e.to_string())?;
    let meta: WalletMeta = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    Ok(WalletInfo {
        has_wallet: true,
        address: meta.address,
        balance_cents: 0,
        network: "base".to_string(),
    })
}

#[tauri::command]
pub fn export_seed() -> Result<String, String> {
    load_mnemonic()
}

/// Sign an x402 payment intent (EIP-3009 TransferWithAuthorization).
/// Called by the proxy when auto_settle_402 is enabled. Returns the signature as hex.
pub async fn sign_x402_payment(
    amount_cents: u64,
    recipient: String,
    network: String,
) -> Result<String, String> {
    let phrase = load_mnemonic()?;
    let signer = signer_from_phrase(&phrase)?;
    let from = signer.address();

    let to = recipient
        .parse::<Address>()
        .map_err(|_| "Invalid recipient address".to_string())?;

    let chain_id: u64 = match network.as_str() {
        "base" => 8453,
        "base-sepolia" => 84532,
        _ => 8453,
    };

    let domain = eip712_domain! {
        name: "USD Coin",
        version: "2",
        chain_id: chain_id,
    };

    let valid_after = U256::ZERO;
    let valid_before = U256::from(u64::MAX);
    let mut nonce_bytes = [0u8; 32];
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| e.to_string())?;
    let nonce = B256::from(nonce_bytes);

    let value = U256::from(amount_cents);

    let payload = TransferWithAuthorization {
        from,
        to,
        value,
        validAfter: valid_after,
        validBefore: valid_before,
        nonce,
    };

    let hash = payload.eip712_signing_hash(&domain);
    let sig = signer.sign_hash(&hash).await.map_err(|e| e.to_string())?;
    Ok(format!("0x{}", hex::encode(sig.as_bytes())))
}
