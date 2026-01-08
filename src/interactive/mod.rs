//! Interactive command-line interface for the Rootstock wallet

mod balance;
mod bulk_transfer;
mod config;
mod contacts;
mod history;
mod system;
mod tokens;
mod transfer;
mod transfer_preview;
mod tx;
mod wallet;

use crate::utils::constants;
use anyhow::Result;
use console::style;
use dialoguer::{Select, theme::ColorfulTheme};

// Re-export public functions
pub use self::{
    balance::{show_balance, show_offline_balance}, bulk_transfer::bulk_transfer, config::show_config_menu,
    contacts::manage_contacts, history::show_history, system::system_menu, tokens::token_menu,
    transfer::send_funds, tx::check_transaction_status, wallet::create_wallet_with_name,
    wallet::wallet_menu,
};

// Import for network status display
use crate::config::ConfigManager;
use crate::utils::network::{check_connectivity, NetworkStatus};

// Import Network from the types module
use crate::types::network::Network;

// Re-export the Network type for consistency
pub use crate::types::network::Network as ConfigNetwork;

// Helper function to get styled network status
fn get_network_status(network: Network) -> console::StyledObject<&'static str> {
    match network {
        Network::Mainnet => style("🔗 Mainnet").cyan(),
        Network::Testnet => style("🔗 Testnet").yellow(),
        Network::Regtest => style("🔗 Regtest").magenta(),
        Network::AlchemyMainnet => style("🔗 Alchemy Mainnet").blue(),
        Network::AlchemyTestnet => style("🔗 Alchemy Testnet").blue(),
        Network::RootStockMainnet => style("🔗 Rootstock Mainnet").green(),
        Network::RootStockTestnet => style("🔗 Rootstock Testnet").green(),
    }
}

/// Starts the interactive CLI interface
pub async fn start() -> Result<()> {
    // Clear the screen for a fresh start
    clearscreen::clear().ok();

    // Display welcome banner
    println!(
        "\n{}",
        style("🌐 Rootstock Wallet").bold().blue().underlined()
    );
    println!(
        "{}",
        style("Your Gateway to the Rootstock Blockchain").dim()
    );
    println!("{}\n", "-".repeat(40));

    // Check network connectivity
    let network_status = check_connectivity().await;
    let is_online = network_status == NetworkStatus::Online;

    // Display current status
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;

    if is_online {
        println!("  {}", style("🟢 Online").green());
    } else {
        println!("  {}", style("🔴 Offline").red());
    }
    println!("  {}", get_network_status(config.default_network));

    // Check if wallet data file exists and count wallets
    let wallet_file = constants::wallet_file_path();
    let wallet_count = if wallet_file.exists() {
        match std::fs::read_to_string(&wallet_file) {
            Ok(contents) => {
                match serde_json::from_str::<crate::types::wallet::WalletData>(&contents) {
                    Ok(wallet_data) => wallet_data.wallets.len(),
                    Err(_) => 0,
                }
            }
            Err(_) => 0,
        }
    } else {
        0
    };

    let wallet_text = match wallet_count {
        0 => "💼 No wallets loaded".to_string(),
        1 => "💼 1 wallet loaded".to_string(),
        _ => format!("💼 {} wallets loaded", wallet_count),
    };
    println!("  {}\n", style(wallet_text).dim());

    if !is_online {
        println!("  {}", style("ℹ️  Limited functionality available offline").yellow());
        println!();
    }

    loop {
        let mut options = vec![];
        let mut option_map = vec![];

        // Add options based on network status
        if is_online {
            options.push(format!("{}  Check Balance", style("💰").bold().green()));
            option_map.push(0);
            options.push(format!("{}  Send Funds", style("💸").bold().yellow()));
            option_map.push(1);
            options.push(format!("{}  Bulk Transfer", style("📤").bold().yellow()));
            option_map.push(2);
            options.push(format!("{}  Check Transaction Status", style("🔍").bold().cyan()));
            option_map.push(3);
            options.push(format!("{}  Transaction History", style("📜").bold().cyan()));
            option_map.push(4);
        } else {
            options.push(format!("{}  Check Balance {}", style("💰").bold().dim(), style("(offline)").dim()));
            option_map.push(0);
        }

        // Always available options
        options.push(format!("{}  Wallet Management", style("🔑").bold().blue()));
        option_map.push(5);
        options.push(format!("{}  Token Management", style("🪙").bold().magenta()));
        option_map.push(6);
        options.push(format!("{}  Contact Management", style("📇").bold().cyan()));
        option_map.push(7);
        options.push(format!("{}  Configuration", style("⚙️").bold().white()));
        option_map.push(8);
        options.push(format!("{}  System", style("💻").bold().cyan()));
        option_map.push(9);
        options.push(format!("{}  Exit", style("🚪").bold().red()));
        option_map.push(10);

        let selection = match Select::with_theme(&ColorfulTheme::default())
            .with_prompt("\nWhat would you like to do?")
            .items(&options)
            .default(0)
            .interact() {
                Ok(selection) => selection,
                Err(_) => {
                    // User pressed ESC or interrupted - ask for confirmation
                    use dialoguer::Confirm;
                    let should_exit = Confirm::new()
                        .with_prompt("Are you sure you want to exit?")
                        .default(false)
                        .interact()
                        .unwrap_or(false);
                    
                    if should_exit {
                        println!("👋 Goodbye!");
                        return Ok(());
                    } else {
                        continue; // Go back to menu
                    }
                }
            };

        match option_map[selection] {
            0 => {
                if is_online {
                    show_balance().await?;
                } else {
                    show_offline_balance().await?;
                }
            },
            1 => send_funds().await?,
            2 => bulk_transfer().await?,
            3 => check_transaction_status().await?,
            4 => show_history().await?,
            5 => wallet_menu().await?,
            6 => token_menu().await?,
            7 => manage_contacts().await?,
            8 => show_config_menu().await?,
            9 => system_menu().await?,
            10 => {
                println!("\n👋 Goodbye!");
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
