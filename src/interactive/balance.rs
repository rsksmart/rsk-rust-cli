use crate::commands::balance::BalanceCommand;
use crate::commands::tokens::TokenRegistry;
use crate::config::ConfigManager;
use crate::types::wallet::WalletData;
use crate::utils::constants;
use crate::utils::table::TableBuilder;
use crate::utils::helper::Helper;
use anyhow::{Result, anyhow};
use console::style;
use inquire::Select;
use std::fs;

/// Displays the balance checking interface
pub async fn show_balance() -> Result<()> {
    println!("\n{}", style("💰 Check Balance").bold());
    println!("{}", "=".repeat(30));

    // Get the current network from config
    let config = ConfigManager::new()?.load()?;
    let network = config.default_network.to_string().to_lowercase();
    println!("Using network: {}", network);

    // Load token registry
    let registry = TokenRegistry::load()
        .map_err(|e| {
            eprintln!("⚠️  Warning: Could not load token registry: {}", e);
            e
        })
        .unwrap_or_default();

    // Get tokens for the current network
    let mut tokens = registry.list_tokens(Some(&network));

    // Add RBTC as the first option
    tokens.insert(
        0,
        (
            "RBTC (Native)".to_string(),
            crate::commands::tokens::TokenInfo {
                address: "0x0000000000000000000000000000000000000000".to_string(),
                decimals: 18,
            },
        ),
    );

    if tokens.is_empty() {
        return Err(anyhow!("No tokens found for {} network", network));
    }

    // Create a vector of (display_name, token_info) pairs
    let token_choices: Vec<(String, crate::commands::tokens::TokenInfo)> = tokens
        .into_iter()
        .filter(|(_, info)| {
            // Only include tokens that match the current network or are RBTC
            info.address == "0x0000000000000000000000000000000000000000"
                || registry
                    .list_tokens(Some(&network))
                    .iter()
                    .any(|(_, token_info)| token_info.address == info.address)
        })
        .collect();

    // Add back option to token list
    let mut token_display_names: Vec<String> =
        token_choices.iter().map(|(name, _)| name.clone()).collect();
    token_display_names.push("🏠 Back to Main Menu".to_string());

    // Let the user select which token to check
    let selection = loop {
        match Select::new("Select token to check balance:", token_display_names.clone()).prompt() {
            Ok(selection) => break selection,
            Err(_) => {
                // User pressed ESC - ask for confirmation
                use dialoguer::Confirm;
                let should_exit = Confirm::new()
                    .with_prompt("Return to main menu?")
                    .default(true)
                    .interact()
                    .unwrap_or(true);
                
                if should_exit {
                    return Ok(());
                }
                // Continue loop to show menu again
            }
        }
    };

    // Handle back option
    if selection == "🏠 Back to Main Menu" {
        return Ok(());
    }

    // Find the selected token info
    let (_, token_info) = token_choices
        .into_iter()
        .find(|(name, _)| name == &selection)
        .ok_or_else(|| anyhow!("Selected token not found"))?;

    // Clone the address since we need to use it in the command
    let token_address = token_info.address; // This is a String which is Clone

    // Execute the balance command
    let cmd = BalanceCommand {
        address: None, // Will use default wallet
        token: if token_address == "0x0000000000000000000000000000000000000000" {
            None
        } else {
            Some(token_address)
        },
    };

    cmd.execute().await
}

/// Displays offline balance information (wallet addresses only)
pub async fn show_offline_balance() -> Result<()> {
    println!("\n{}", style("💰 Check Balance (Offline Mode)").bold());
    println!("{}", "=".repeat(40));
    
    println!("{}", style("⚠️  Network connectivity required for balance checking").yellow());
    println!("{}", style("   Showing wallet information instead:").dim());
    println!();

    // Load wallet data
    let wallet_file = constants::wallet_file_path();
    if !wallet_file.exists() {
        return Err(anyhow!("No wallets found. Please create or import a wallet first."));
    }

    let data = fs::read_to_string(&wallet_file)?;
    let wallet_data = serde_json::from_str::<WalletData>(&data)?;
    
    if wallet_data.wallets.is_empty() {
        return Err(anyhow!("No wallets available."));
    }

    // Get current network for display
    let config = ConfigManager::new()?.load()?;
    
    let mut table = TableBuilder::new();
    table.add_header(&["Wallet Name", "Address", "Network", "Status"]);
    
    for (name, wallet) in &wallet_data.wallets {
        let is_current = wallet_data.current_wallet == *name;
        let status = if is_current { "Current" } else { "Available" };
        
        table.add_row(&[
            name,
            &Helper::format_address(&wallet.address),
            &config.default_network.to_string(),
            status,
        ]);
    }
    
    table.print();
    
    println!("\n{}", style("💡 Tip: Connect to internet to check actual balances").dim());
    Ok(())
}
