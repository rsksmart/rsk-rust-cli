use crate::{
    commands::{tokens::TokenRegistry, transfer::TransferCommand},
    config::ConfigManager,
    types::wallet::WalletData,
    utils::{constants, secrets::SecretPassword},
};
use anyhow::{Result, anyhow};
use dialoguer::{Confirm, Input, Select};
use alloy::primitives::Address;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone)]
struct Transfer {
    to: Address,
    value: String, // Keep as string to avoid precision loss
    token_address: Option<String>,
    token_symbol: String,
}

#[derive(Debug, Deserialize)]
struct TransferInput {
    to: String,
    value: String,
    token: Option<String>, // Optional token address for JSON input
}

/// Interactive menu for bulk token transfers
pub async fn bulk_transfer() -> Result<()> {
    println!("\n💸 Bulk Token Transfer");
    println!("=====================");

    // Load config to get network
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;
    let network = config.default_network.to_string().to_lowercase();

    // Load token registry
    let registry = TokenRegistry::load().unwrap_or_default();
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

    // Let user select token
    let token_choices: Vec<String> = tokens.iter().map(|(name, _)| name.clone()).collect();
    let selected_token_name = Select::new()
        .with_prompt("Select token to send:")
        .items(&token_choices)
        .interact()?;

    let selected_token_name = &token_choices[selected_token_name];

    let (_, selected_token) = tokens
        .into_iter()
        .find(|(name, _)| name == selected_token_name)
        .ok_or_else(|| anyhow!("Selected token not found"))?;

    // Extract token symbol from display name
    let token_symbol = selected_token_name
        .split_whitespace()
        .next()
        .unwrap_or("UNKNOWN")
        .to_string();
        
    let token_address = if selected_token.address == "0x0000000000000000000000000000000000000000" {
        None
    } else {
        Some(selected_token.address.clone())
    };

    // Load wallet data
    let wallet_file = constants::wallet_file_path();
    let wallet_data = if wallet_file.exists() {
        let data = fs::read_to_string(&wallet_file)?;
        serde_json::from_str::<WalletData>(&data)?
    } else {
        return Err(anyhow!("No wallet found. Please create a wallet first."));
    };

    // Get current wallet
    let current_wallet = wallet_data
        .get_current_wallet()
        .ok_or_else(|| anyhow!("No active wallet found. Please select a wallet first."))?;

    // Prompt for password once at the beginning and validate it
    let password = SecretPassword::new(rpassword::prompt_password("Enter password for the wallet: ")?);

    // Validate password by trying to decrypt
    match current_wallet.decrypt_private_key(&password) {
        Ok(_) => {
            println!("✅ Password validated successfully");
        }
        Err(_) => {
            return Err(anyhow!("Incorrect password. Please try again."));
        }
    }

    // Ask if user wants to use a file or manual input
    let use_file = Confirm::new()
        .with_prompt("Do you want to load recipients from a JSON file?")
        .default(false)
        .interact()?;

    let transfers = if use_file {
        // Load transfers from file
        let file_path: String = Input::new()
            .with_prompt("Enter path to JSON file with transfer details")
            .interact_text()?;

        let file_content = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read file: {}", e))?;

        let transfer_inputs: Vec<TransferInput> = serde_json::from_str(&file_content)
            .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

        transfer_inputs
            .into_iter()
            .map(|input| {
                let to_addr = input
                    .to
                    .parse::<Address>()
                    .map_err(|e| anyhow!("Invalid address {}: {}", input.to, e))?;
                
                // Use token from JSON or default to selected token
                let transfer_token_address = input.token.or_else(|| token_address.clone());
                
                Ok(Transfer {
                    to: to_addr,
                    value: input.value,
                    token_address: transfer_token_address,
                    token_symbol: token_symbol.clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        // Manual input
        let count_str: String = Input::new()
            .with_prompt("How many recipients?")
            .validate_with(|input: &String| {
                if input.parse::<usize>().is_ok() {
                    Ok(())
                } else {
                    Err("Please enter a valid number".to_string())
                }
            })
            .interact_text()?;

        let count = count_str
            .parse::<usize>()
            .map_err(|_| anyhow!("Failed to parse number of recipients"))?;

        let mut transfers = Vec::with_capacity(count);
        for i in 0..count {
            println!("\nRecipient #{}:", i + 1);

            let to: String = Input::new()
                .with_prompt("Recipient address (0x...)")
                .validate_with(|input: &String| {
                    if input.starts_with("0x") && input.len() == 42 {
                        Ok(())
                    } else {
                        Err("Please enter a valid address starting with 0x".to_string())
                    }
                })
                .interact()?;

            let to = to
                .parse::<Address>()
                .map_err(|e| anyhow!("Invalid address: {}", e))?;

            let amount: String = Input::new()
                .with_prompt(&format!("Amount of {} to send (e.g., 1.0)", token_symbol))
                .interact()?;

            transfers.push(Transfer { 
                to, 
                value: amount,
                token_address: token_address.clone(),
                token_symbol: token_symbol.clone(),
            });
        }
        transfers
    };

    // Show summary
    println!("\n📋 Transaction Summary:");
    println!("====================");

    for (i, transfer) in transfers.iter().enumerate() {
        println!(
            "{:2}. To: {} - Amount: {} {}",
            i + 1,
            transfer.to,
            transfer.value,
            transfer.token_symbol
        );
    }

    println!("\nToken: {}", token_symbol);
    println!("Total transactions: {}", transfers.len());

    // Confirm before sending
    let confirm = Confirm::new()
        .with_prompt("\nDo you want to send these transactions?")
        .default(false)
        .interact()?;

    if !confirm {
        println!("Transaction cancelled");
        return Ok(());
    }

    // Send transactions using TransferCommand
    println!("\n🚀 Sending transactions...");

    let mut successful = 0;
    let mut failed = 0;

    for (i, transfer) in transfers.iter().enumerate() {
        print!("Sending {}/{}... ", i + 1, transfers.len());

        let transfer_cmd = TransferCommand {
            address: format!("{:?}", transfer.to),
            value: transfer.value.clone(),
            token: transfer.token_address.clone(),
        };

        match transfer_cmd.execute_with_password(Some(password.expose())).await {
            Ok(result) => {
                println!("✅ Success! Tx: {:?}", result.tx_hash);
                successful += 1;
            }
            Err(e) => {
                // Check if it's a password error and provide better message
                let error_msg = if e.to_string().contains("Incorrect password") {
                    "Incorrect password entered"
                } else {
                    &e.to_string()
                };
                println!("❌ Failed: {}", error_msg);
                failed += 1;
            }
        }

        // Small delay between transactions
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    println!("\n📊 Transaction Summary:");
    println!("====================");
    println!("Total transactions: {}", successful + failed);
    println!("✅ Successful: {}", successful);
    println!("❌ Failed: {}", failed);

    // password is automatically zeroized when it goes out of scope

    Ok(())
}



