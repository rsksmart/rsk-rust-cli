use crate::types::wallet::WalletData;
use crate::utils::constants;
use crate::utils::helper::Helper;
use crate::utils::table::TableBuilder;
use anyhow::{Result, anyhow};
use clap::Parser;
use ethers::types::Address;
use std::fs;
use std::str::FromStr;
use crate::config::ConfigManager;

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
        
        let (_config, eth_client) = Helper::init_eth_client(&network).await?;

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

        let (balance, token_name) = if let Some(token) = &self.token {
            // Check if it's the RBTC zero address
            if token == "0x0000000000000000000000000000000000000000" {
                let balance = eth_client.get_balance(&address, &None).await?;
                (balance, "RBTC".to_string())
            } else {
                let token_address = Address::from_str(token)
                    .map_err(|_| anyhow!("Invalid token address format: {}", token))?;
                let balance = eth_client.get_balance(&address, &Some(token_address)).await?;
                
                // Try to get token info, but don't fail if we can't
                let token_name = match eth_client.get_token_info(token_address).await {
                    Ok((_, symbol)) => symbol,
                    Err(_) => format!("Token (0x{})", &token[2..10]),
                };
                
                (balance, token_name)
            }
        } else {
            // Native RBTC balance
            let balance = eth_client.get_balance(&address, &None).await?;
            (balance, "RBTC".to_string())
        };

        // Format the balance with appropriate decimals
        let decimals = if token_name == "RBTC" { 18 } else { 18 }; // Default to 18 for tokens
        let balance_str = ethers::utils::format_units(balance, decimals)
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
}
