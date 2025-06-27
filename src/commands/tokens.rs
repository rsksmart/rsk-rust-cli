use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
pub struct TokenAddCommand {
    /// Token symbol (e.g., RIF, USD)
    #[arg(short, long)]
    pub symbol: String,

    /// Token contract address
    #[arg(short, long)]
    pub address: String,

    /// Number of decimal places for the token
    #[arg(short, long, default_value_t = 18)]
    pub decimals: u8,

    /// Network to add token to (mainnet/testnet)
    #[arg(short, long, default_value = "mainnet")]
    pub network: String,
}

#[derive(Parser, Debug)]
pub struct TokenRemoveCommand {
    /// Token symbol to remove
    #[arg(short, long)]
    pub symbol: String,

    /// Network to remove token from (mainnet/testnet)
    #[arg(short, long, default_value = "mainnet")]
    pub network: String,
}

#[derive(Parser, Debug)]
pub struct TokenListCommand {
    /// Network to list tokens for (mainnet/testnet)
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenInfo {
    pub address: String,
    pub decimals: u8,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TokenRegistry {
    pub mainnet: HashMap<String, TokenInfo>,
    pub testnet: HashMap<String, TokenInfo>,
}

impl TokenRegistry {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = "tokens.json";
        if !Path::new(path).exists() {
            // Create a new empty registry if file doesn't exist
            let registry = TokenRegistry {
                mainnet: HashMap::new(),
                testnet: HashMap::new(),
            };
            let json = serde_json::to_string_pretty(&json!(&registry))?;
            fs::write(path, json)?;
            return Ok(registry);
        }

        let content = fs::read_to_string(path)?;
        let registry: TokenRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write("tokens.json", json)?;
        Ok(())
    }

    pub fn add_token(
        &mut self,
        network: &str,
        symbol: &str,
        address: &str,
        decimals: u8,
    ) -> Result<(), String> {
        let network_lower = network.to_lowercase();
        let symbol_upper = symbol.to_uppercase();
        let address_lower = address.to_lowercase();

        // Check if symbol already exists in any network
        if self.mainnet.contains_key(&symbol_upper) || self.testnet.contains_key(&symbol_upper) {
            return Err(format!(
                "Token symbol '{}' already exists in the registry",
                symbol_upper
            ));
        }

        // Check if address already exists in any network
        let all_tokens = self.mainnet.iter().chain(self.testnet.iter());
        for (_, token) in all_tokens {
            if token.address.to_lowercase() == address_lower {
                return Err(format!("Token address '{}' is already registered", address));
            }
        }

        let token = TokenInfo {
            address: address.to_string(),
            decimals,
        };

        match network_lower.as_str() {
            "mainnet" => {
                self.mainnet.insert(symbol_upper, token);
            }
            "testnet" => {
                self.testnet.insert(symbol_upper, token);
            }
            _ => return Err("Invalid network. Use 'mainnet' or 'testnet'.".to_string()),
        }
        Ok(())
    }

    pub fn remove_token(&mut self, network: &str, symbol: &str) -> Result<(), &'static str> {
        match network.to_lowercase().as_str() {
            "mainnet" => {
                self.mainnet.remove(&symbol.to_uppercase());
            }
            "testnet" => {
                self.testnet.remove(&symbol.to_uppercase());
            }
            _ => return Err("Invalid network. Use 'mainnet' or 'testnet'."),
        }
        Ok(())
    }

    pub fn list_tokens(&self, network: Option<&str>) -> Vec<(String, TokenInfo)> {
        let mut result = Vec::new();

        match network {
            Some(net) => match net.to_lowercase().as_str() {
                "mainnet" => {
                    for (symbol, info) in &self.mainnet {
                        result.push((
                            symbol.clone(),
                            TokenInfo {
                                address: info.address.clone(),
                                decimals: info.decimals,
                            },
                        ));
                    }
                }
                "testnet" => {
                    for (symbol, info) in &self.testnet {
                        result.push((
                            symbol.clone(),
                            TokenInfo {
                                address: info.address.clone(),
                                decimals: info.decimals,
                            },
                        ));
                    }
                }
                _ => {}
            },
            None => {
                for (symbol, info) in &self.mainnet {
                    result.push((
                        format!("mainnet/{}", symbol),
                        TokenInfo {
                            address: info.address.clone(),
                            decimals: info.decimals,
                        },
                    ));
                }
                for (symbol, info) in &self.testnet {
                    result.push((
                        format!("testnet/{}", symbol),
                        TokenInfo {
                            address: info.address.clone(),
                            decimals: info.decimals,
                        },
                    ));
                }
            }
        }
        result
    }
}

pub fn add_token(
    network: &str,
    symbol: &str,
    address: &str,
    decimals: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TokenRegistry::load()?;
    if let Err(e) = registry.add_token(network, symbol, address, decimals) {
        return Err(e.into());
    }
    registry.save()?;
    println!("Added token {} to {} network", symbol, network);
    Ok(())
}

pub fn remove_token(network: &str, symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = TokenRegistry::load()?;
    registry.remove_token(network, symbol)?;
    registry.save()?;
    println!("Removed token {} from {} network", symbol, network);
    Ok(())
}

pub fn list_tokens(
    network: Option<&str>,
) -> Result<Vec<(String, TokenInfo)>, Box<dyn std::error::Error>> {
    let registry = TokenRegistry::load()?;
    let tokens = registry.list_tokens(network);

    if tokens.is_empty() {
        match network {
            Some(net) => println!("No tokens found in {} network", net),
            None => println!("No tokens found in registry"),
        }
    }

    Ok(tokens)
}
