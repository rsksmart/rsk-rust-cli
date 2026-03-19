use crate::types::contacts::Contact;
use crate::utils::secrets::{SecretPassword, SecretPrivateKey};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use anyhow::Result;
use anyhow::{Error, anyhow};
use base64::engine::general_purpose::STANDARD;
use base64::{self, Engine as _};
use chrono::Utc;
use alloy::primitives::{Address, U256};
use alloy::signers::local::PrivateKeySigner;
use rand::{RngCore, rngs::OsRng};
use scrypt::{Params, scrypt};
use zeroize::Zeroize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub address: Address,
    pub balance: U256,
    pub network: String,
    pub name: String,
    pub encrypted_private_key: String,
    pub salt: String,
    pub iv: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletData {
    pub current_wallet: String,
    pub wallets: HashMap<String, Wallet>,
    pub contacts: Vec<Contact>,
    pub api_key: Option<crate::utils::secrets::SecretString>,
}

// Note: Drop implementation removed - Zeroizing wrapper handles cleanup automatically

impl Wallet {
    pub fn address(&self) -> Address {
        self.address
    }

    pub fn new(wallet: PrivateKeySigner, name: &str, password: &SecretPassword) -> Result<Self, Error> {
        let mut private_key_bytes = wallet.to_bytes().to_vec();
        let (encrypted_key, iv, salt) =
            Self::encrypt_private_key(&private_key_bytes, password.expose())?;
        private_key_bytes.zeroize();
        Ok(Self {
            address: wallet.address(),
            balance: U256::ZERO,
            network: String::new(),
            name: name.to_string(),
            encrypted_private_key: STANDARD.encode(&encrypted_key),
            salt: STANDARD.encode(&salt),
            iv: STANDARD.encode(&iv),
            created_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn encrypt_private_key(
        private_key: &[u8],
        password: &str,
    ) -> anyhow::Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let mut nonce = [0u8; 12]; // GCM uses 12-byte nonce
        OsRng.fill_bytes(&mut nonce);
        let params = Params::recommended();
        let mut key = [0u8; 32];
        scrypt(password.as_bytes(), &salt, &params, &mut key)?;

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), private_key)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Zeroize sensitive data
        key.zeroize();

        Ok((ciphertext, nonce.to_vec(), salt.to_vec()))
    }

    pub fn decrypt_private_key(&self, password: &SecretPassword) -> Result<SecretPrivateKey, anyhow::Error> {
        // Decode Base64-encoded salt, nonce/IV, and encrypted key
        let salt = STANDARD
            .decode(&self.salt)
            .map_err(|e| anyhow!("Failed to decode salt: {}", e))?;
        let nonce_or_iv = STANDARD
            .decode(&self.iv)
            .map_err(|e| anyhow!("Failed to decode nonce/IV: {}", e))?;
        let encrypted_key = STANDARD
            .decode(&self.encrypted_private_key)
            .map_err(|e| anyhow!("Failed to decode encrypted private key: {}", e))?;

        // Validate lengths
        if salt.len() != 16 {
            return Err(anyhow!("Salt must be 16 bytes, got {} bytes", salt.len()));
        }

        // Derive the key using scrypt
        let mut key = [0u8; 32];
        let params = Params::recommended();
        scrypt(password.expose().as_bytes(), &salt, &params, &mut key)
            .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

        // Try GCM first (new format), fallback to CBC (legacy)
        let result = if nonce_or_iv.len() == 12 {
            // New GCM format
            let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
            let mut plaintext = cipher.decrypt(Nonce::from_slice(&nonce_or_iv), encrypted_key.as_ref())
                .map_err(|_| anyhow!("Incorrect password. Please try again."))?;

            if plaintext.len() != 32 {
                return Err(anyhow!("Decrypted private key has invalid length: {} bytes (expected 32)", plaintext.len()));
            }
            let result = format!("0x{}", hex::encode(&plaintext));
            plaintext.zeroize();
            result
        } else {
            return Err(anyhow!("Unsupported encryption format"));
        };

        // Zeroize sensitive data
        key.zeroize();

        Ok(SecretPrivateKey::new(result))
    }
}

