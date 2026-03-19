use crate::api::{ApiKey, ApiProvider};
use crate::config::ConfigManager;
use crate::types::wallet::WalletData;
use crate::utils::api_validator::{validate_api_key, validate_api_key_format, ValidationResult};
use crate::utils::constants;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::fs;

#[derive(Parser)]
pub struct SetApiKeyCommand {
    /// API key to set
    #[arg(long, required = true)]
    pub api_key: String,
}

// Custom Debug implementation that redacts the API key
impl std::fmt::Debug for SetApiKeyCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetApiKeyCommand")
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl SetApiKeyCommand {
    pub async fn execute(&self) -> Result<()> {
        // Get current network from config
        let config = ConfigManager::new()?.load()?;
        let network = config.default_network.to_string().to_lowercase();
        
        // For now, assume RSK RPC provider (can be extended later)
        let provider = ApiProvider::RskRpc;
        
        // Validate format first
        if let Err(e) = validate_api_key_format(&provider, &self.api_key) {
            println!("{}: {}", "Format Error".red().bold(), e);
            return Ok(());
        }

        // Create API key for validation
        let api_key = ApiKey {
            key: crate::utils::secrets::SecretString::new(self.api_key.clone()),
            network: network.clone(),
            provider: provider.clone(),
            name: None,
        };

        println!("🔍 Validating API key for {} {}...", provider, network);

        // Validate the key
        match validate_api_key(&api_key).await? {
            ValidationResult::Valid => {
                println!("{}: API key is valid", "✅ Success".green().bold());
                
                // Save the key
                let wallet_file = constants::wallet_file_path();
                let mut wallet_data = if wallet_file.exists() {
                    let data = fs::read_to_string(&wallet_file)?;
                    serde_json::from_str::<WalletData>(&data)?
                } else {
                    WalletData::new()
                };

                wallet_data.api_key = Some(crate::utils::secrets::SecretString::new(self.api_key.clone()));
                crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
                println!("{}: API key saved successfully", "💾 Saved".green().bold());
            }
            ValidationResult::Invalid(reason) => {
                println!("{}: {}", "❌ Invalid".red().bold(), reason);
                println!("💡 Please check your API key and try again");
            }
            ValidationResult::NetworkError(error) => {
                println!("{}: {}", "⚠️ Network Error".yellow().bold(), error);
                println!("💡 Saving key anyway - validation will retry when network is available");
                
                // Save anyway for offline use
                let wallet_file = constants::wallet_file_path();
                let mut wallet_data = if wallet_file.exists() {
                    let data = fs::read_to_string(&wallet_file)?;
                    serde_json::from_str::<WalletData>(&data)?
                } else {
                    WalletData::new()
                };

                wallet_data.api_key = Some(crate::utils::secrets::SecretString::new(self.api_key.clone()));
                crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
                println!("{}: API key saved (unvalidated)", "💾 Saved".yellow().bold());
            }
        }
        
        Ok(())
    }
}
