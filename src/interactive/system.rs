use crate::config::ConfigManager;
use crate::types::network::Network;
use crate::utils::eth::EthClient;
use crate::utils::helper::Config;
use crate::utils::terminal::{self, show_version};
use anyhow::Result;
use console::style;
use dialoguer::{Select, theme::ColorfulTheme};
use alloy::providers::Provider;
use std::io;
use std::time::Duration;

/// Helper function to get styled network status
fn get_network_status(network: &Network) -> String {
    match network {
        Network::Mainnet => style(format!("🌐 {}", network)).green().bold().to_string(),
        Network::Testnet => style(format!("🔧 {}", network)).yellow().bold().to_string(),
        _ => style(format!("❓ {}", network)).white().bold().to_string(),
    }
}

/// Helper function to get configuration status
fn get_config_status(has_key: bool) -> String {
    if has_key {
        style("✓ Configured").green().to_string()
    } else {
        style("✗ Not configured").red().to_string()
    }
}

/// Get current block number from the network
async fn get_block_number(eth_client: &EthClient) -> Result<u64> {
    let block_number = eth_client
        .provider()
        .get_block_number()
        .await
        .map_err(|_| anyhow::anyhow!("Failed to get block number"))?;
    Ok(block_number)
}

/// Get current gas price from the network
async fn get_gas_price(eth_client: &EthClient) -> Result<u128> {
    eth_client
        .provider()
        .get_gas_price()
        .await
        .map_err(|_| anyhow::anyhow!("Failed to get gas price"))
}

/// Check network health by measuring block time
async fn check_network_health(eth_client: &EthClient) -> Result<String> {
    let start_block = get_block_number(eth_client).await?;
    tokio::time::sleep(Duration::from_secs(2)).await; // Wait 2 seconds
    let end_block = get_block_number(eth_client).await?;

    let block_diff = end_block.saturating_sub(start_block);

    Ok(match block_diff {
        0 => "🟡 Idle (no new blocks in 2s)".to_string(),
        1 => "🟢 Healthy (1 new block in 2s)".to_string(),
        _ => format!("🟢 Very Healthy ({} new blocks in 2s)", block_diff),
    })
}

/// Display system information including network status and API key configuration
async fn show_system_info() -> Result<()> {
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;

    println!("\n{}", style("System Information").bold().underlined());
    println!("• Version: {}", style(env!("CARGO_PKG_VERSION")).cyan());
    println!("• Network: {}", get_network_status(&config.default_network));

    // Show configuration status
    match config.default_network {
        Network::Mainnet => {
            let has_key = config
                .alchemy_mainnet_key
                .as_ref()
                .map_or(false, |k| !k.is_empty());
            println!("• Service Configuration: {}", get_config_status(has_key));
        }
        Network::Testnet => {
            let has_key = config
                .alchemy_testnet_key
                .as_ref()
                .map_or(false, |k| !k.is_empty());
            println!("• Service Configuration: {}", get_config_status(has_key));
        }
        _ => {}
    }

    // Show network details if connected
    println!("\n{}", style("Network Status").bold().underlined());

    // Create an EthClient to fetch network info
    let helper_config = Config {
        network: config.default_network.get_config(),
        wallet: Default::default(),
    };

    match EthClient::new(&helper_config, None).await {
        Ok(eth_client) => {
            // Get current block number
            match get_block_number(&eth_client).await {
                Ok(block_number) => println!("• Current Block: {}", style(block_number).cyan()),
                Err(_) => println!("• Current Block: {}", style("Unavailable").red().bold()),
            }

            // Get gas price
            match get_gas_price(&eth_client).await {
                Ok(gas_price) => {
                    let gwei = gas_price as f64 / 1_000_000_000.0;
                    println!(
                        "• Current Gas Price: {} Gwei",
                        style(format!("{:.2}", gwei)).yellow()
                    );
                }
                Err(_) => println!("• Current Gas Price: {}", style("Unavailable").red().bold()),
            }

            // Check network health
            match check_network_health(&eth_client).await {
                Ok(health) => println!("• Network Health: {}", health),
                Err(_) => println!("• Network Health: {}", style("Unavailable").red().bold()),
            }
        }
        Err(e) => {
            println!("• Network Status: {}", style("Disconnected").red().bold());
            println!("  {}", style(format!("Error: {}", e)).dim());
        }
    }

    println!();
    Ok(())
}

/// System menu for various system-related commands
pub async fn system_menu() -> Result<()> {
    loop {
        let options = vec![
            format!("{}  Clear Screen", style("🧹").bold().cyan()),
            format!("{}  Show Version", style("ℹ️").bold().blue()),
            format!("{}  Network Status", style("🌐").bold().green()),
            format!("{}  Back to Main Menu", style("⬅️").bold().white()),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("\nSystem Menu")
            .items(&options)
            .default(0)
            .interact()?;

        let result = match selection {
            0 => {
                terminal::clear_screen();
                Ok(())
            }
            1 => {
                show_version();
                Ok(())
            }
            2 => show_system_info().await,
            3 => break,
            _ => Ok(()),
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            continue;
        }

        if selection < 3 {
            // Don't pause after "Back"
            println!("\nPress Enter to continue...");
            let _ = io::stdin().read_line(&mut String::new())?;
        }
    }

    Ok(())
}
