use crate::config::ConfigManager;
use crate::types::network::{Network, NetworkConfig};
use crate::utils::eth::EthClient;
use anyhow::Result;
use colored::Colorize;
use alloy::primitives::Address;
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct Config {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
}

#[derive(Debug, Clone, Default, zeroize::Zeroize)]
pub struct WalletConfig {
    #[zeroize(skip)]
    pub current_wallet_address: Option<String>,
    pub private_key: Option<Zeroizing<String>>,
    pub mnemonic: Option<Zeroizing<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                name: "RSK Mainnet".to_string(),
                rpc_url: "https://public-node.rsk.co".to_string(),
                explorer_url: "https://explorer.rsk.co".to_string(),
            },
            wallet: WalletConfig {
                current_wallet_address: None,
                private_key: None,
                mnemonic: None,
            },
        }
    }
}

pub struct Helper;

impl Helper {
    pub async fn init_eth_client(network: &str) -> Result<(Config, EthClient)> {
        let network_enum = Network::from_str(network).unwrap_or(Network::Mainnet);

        // Load configuration to get API keys
        let config_manager = ConfigManager::new()?;
        let app_config = config_manager.load()?;

        // Get API keys from config
        let rsk_api_key = app_config.get_rsk_rpc_key();
        let alchemy_api_key = app_config.get_alchemy_key();

        // Get the appropriate RPC URL with API key preference
        let rpc_url = network_enum.get_rpc_url_with_key(rsk_api_key, alchemy_api_key);

        // Create network config with the selected RPC URL
        let mut net_cfg = network_enum.get_config();
        net_cfg.rpc_url = rpc_url.clone();

        let mut config = Config::default();
        config.network = net_cfg.clone();

        // Log which RPC endpoint is being used
        let rpc_type = if rsk_api_key.is_some() {
            "RSK RPC API"
        } else if alchemy_api_key.is_some() {
            "Alchemy API"
        } else {
            "Public Node"
        };

        println!(
            "[rsk-rust-cli] Connected to {} at {} ({})",
            config.network.name,
            config.network.rpc_url,
            rpc_type.dimmed()
        );

        let eth_client = EthClient::new(&config, None).await?;
        Ok((config, eth_client))
    }

    pub fn format_network(network: &str) -> String {
        match network.to_lowercase().as_str() {
            "mainnet" => format!("{}", "Mainnet".yellow().bold()),
            "testnet" => format!("{}", "Testnet".blue().bold()),
            _ => network.to_string(),
        }
    }

    pub fn format_address(address: &Address) -> String {
        format!("{}{}", "0x".green(), address.to_string()[2..].green())
    }

    pub fn format_balance(balance: u128, as_tokens: bool) -> Result<String> {
        if as_tokens {
            Ok(format!(
                "{} RBTC",
                alloy::primitives::utils::format_units(balance, 18)?
            ))
        } else {
            Ok(format!("{} wei", balance))
        }
    }

    pub fn format_tx_status(status: Option<u64>) -> String {
        match status {
            Some(1) => format!("{}", "Success".green().bold()),
            Some(0) => format!("{}", "Failed".red().bold()),
            None => format!("{}", "Pending".yellow().bold()),
            _ => format!("{}", "Unknown".yellow().bold()),
        }
    }
}
