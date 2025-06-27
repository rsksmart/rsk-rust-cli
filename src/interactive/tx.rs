use anyhow::Result;
use console::style;
use dialoguer::Input;

use crate::{
    commands::tx::TxCommand,
    config::ConfigManager,
    types::network::Network,
};

/// Interactive transaction status checker
pub async fn check_transaction_status() -> Result<()> {
    loop {
        println!("\n{}", style("🔍 Check Transaction Status").bold().cyan());
        println!("{}", "=".repeat(30));

        // Get the current network from config
        let config = ConfigManager::new()?.load()?;
        let (_, is_testnet) = match config.default_network {
            Network::RootStockMainnet => ("mainnet", false),
            Network::RootStockTestnet => ("testnet", true),
            _ => ("testnet", true), // Default to testnet if not specified
        };

        // Get transaction hash from user
        let input = Input::new()
            .with_prompt("Enter transaction hash (0x...) or 'q' to go back")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.to_lowercase() == "q" {
                    return Ok(());
                }
                if input.starts_with("0x") && input.len() == 66 {
                    Ok(())
                } else {
                    Err("Please enter a valid transaction hash (0x followed by 64 hex characters) or 'q' to go back")
                }
            })
            .interact_text()?;

        if input.to_lowercase() == "q" {
            return Ok(());
        }
        
        let tx_hash = input;

        // Create and execute the transaction status command
        let cmd = TxCommand {
            tx_hash: tx_hash.clone(),
            testnet: is_testnet,
            api_key: None, // Will use the configured API key
        };

        println!("\n{}", style("⏳ Fetching transaction status...").dim());
        
        match cmd.execute().await {
            Ok(_) => {
                // Offer to check another transaction
                let check_another = dialoguer::Confirm::new()
                    .with_prompt("\nCheck another transaction?")
                    .default(false)
                    .interact()?;
                
                if !check_another {
                    break;
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("not found") || error_msg.contains("does not exist") {
                    println!("\n{}", style("❌ Transaction not found or still pending.").yellow());
                    println!("The transaction might still be in the mempool or may have failed.");
                    
                    println!("\n{}", style("💡 Tip: Transactions usually take 15-30 seconds to be mined.").dim());
                } else {
                    println!("\n{}", style("❌ Error checking transaction status:").red());
                    println!("{}", error_msg);
                }
                
                // Ask if user wants to try again
                let try_again = dialoguer::Confirm::new()
                    .with_prompt("Would you like to try again?")
                    .default(true)
                    .interact()?;
                
                if !try_again {
                    break;
                }
            }
        }
    }

    Ok(())
}
