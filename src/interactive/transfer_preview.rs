use crate::{
    config::ConfigManager,
    types::network::{Network, NetworkConfig},
    utils::{
        eth::EthClient,
        helper::{Config as HelperConfig, WalletConfig},
    },
};
use anyhow::{Result, anyhow};
use console::style;
use dialoguer::Confirm;
use ethers::types::{Address, U256};
use ethers_providers::Middleware;

/// Helper function to convert wei to RBTC
fn convert_wei_to_rbtc(wei: U256) -> f64 {
    // 1 RBTC = 10^18 wei
    let wei_f64 = wei.as_u128() as f64;
    wei_f64 / 1_000_000_000_000_000_000.0
}

/// Displays transaction details and asks for confirmation
pub async fn show_transaction_preview(
    to: &str,
    amount: &str,
    network: Network,
) -> Result<bool> {
    println!("\n{}", style("Transaction Preview").bold().underlined());
    println!("• To: {}", style(to).cyan());
    
    // Parse amount
    let amount_wei = U256::from_dec_str(amount).map_err(|e| {
        anyhow::anyhow!("Invalid amount format: {}", e)
    })?;
    
    // Convert to RBTC for display
    let amount_rbtc = convert_wei_to_rbtc(amount_wei);
    println!("• Amount: {} RBTC ({} wei)", 
        style(amount_rbtc).green(), 
        style(amount_wei).dim()
    );
    
    // Get current config and initialize EthClient
    let config = ConfigManager::new()?.load()?;
    let helper_config = HelperConfig {
        network: NetworkConfig {
            name: config.default_network.to_string(),
            rpc_url: config.default_network.get_config().rpc_url,
            explorer_url: config.default_network.get_config().explorer_url,
        },
        wallet: WalletConfig {
            current_wallet_address: None,
            private_key: None,
            mnemonic: None,
        },
    };
    let eth_client = EthClient::new(&helper_config, None).await?;
    
    // Fetch current gas price from the network
    let gas_price = eth_client.provider()
        .get_gas_price()
        .await
        .map_err(|e| anyhow!("Failed to get gas price: {}", e))?;
        
    // Estimate gas for the transaction
    let to_address: Address = to.parse().map_err(|_| anyhow!("Invalid recipient address"))?;
    let estimated_gas = eth_client.estimate_gas(
        to_address,
        amount_wei,
        None, // No token address for native transfers
    ).await?;
    let gas_cost = gas_price.checked_mul(estimated_gas).unwrap_or_default();
    let gas_cost_rbtc = convert_wei_to_rbtc(gas_cost);
    
    println!("• Network: {}", style(network).cyan());
    println!("• Gas Price: {} Gwei", style(convert_wei_to_gwei(gas_price)).yellow());
    println!("• Estimated Gas: {}", style(estimated_gas).yellow());
    println!("• Estimated Fee: {} RBTC", style(gas_cost_rbtc).red());
    
    let total_amount = amount_wei.checked_add(gas_cost).unwrap_or(amount_wei);
    let total_rbtc = convert_wei_to_rbtc(total_amount);
    println!("• Total (Amount + Fee): {} RBTC", style(total_rbtc).green().bold());
    
    // Ask for confirmation
    let confirm = Confirm::new()
        .with_prompt("\nDo you want to send this transaction?")
        .default(false)
        .interact()?;
    
    Ok(confirm)
}

/// Helper function to convert wei to Gwei
fn convert_wei_to_gwei(wei: U256) -> f64 {
    let gwei = wei.as_u128() as f64 / 1_000_000_000.0;
    (gwei * 100.0).round() / 100.0 // Round to 2 decimal places
}
