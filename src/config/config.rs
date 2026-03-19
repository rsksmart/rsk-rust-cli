use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dirs;
use serde::{Deserialize, Serialize};

// Re-export the API types for easier access
pub use crate::api::{ApiConfig, ApiKey, ApiProvider};
use crate::types::network::Network;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_network: Network,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alchemy_mainnet_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alchemy_testnet_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_wallet: Option<String>,
}

impl Config {
    /// Get the appropriate API key for the current network and provider
    pub fn get_api_key(&self, provider: &ApiProvider) -> Option<&str> {
        let network_str = match self.default_network {
            Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => "mainnet",
            Network::Testnet
            | Network::AlchemyTestnet
            | Network::RootStockTestnet
            | Network::Regtest => "testnet",
        };

        // First try to get from the new API config
        if let Some(key) = self
            .api
            .keys
            .iter()
            .find(|k| &k.provider == provider && k.network == network_str)
        {
            return Some(key.key.expose());
        }

        // Fall back to legacy keys for backward compatibility (Alchemy only)
        match (provider, network_str) {
            (ApiProvider::Alchemy, "mainnet") => self.alchemy_mainnet_key.as_deref(),
            (ApiProvider::Alchemy, "testnet") => self.alchemy_testnet_key.as_deref(),
            _ => None,
        }
    }

    /// Get RSK RPC API key for blockchain operations
    pub fn get_rsk_rpc_key(&self) -> Option<&str> {
        self.get_api_key(&ApiProvider::RskRpc)
    }

    /// Get Alchemy API key for transaction history
    pub fn get_alchemy_key(&self) -> Option<&str> {
        self.get_api_key(&ApiProvider::Alchemy)
    }

    /// Add or update an API key
    pub fn set_api_key(
        &mut self,
        provider: ApiProvider,
        key: String,
        name: Option<String>,
    ) -> String {
        let network = match self.default_network {
            Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => "mainnet",
            _ => "testnet",
        };

        let display_name = name.as_deref().unwrap_or("unnamed");

        // Create and add the API key
        let api_key = ApiKey {
            key: crate::utils::secrets::SecretString::new(key.clone()),
            network: network.to_string(),
            provider: provider.clone(),
            name: name.clone(),
        };

        // Add to the API keys list
        self.api.keys.push(api_key);

        // Also update the legacy fields for backward compatibility
        match (provider.clone(), network) {
            (ApiProvider::Alchemy, "mainnet") => self.alchemy_mainnet_key = Some(key),
            (ApiProvider::Alchemy, _) => self.alchemy_testnet_key = Some(key),
            _ => {}
        }

        format!(
            "API key for {} on {} saved as '{}'",
            provider, network, display_name
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_network: Network::Testnet,
            api: ApiConfig::default(),
            alchemy_mainnet_key: None,
            alchemy_testnet_key: None,
            default_wallet: None,
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("rsk-rust-cli");

        std::fs::create_dir_all(&config_dir)?;

        Ok(Self {
            config_path: config_dir.join("config.json"),
        })
    }

    pub fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            return Ok(Config::default());
        }

        let content =
            fs::read_to_string(&self.config_path).context("Failed to read config file")?;

        serde_json::from_str(&content).context("Failed to parse config file")
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&self.config_path, content).context("Failed to write config file")
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn ensure_configured(&self) -> Result<()> {
        let config = self.load()?;

        match config.default_network {
            Network::Mainnet if config.alchemy_mainnet_key.is_none() => {
                anyhow::bail!(
                    "Mainnet API key not configured. Please run `setup` or `config set alchemy-mainnet-key <key>`"
                );
            }
            Network::Testnet if config.alchemy_testnet_key.is_none() => {
                anyhow::bail!(
                    "Testnet API key not configured. Please run `setup` or `config set alchemy-testnet-key <key>`"
                );
            }
            _ => Ok(()),
        }
    }

    /// Removes all wallet data, configuration, and cache
    /// WARNING: This will delete ALL wallet data and cannot be undone!
    pub fn clear_cache(&self) -> Result<()> {
        use std::fs;

        // Clear config directory
        let config_dir = self
            .config_path()
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid config directory path"))?;

        if config_dir.exists() {
            // Remove all files in the config directory
            for entry in fs::read_dir(config_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }

                println!("Removed: {}", path.display());
            }

            // Recreate the empty directory
            fs::create_dir_all(config_dir)?;
        }

        // Clear wallet data directory
        if let Some(data_dir) = dirs::data_local_dir() {
            let wallet_data_dir = data_dir.join("rsk-rust-cli");
            if wallet_data_dir.exists() {
                // Remove all files in the wallet data directory
                for entry in fs::read_dir(&wallet_data_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_dir() {
                        fs::remove_dir_all(&path)?;
                    } else {
                        fs::remove_file(&path)?;
                    }

                    println!("Removed: {}", path.display());
                }

                // Remove the wallet data directory itself
                fs::remove_dir(&wallet_data_dir)?;
                println!("Removed: {}", wallet_data_dir.display());
            }
        }

        println!("\n✅ Cache and all wallet data have been cleared successfully.");
        println!("A new configuration will be created when you start the wallet again.");

        Ok(())
    }
}
