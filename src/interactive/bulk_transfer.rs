use crate::{
    types::{
        wallet::WalletData,
        network::Network,
    },
    config::ConfigManager,
    utils::constants,
};
use anyhow::{anyhow, Result};
use dialoguer::{Confirm, Input};
use ethers::{
    middleware::SignerMiddleware,
    prelude::*,
    providers::{Http, Provider},
    signers::LocalWallet,
    types::{Address, U256},
};
use serde::Deserialize;
use std::{fs, sync::Arc};

#[derive(Debug, Clone)]
struct Transfer {
    to: Address,
    value: U256,
}

#[derive(Debug, Deserialize)]
struct TransferInput {
    to: String,
    value: String,
}

/// Interactive menu for bulk token transfers
pub async fn bulk_transfer() -> Result<()> {
    println!("\n💸 Bulk Token Transfer");
    println!("=====================");

    // Load wallet data
    let wallet_file = constants::wallet_file_path();
    let wallet_data = if wallet_file.exists() {
        let data = fs::read_to_string(&wallet_file)?;
        serde_json::from_str::<WalletData>(&data)?
    } else {
        return Err(anyhow!("No wallet found. Please create a wallet first."));
    };

    // Get current wallet
    let current_wallet = wallet_data.get_current_wallet()
        .ok_or_else(|| anyhow!("No active wallet found. Please select a wallet first."))?;

    // Load config
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;
    
    // Get the network configuration
    let network_config = config.default_network.get_config();
    
    // Get the chain ID based on the network
    let chain_id = match config.default_network {
        Network::RootStockMainnet => 30,
        Network::RootStockTestnet => 31,
        Network::Mainnet => 30,
        Network::Testnet => 31,
        Network::Regtest => 1337,
        _ => return Err(anyhow!("Unsupported network for bulk transfers")),
    };
    
    // Prompt for password to decrypt the private key
    let password = rpassword::prompt_password("Enter password for the wallet: ")?;
    
    // Decrypt the private key
    let private_key = current_wallet.decrypt_private_key(&password)?;
    
    // Create a wallet with the chain ID
    let wallet = private_key
        .parse::<LocalWallet>()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?
        .with_chain_id(chain_id as u64);
    
    // Create a provider with the network RPC URL
    let provider = Provider::<Http>::try_from(&network_config.rpc_url)
        .map_err(|e| anyhow!("Failed to connect to RPC: {}", e))?;
    
    // Create a signer middleware with the provider and wallet
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

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
        
        transfer_inputs.into_iter().map(|input| {
            let to_addr = input.to.parse::<Address>()
                .map_err(|e| anyhow!("Invalid address {}: {}", input.to, e))?;
            let value_wei = parse_amount(&input.value)?;
            Ok(Transfer { to: to_addr, value: value_wei })
        }).collect::<Result<Vec<_>>>()?
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
            
        let count = count_str.parse::<usize>()
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
                        Err("Please enter a valid Ethereum address starting with 0x".to_string())
                    }
                })
                .interact()?;
            
            let to = to.parse::<Address>()
                .map_err(|e| anyhow!("Invalid address: {}", e))?;
            
            let amount: String = Input::new()
                .with_prompt("Amount to send (e.g., 1.0)")
                .interact()?;
            
            let value = parse_amount(&amount)?;
            
            transfers.push(Transfer { to, value });
        }
        transfers
    };

    // Show summary
    println!("\n📋 Transaction Summary:");
    println!("====================");
    let total = transfers.iter().fold(U256::zero(), |acc, t| acc + t.value);
    
    for (i, transfer) in transfers.iter().enumerate() {
        println!("{:2}. To: {} - Amount: {} ETH", 
            i + 1, 
            transfer.to,
            format_eth(transfer.value)
        );
    }
    
    println!("\nTotal to send: {} ETH", format_eth(total));
    
    // Get current gas price
    let gas_price = client.get_gas_price().await?;
    println!("Current gas price: {} Gwei", format_gwei(gas_price));
    
    // Estimate gas cost (21,000 gas per basic transfer)
    let gas_per_tx = U256::from(21000u64);
    let total_gas = gas_per_tx.checked_mul(U256::from(transfers.len())).unwrap_or_default();
    let total_gas_cost = total_gas.checked_mul(gas_price).unwrap_or_default();
    
    println!("Estimated gas cost: {} ETH", format_eth(total_gas_cost));
    println!("Total cost (amount + gas): {} ETH", format_eth(total + total_gas_cost));
    
    // Confirm before sending
    let confirm = Confirm::new()
        .with_prompt("\nDo you want to send these transactions?")
        .default(false)
        .interact()?;
    
    if !confirm {
        println!("Transaction cancelled");
        return Ok(());
    }
    
    // Send transactions
    println!("\n🚀 Sending transactions...");
    
    let mut successful = 0;
    let mut failed = 0;
    
    for (i, transfer) in transfers.clone().into_iter().enumerate() {
        print!("Sending {}/{}... ", i + 1, transfers.clone().len());
        
        let tx = ethers::types::TransactionRequest::new()
            .to(transfer.to)
            .value(transfer.value)
            .gas(gas_per_tx)
            .gas_price(gas_price);
        
        match client.send_transaction(tx, None).await {
            Ok(pending_tx) => {
                match pending_tx.await {
                    Ok(Some(receipt)) => {
                        if receipt.status == Some(1.into()) {
                            println!("✅ Success! Tx: {:?}", receipt.transaction_hash);
                            successful += 1;
                        } else {
                            println!("❌ Failed! Tx: {:?}", receipt.transaction_hash);
                            failed += 1;
                        }
                    },
                    Ok(None) => {
                        println!("❌ Transaction was dropped from the mempool");
                        failed += 1;
                    },
                    Err(e) => {
                        println!("❌ Error: {}", e);
                        failed += 1;
                    }
                }
            },
            Err(e) => {
                println!("❌ Failed to send transaction: {}", e);
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
    
    Ok(())
}

/// Parse amount string (e.g., "1.0" or "0.5") into wei
fn parse_amount(amount: &str) -> Result<U256> {
    let parts: Vec<&str> = amount.split('.').collect();
    match parts.len() {
        1 => {
            // Whole number
            let whole = parts[0].parse::<u64>()
                .map_err(|_| anyhow!("Invalid amount: {}", amount))?;
            Ok(U256::from(whole) * U256::exp10(18))
        },
        2 => {
            // With decimal part
            let whole = parts[0].parse::<u64>()
                .map_err(|_| anyhow!("Invalid amount: {}", amount))?;
            let decimals = parts[1];
            let decimals = if decimals.len() > 18 { &decimals[..18] } else { decimals };
            
            let decimal_part = decimals.parse::<u64>()
                .map_err(|_| anyhow!("Invalid decimal part: {}", decimals))?;
            let decimal_places = decimals.len() as u32;
            
            let value = U256::from(whole) * U256::exp10(18) +
                       U256::from(decimal_part) * U256::exp10(18 - decimal_places as usize);
            
            Ok(value)
        },
        _ => Err(anyhow!("Invalid amount format: {}", amount)),
    }
}

/// Format wei amount to ETH with 6 decimal places
fn format_eth(wei: U256) -> String {
    let wei_str = wei.to_string();
    let len = wei_str.len();
    
    if len <= 18 {
        format!("0.{:0>18}", wei_str)
    } else {
        let (whole, decimal) = wei_str.split_at(len - 18);
        let decimal = &decimal[..6.min(decimal.len())]; // Show up to 6 decimal places
        format!("{}.{}", whole, decimal)
    }
}

/// Format wei to Gwei
fn format_gwei(wei: U256) -> String {
    let gwei = wei / U256::from(1_000_000_000u64);
    format!("{} Gwei", gwei)
}
