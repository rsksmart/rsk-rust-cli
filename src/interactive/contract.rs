use crate::{
    config::ConfigManager,
    types::network::Network,
    wallet::load_wallet,
};
use anyhow::{anyhow, Result};
use dialoguer::{Confirm, Input, Select};
use ethers::{
    abi::Abi,
    prelude::*,
    providers::{Http, Provider, Middleware},
    signers::LocalWallet,
    types::{Address, U256},
};
use std::sync::Arc;
use std::str::FromStr;

/// Interactive menu for interacting with smart contracts
pub async fn interact_with_contract() -> Result<()> {
    println!("\n📝 Smart Contract Interaction");
    println!("========================");

    // Load wallet
    let wallet_data = match load_wallet()? {
        Some(w) => w,
        None => return Err(anyhow!("No wallet found. Please create a wallet first.")),
    };
    
    // Load config
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;
    
    // Get the network configuration
    let network_config = config.default_network.get_config();
    
    // Get the chain ID based on the network
    let chain_id = match config.default_network {
        Network::RootStockMainnet => 30,
        Network::RootStockTestnet => 31,
        Network::Mainnet => 1,
        Network::Testnet => 5, // Goerli
        Network::Regtest => 1337,
        _ => return Err(anyhow!("Unsupported network for contract interaction")),
    };
    
    // Create a wallet with the chain ID
    let private_key = wallet_data.private_key
        .as_ref()
        .ok_or_else(|| anyhow!("No private key found in wallet"))?;
    
    let wallet = private_key
        .parse::<LocalWallet>()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?
        .with_chain_id(chain_id);
    
    // Create provider
    let provider = Provider::<Http>::try_from(network_config.rpc_url.as_str())
        .map_err(|e| anyhow!("Failed to connect to RPC: {}", e))?;
    
    // Get contract address
    let contract_address: String = Input::new()
        .with_prompt("Enter contract address (0x...)")
        .validate_with(|input: &String| {
            if input.starts_with("0x") && input.len() == 42 {
                Ok(())
            } else {
                Err("Please enter a valid contract address starting with 0x".to_string())
            }
        })
        .interact()?;
    
    let contract_address = contract_address.parse::<Address>()
        .map_err(|e| anyhow!("Invalid contract address: {}", e))?;
    
    // Get ABI file path
    let abi_path: String = Input::new()
        .with_prompt("Enter path to ABI JSON file")
        .interact()?;
    
    // Read and parse ABI
    let abi_content = std::fs::read_to_string(&abi_path)
        .map_err(|e| anyhow!("Failed to read ABI file: {}", e))?;
    
    let abi: Abi = serde_json::from_str(&abi_content)
        .map_err(|e| anyhow!("Failed to parse ABI: {}", e))?;
    
    println!("\n📋 Available functions:");
    for (i, function) in abi.functions().enumerate() {
        println!("{:2}. {}", i + 1, function.signature());
    }
    
    // Select function
    let function_index: usize = Input::new()
        .with_prompt("Select function to call")
        .default(0)
        .interact()?;
    
    let selected_function = abi.functions().nth(function_index)
        .ok_or_else(|| anyhow!("Invalid function index"))?;
    
    println!("\n🔧 Function: {}", selected_function.signature());
    
    // TODO: Add parameter input and function call logic
    
    Ok(())
}

// Helper function to load wallet
fn load_wallet() -> Result<LocalWallet> {
    // TODO: Implement wallet loading logic
    // This is a placeholder - replace with actual wallet loading logic
    let private_key = "0x...".to_string();
    
    private_key.parse::<LocalWallet>()
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))
}

// Helper function to load config
fn load_config() -> Result<Config> {
    // TODO: Implement config loading logic
    // This is a placeholder - replace with actual config loading logic
    Ok(Config::default())
}

#[derive(Default)]
struct Config {
    default_network: Network,
}

#[derive(Default)]
enum Network {
    #[default]
    Mainnet,
    Testnet,
}

impl Network {
    fn get_config(&self) -> NetworkConfig {
        match self {
            Network::Mainnet => NetworkConfig {
                rpc_url: "https://public-node.rsk.co".to_string(),
            },
            Network::Testnet => NetworkConfig {
                rpc_url: "https://public-node.testnet.rsk.co".to_string(),
            },
        }
    }
}

struct NetworkConfig {
    rpc_url: String,
}
