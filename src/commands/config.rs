use anyhow::Result;
use clap::{Args, Subcommand};
use console::style;

use crate::config::{Config, ConfigManager, Network};

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[clap(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Show current configuration
    Show,
    
    /// Set a configuration value
    Set {
        /// Configuration key to set (e.g., "default-network", "alchemy-mainnet-key")
        key: String,
        
        /// Value to set
        value: String,
    },
    
    /// Run the setup wizard
    Setup,
    
    /// Run diagnostics
    Doctor,
}

impl ConfigCommand {
    pub async fn execute(&self) -> Result<()> {
        let config_manager = ConfigManager::new()?;
        
        match &self.command {
            ConfigSubcommand::Show => self.show_config(&config_manager).await,
            ConfigSubcommand::Set { key, value } => self.set_config(&config_manager, key, value).await,
            ConfigSubcommand::Setup => {
                crate::config::run_setup_wizard()?;
                Ok(())
            }
            ConfigSubcommand::Doctor => {
                crate::config::run_doctor()?;
                Ok(())
            }
        }
    }

    async fn show_config(&self, config_manager: &ConfigManager) -> Result<()> {
        let config = config_manager.load()?;
        
        println!("\n{}", style("Current Configuration:").bold().cyan());
        println!("{}", "=".repeat(60));
        
        println!("\n{}", style("🌐 Network").bold());
        println!("  Default network: {}", config.default_network);
        
        println!("\n{}", style("🔑 API Keys").bold());
        println!(
            "  Mainnet API key: {}",
            config.alchemy_mainnet_key
                .as_deref()
                .map(|_| "********".to_string())
                .unwrap_or_else(|| style("Not set").dim().to_string())
        );
        println!(
            "  Testnet API key: {}",
            config.alchemy_testnet_key
                .as_deref()
                .map(|_| "********".to_string())
                .unwrap_or_else(|| style("Not set").dim().to_string())
        );
        
        if let Some(wallet) = &config.default_wallet {
            println!("\n{}", style("💼 Wallet").bold());
            println!("  Default wallet: {}", wallet);
        }
        
        println!("\n{}", style("Paths").bold());
        println!("  Config file: {}", config_manager.config_path().display());
        
        Ok(())
    }

    async fn set_config(&self, config_manager: &ConfigManager, key: &str, value: &str) -> Result<()> {
        let mut config = config_manager.load()?;
        
        match key.to_lowercase().as_str() {
            "default-network" => {
                let network = value.parse()?;
                config.default_network = network;
                println!("Set default network to: {}", network);
            }
            "alchemy-mainnet-key" => {
                config.alchemy_mainnet_key = Some(value.to_string());
                println!("Set Alchemy Mainnet API key");
            }
            "alchemy-testnet-key" => {
                config.alchemy_testnet_key = Some(value.to_string());
                println!("Set Alchemy Testnet API key");
            }
            "default-wallet" => {
                config.default_wallet = Some(value.to_string());
                println!("Set default wallet to: {}", value);
            }
            _ => anyhow::bail!("Unknown configuration key: {}", key),
        }
        
        config_manager.save(&config)?;
        Ok(())
    }
}