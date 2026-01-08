// src/utils/api.rs
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

const API_KEYS_FILE: &str = "api_keys.json";
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ApiKeys {
    pub alchemy_mainnet: Option<String>,
    pub alchemy_testnet: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Testnet => write!(f, "testnet"),
        }
    }
}

impl ApiKeys {
    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path).context("Failed to read API keys file")?;
            serde_json::from_str(&content).context("Failed to parse API keys")
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).context("Failed to save API keys")
    }

    pub fn get_alchemy_url(&self, network: Network) -> Result<String> {
        let api_key = match network {
            Network::Mainnet => self.alchemy_mainnet.as_ref(),
            Network::Testnet => self.alchemy_testnet.as_ref(),
        }
        .context("No API key found. Please set one using `config set-api-key`")?;

        Ok(format!(
            "https://rootstock-{}-g.alchemy.com/v2/{}",
            network, api_key
        ))
    }

    pub fn get_http_client() -> &'static Client {
        HTTP_CLIENT.get_or_init(Client::new)
    }

    fn get_config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .context("Could not find config directory")?
            .join("rsk-rust-cli");
        
        std::fs::create_dir_all(&path)?;
        path.push(API_KEYS_FILE);
        Ok(path)
    }
}

// Helper function to mask API keys in logs/output
pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let visible = 4;
    let masked = key.len().saturating_sub(visible * 2);
    format!(
        "{}{}{}",
        &key[..visible],
        "*".repeat(masked),
        &key[key.len() - visible..]
    )
}