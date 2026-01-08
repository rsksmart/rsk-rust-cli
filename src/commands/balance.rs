use crate::config::ConfigManager;
use crate::types::wallet::WalletData;
use crate::utils::constants;
use crate::utils::helper::Helper;
use crate::utils::table::TableBuilder;
use anyhow::{Result, anyhow};
use clap::Parser;
use alloy::primitives::Address;
use console;
use std::fs;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct BalanceCommand {
    /// Address to check balance for
    #[arg(long)]
    pub address: Option<String>,

    /// Optional Token to get Balance for
    #[arg(long)]
    pub token: Option<String>,
}

impl BalanceCommand {
    pub async fn execute(&self) -> Result<()> {
        // Load config to get the current network
        let config = ConfigManager::new()?.load()?;
        let network = config.default_network.to_string().to_lowercase();

        // Try to initialize eth client with graceful failure handling
        let eth_client_result = Helper::init_eth_client(&network).await;
        let (_config, eth_client) = match eth_client_result {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{}", console::style("❌ Network Error").red().bold());
                eprintln!("{}", console::style(format!("Failed to connect to network: {}", e)).red());
                eprintln!("{}", console::style("💡 Try again when internet connection is available").yellow());
                
                // Show offline wallet info instead
                return self.show_offline_info(&config).await;
            }
        };

        // Get address - use default wallet if none provided
        let address = if let Some(addr) = &self.address {
            Address::from_str(addr).map_err(|_| anyhow!("Invalid address format: {}", addr))?
        } else {
            // Load wallet data to get default wallet
            let wallet_file = constants::wallet_file_path();
            if !wallet_file.exists() {
                return Err(anyhow!(
                    "No wallets found. Please create or import a wallet first."
                ));
            }

            let data = fs::read_to_string(&wallet_file)?;
            let wallet_data = serde_json::from_str::<WalletData>(&data)?;
            let default_wallet = wallet_data.get_current_wallet()
                .ok_or_else(|| anyhow!("No default wallet selected. Please use 'wallet switch' to select a default wallet."))?;

            default_wallet.address
        };

        // Try to get balance with network error handling
        let balance_result = if let Some(token) = &self.token {
            // Check if it's the RBTC zero address
            if token == "0x0000000000000000000000000000000000000000" {
                eth_client.get_balance(&address, &None).await
                    .map(|balance| (balance, "RBTC".to_string()))
            } else {
                let token_address = Address::from_str(token)
                    .map_err(|_| anyhow!("Invalid token address format: {}", token))?;
                
                let balance_result = eth_client.get_balance(&address, &Some(token_address)).await;
                match balance_result {
                    Ok(balance) => {
                        // Try to get token info, but don't fail if we can't
                        let token_name = match eth_client.get_token_info(token_address).await {
                            Ok((_, symbol)) => symbol,
                            Err(_) => format!("Token (0x{})", &token[2..10]),
                        };
                        Ok((balance, token_name))
                    }
                    Err(e) => Err(e)
                }
            }
        } else {
            // Native RBTC balance
            eth_client.get_balance(&address, &None).await
                .map(|balance| (balance, "RBTC".to_string()))
        };

        let (balance, token_name) = match balance_result {
            Ok(result) => result,
            Err(e) => {
                eprintln!("{}", console::style("❌ Balance Check Failed").red().bold());
                eprintln!("{}", console::style(format!("Error: {}", e)).red());
                eprintln!("{}", console::style("💡 Check your internet connection and try again").yellow());
                return self.show_offline_info(&config).await;
            }
        };

        // Format the balance with appropriate decimals
        // All tokens including RBTC use 18 decimals
        let decimals = 18;
        let balance_str = alloy::primitives::utils::format_units(balance, decimals)
            .map_err(|e| anyhow!("Failed to format balance: {}", e))?;

        let mut table = TableBuilder::new();
        table.add_header(&["Address", "Network", "Token", "Balance"]);
        table.add_row(&[
            &Helper::format_address(&address),
            &config.default_network.to_string(),
            &token_name,
            &balance_str,
        ]);

        table.print();
        Ok(())
    }

    /// Show offline wallet information when network is unavailable
    async fn show_offline_info(&self, config: &crate::config::Config) -> Result<()> {
        println!("\n{}", console::style("📱 Offline Mode - Wallet Information").cyan().bold());
        println!("{}", "=".repeat(45));

        // Load wallet data
        let wallet_file = constants::wallet_file_path();
        if !wallet_file.exists() {
            return Err(anyhow!("No wallets found. Please create or import a wallet first."));
        }

        let data = fs::read_to_string(&wallet_file)?;
        let wallet_data = serde_json::from_str::<WalletData>(&data)?;

        let address = if let Some(addr) = &self.address {
            Address::from_str(addr).map_err(|_| anyhow!("Invalid address format: {}", addr))?
        } else {
            let default_wallet = wallet_data.get_current_wallet()
                .ok_or_else(|| anyhow!("No default wallet selected."))?;
            default_wallet.address
        };

        let mut table = TableBuilder::new();
        table.add_header(&["Address", "Network", "Status"]);
        table.add_row(&[
            &Helper::format_address(&address),
            &config.default_network.to_string(),
            "Offline - Balance unavailable",
        ]);

        table.print();
        println!("\n{}", console::style("💡 Connect to internet to check actual balance").dim());
        Ok(())
    }
}
