use crate::{
    commands::{
        contacts::{ContactsAction, ContactsCommand},
        tokens::TokenRegistry,
        transfer::TransferCommand,
    },
    config::ConfigManager,
    interactive::transfer_preview,
};
use anyhow::{Context, Result, anyhow};
use colored::*;
use console::style;
use inquire::{Select, Text, validator::Validation};

/// Displays the fund transfer interface
pub async fn send_funds() -> Result<()> {
    println!("\n{}", style("💸 Send Funds").bold());
    println!("{}", "=".repeat(30));

    // Get the current network from config
    let config = ConfigManager::new()?.load()?;
    let network = config.default_network.to_string().to_lowercase();
    println!("Using network: {}", network);

    // Ask user if they want to select from contacts or enter address manually
    let send_options = vec!["📝 Enter address manually", "👥 Select from contacts"];

    let send_choice =
        Select::new("How would you like to specify the recipient?", send_options).prompt()?;

    let to = if send_choice == "👥 Select from contacts" {
        // Load contacts
        let cmd = ContactsCommand {
            action: ContactsAction::List,
        };
        let contacts = cmd.load_contacts()?;

        if contacts.is_empty() {
            println!("No contacts available. Please enter the address manually.");
            get_recipient_address()?
        } else {
            // Show contact selection
            let contact_names: Vec<String> = contacts
                .iter()
                .map(|c| {
                    format!(
                        "{} (0x{:x}) - {}",
                        c.name,
                        c.address,
                        c.notes.as_deref().unwrap_or("No notes")
                    )
                })
                .collect();

            let selection = Select::new("Select contact:", contact_names)
                .prompt()
                .context("Failed to select contact")?;

            // Extract the address from the selection (it's in the format "Name (0x...)")
            let addr_start = selection.find('(').unwrap_or(0) + 1;
            let addr_end = selection.find(')').unwrap_or(selection.len());
            selection[addr_start..addr_end].to_string()
        }
    } else {
        get_recipient_address()?
    };

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

    // Get just the display names for the selection menu
    let mut token_display_names: Vec<String> =
        token_choices.iter().map(|(name, _)| name.clone()).collect();
    token_display_names.push("🏠 Back to Main Menu".to_string());

    // Let the user select which token to send
    let selection = loop {
        match Select::new("Select token to send:", token_display_names.clone()).prompt() {
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
    let (display_name, token_info) = token_choices
        .into_iter()
        .find(|(name, _)| name == &selection)
        .ok_or_else(|| anyhow!("Selected token not found"))?;

    // Extract the token symbol (remove the (Native) suffix if present)
    let token_symbol = display_name
        .split_whitespace()
        .next()
        .unwrap_or(&display_name)
        .to_string();

    let amount = loop {
        let input = inquire::Text::new(&format!("Amount of {} to send:", token_symbol))
            .with_help_message("Enter the amount to send")
            .with_validator(|input: &str| {
                if input.parse::<f64>().is_ok() {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid("Please enter a valid number".into()))
                }
            })
            .prompt()?;

        // Convert RBTC to wei for preview
        let rbtc: f64 = input.parse().unwrap_or(0.0);
        let wei = (rbtc * 1e18) as u128;

        // Show preview and ask for confirmation
        let confirmed = transfer_preview::show_transaction_preview(
            &to,
            &wei.to_string(),
            config.default_network,
            &display_name,
        )
        .await?;

        if confirmed {
            break input;
        } else {
            println!("Transaction cancelled. Please enter a new amount or press Ctrl+C to exit.");
        }
    };

    // Clone the address since we need to use it multiple times
    let token_address = token_info.address.clone();
    let _token = if token_address == "0x0000000000000000000000000000000000000000" {
        None
    } else {
        Some(token_address.clone())
    };

    // Show transaction summary
    println!("\n{}", style("📝 Transaction Summary").bold());
    println!("{}", "=".repeat(30));
    println!("To: {}", to);
    println!("Token: {}", token_symbol);
    println!("Amount: {} {}", amount, token_symbol);
    println!("Network: {}", network);

    // Confirm transaction
    let confirm = inquire::Confirm::new("Confirm transaction?")
        .with_default(false)
        .prompt()?;

    if !confirm {
        println!("Transaction cancelled");
        return Ok(());
    }

    // Execute the transfer command
    let cmd = TransferCommand {
        address: to.clone(),
        value: amount.clone(),
        token: if token_address == "0x0000000000000000000000000000000000000000" {
            None
        } else {
            Some(token_address.clone())
        },
    };

    match cmd.execute().await {
        Ok(result) => {
            println!(
                "\n{}: Transaction confirmed! Tx Hash: {}",
                "Success".green().bold(),
                result.tx_hash
            );
            
            let explorer_url = if network.to_string().to_lowercase().contains("testnet") {
                format!("https://explorer.testnet.rsk.co/tx/{:x}", result.tx_hash)
            } else {
                format!("https://explorer.rsk.co/tx/{:x}", result.tx_hash)
            };
            
            println!("🔗 View on Explorer: {}", explorer_url);
        }
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Insufficient") {
                println!("{}", style("❌ Transaction Failed").red().bold());
                println!("{}", error_msg);
                println!("💡 Please check your balance and try again with a smaller amount.");
            } else if error_msg.contains("gas") {
                println!("{}", style("⛽ Gas Error").yellow().bold());
                println!("{}", error_msg);
                println!("💡 Try again when network conditions improve.");
            } else {
                println!("{}", style("❌ Transaction Failed").red().bold());
                println!("Error: {}", error_msg);
                println!("💡 Please check your inputs and network connection.");
            }
            return Ok(());
        }
    }

    Ok(())
}

/// Helper function to get recipient address with validation
fn get_recipient_address() -> Result<String> {
    Text::new("Recipient address (0x...):")
        .with_help_message("Enter the Ethereum address to send to")
        .with_validator(|input: &str| {
            if input.starts_with("0x") && input.len() == 42 {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Please enter a valid Ethereum address (0x...)".into(),
                ))
            }
        })
        .prompt()
        .map_err(Into::into)
}