impl fmt::Display for Wallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Name: {}\nAddress: {}\nNetwork: {}",
            self.name, self.address, self.network
        )
    }
}

impl Default for WalletData {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletData {
    /// Creates a new, empty wallet data structure.
    pub fn new() -> Self {
        Self {
            current_wallet: String::new(),
            wallets: HashMap::new(),
            contacts: Vec::new(),
            api_key: None,
        }
    }

    pub fn add_wallet(&mut self, wallet: Wallet) -> anyhow::Result<()> {
        let address = format!("0x{:x}", wallet.address);
        if self.wallets.contains_key(&address) {
            return Err(anyhow!("Wallet with address {} already exists", address));
        }
        self.wallets.insert(address.clone(), wallet);
        self.current_wallet = address;
        Ok(())
    }

    pub fn get_current_wallet(&self) -> Option<&Wallet> {
        self.wallets.get(&self.current_wallet)
    }

    pub fn switch_wallet(&mut self, address: &str) -> anyhow::Result<()> {
        if !self.wallets.contains_key(address) {
            return Err(anyhow!("Wallet with address {} not found", address));
        }
        self.current_wallet = address.to_string();
        Ok(())
    }

    pub fn get_wallet_by_name(&self, name: &str) -> Option<&Wallet> {
        self.wallets.values().find(|w| w.name == name)
    }

    pub fn remove_wallet(&mut self, address: &str) -> anyhow::Result<()> {
        if !self.wallets.contains_key(address) {
            return Err(anyhow!("Wallet with address {} not found", address));
        }
        if self.current_wallet == address {
            self.current_wallet = String::new();
        }
        self.wallets.remove(address);
        Ok(())
    }

    pub fn rename_wallet(&mut self, wallet: &Wallet, new_name: &str) -> anyhow::Result<()> {
        let address = format!("0x{:x}", wallet.address);
        if !self.wallets.contains_key(&address) {
            return Err(anyhow!("Wallet with address {} not found", address));
        }
        if let Some(w) = self.wallets.get_mut(&address) {
            w.name = new_name.to_string();
            Ok(())
        } else {
            Err(anyhow!("Failed to rename wallet {}", address))
        }
    }

    pub fn list_wallets(&self) -> Vec<&Wallet> {
        self.wallets.values().collect()
    }

    pub fn add_contact(&mut self, contact: Contact) -> anyhow::Result<()> {
        if self
            .contacts
            .iter()
            .any(|c| c.name == contact.name || c.address == contact.address)
        {
            return Err(anyhow!("Contact with name or address already exists"));
        }
        self.contacts.push(contact);
        Ok(())
    }

    pub fn remove_contact(&mut self, identifier: &str) -> anyhow::Result<()> {
        let index = self
            .contacts
            .iter()
            .position(|c| c.name == identifier || c.address.to_string() == identifier)
            .ok_or_else(|| anyhow!("Contact not found"))?;
        self.contacts.remove(index);
        Ok(())
    }

    pub fn update_contact(&mut self, identifier: &str, contact: Contact) -> anyhow::Result<()> {
        let index = self
            .contacts
            .iter()
            .position(|c| c.name == identifier || c.address.to_string() == identifier)
            .ok_or_else(|| anyhow!("Contact not found"))?;
        self.contacts[index] = contact;
        Ok(())
    }

    pub fn get_contact(&self, identifier: &str) -> Option<&Contact> {
        self.contacts
            .iter()
            .find(|c| c.name == identifier || c.address.to_string() == identifier)
    }

    pub fn search_contacts(&self, query: &str) -> Vec<&Contact> {
        self.contacts
            .iter()
            .filter(|c| {
                c.name.to_lowercase().contains(&query.to_lowercase())
                    || c.address.to_string().contains(query)
                    || c.notes.as_ref().is_some_and(|n| n.contains(query))
                    || c.tags.iter().any(|t| t.contains(query))
            })
            .collect()
    }
}
