use anyhow::Result;
use console::style;
use dialoguer::Input;

use crate::{commands::tx::TxCommand, config::ConfigManager, types::network::Network, interactive::config::show_config_menu};

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
                if !input.starts_with("0x") {
                    return Err("Transaction hash must start with '0x'");
                }
                if input.len() != 66 {
                    return Err("Transaction hash must be exactly 66 characters (0x + 64 hex chars)");
                }
                if !input[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err("Transaction hash contains invalid characters (only 0-9, a-f, A-F allowed)");
                }
                Ok(())
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
                if error_msg.contains("No API key found") {
                    println!("\n{}", style("❌ API Configuration Required").red().bold());
                    println!("To check transaction status, you need to configure an API key.");
                    println!("\n{}", style("Available options:").bold());
                    println!("  • RSK Public Node (free)");
                    println!("  • Alchemy API (requires signup)");
                    
                    println!("\n{}", style("To configure:").bold());
                    println!("  1. Run: {} or {}", 
                        style("rsk-rust-cli config").cyan(),
                        style("wallet config").cyan()
                    );
                    println!("  2. Select your preferred API provider");
                    println!("  3. Enter your API key or endpoint");
                    
                    let setup_now = dialoguer::Confirm::new()
                        .with_prompt("\nWould you like to set up API configuration now?")
                        .default(true)
                        .interact()?;

                    if setup_now {
                        show_config_menu().await?;
                        continue; // Return to transaction input after config
                    }
                    break;
                } else if error_msg.contains("not found") || error_msg.contains("does not exist") {
                    println!(
                        "\n{}",
                        style("❌ Transaction not found or still pending.").yellow()
                    );
                    println!("The transaction might still be in the mempool or may have failed.");

                    println!(
                        "\n{}",
                        style("💡 Tip: Transactions usually take 15-30 seconds to be mined.").dim()
                    );
                } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
                    println!("\n{}", style("❌ Network timeout").red());
                    println!("The request took too long. Check your internet connection.");
                } else if error_msg.contains("dns") || error_msg.contains("resolve") || error_msg.contains("No such host") {
                    println!("\n{}", style("❌ DNS Resolution Failed").red());
                    println!("Cannot resolve the API endpoint. Check your internet connection.");
                } else if error_msg.contains("Connection refused") || error_msg.contains("unreachable") {
                    println!("\n{}", style("❌ Connection Failed").red());
                    println!("Cannot connect to the network. You may be offline.");
                    println!("\n{}", style("💡 Try:").blue());
                    println!("  • Check your internet connection");
                    println!("  • Verify firewall settings");
                    println!("  • Try again in a few moments");
                } else if error_msg.contains("401") || error_msg.contains("403") || error_msg.contains("invalid") && error_msg.contains("key") {
                    println!("\n{}", style("❌ Invalid API Key").red());
                    println!("Your API key appears to be invalid or expired.");
                    println!("Please update your configuration with a valid API key.");
                } else if error_msg.contains("Request failed") {
                    println!("\n{}", style("❌ Network Error").red());
                    println!("Failed to connect to the API. Check your internet connection.");
                } else {
                    println!("\n{}", style("❌ Error checking transaction status:").red());
                    println!("{}", error_msg);
                }

                // Ask if user wants to try again (except for API key errors)
                if !error_msg.contains("No API key found") {
                    let try_again = dialoguer::Confirm::new()
                        .with_prompt("Would you like to try again?")
                        .default(true)
                        .interact()?;

                    if !try_again {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}
