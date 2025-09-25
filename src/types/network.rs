use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub name: String,
    pub rpc_url: String,
    pub explorer_url: String,
}

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
    AlchemyMainnet,
    AlchemyTestnet,
    RootStockMainnet,
    RootStockTestnet,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Mainnet => write!(f, "Mainnet"),
            Network::Testnet => write!(f, "Testnet"),
            Network::Regtest => write!(f, "Regtest"),
            Network::AlchemyMainnet => write!(f, "Alchemy Mainnet"),
            Network::AlchemyTestnet => write!(f, "Alchemy Testnet"),
            Network::RootStockMainnet => write!(f, "Rootstock Mainnet"),
            Network::RootStockTestnet => write!(f, "Rootstock Testnet"),
        }
    }
}

impl Network {
    pub fn get_config(&self) -> NetworkConfig {
        match self {
            Network::Mainnet => NetworkConfig {
                name: "RSK Mainnet".to_string(),
                rpc_url: "https://public-node.rsk.co".to_string(),
                explorer_url: "https://explorer.rsk.co".to_string(),
            },
            Network::Testnet => NetworkConfig {
                name: "RSK Testnet".to_string(),
                rpc_url: "https://public-node.testnet.rsk.co".to_string(),
                explorer_url: "https://explorer.testnet.rsk.co".to_string(),
            },
            Network::Regtest => NetworkConfig {
                name: "RSK Regtest".to_string(),
                rpc_url: "http://localhost:4444".to_string(),
                explorer_url: "".to_string(),
            },
            // Legacy network types - use public nodes by default
            Network::AlchemyMainnet => NetworkConfig {
                name: "RSK Mainnet".to_string(),
                rpc_url: "https://public-node.rsk.co".to_string(),
                explorer_url: "https://explorer.rsk.co".to_string(),
            },
            Network::AlchemyTestnet => NetworkConfig {
                name: "RSK Testnet".to_string(),
                rpc_url: "https://public-node.testnet.rsk.co".to_string(),
                explorer_url: "https://explorer.testnet.rsk.co".to_string(),
            },
            Network::RootStockMainnet => NetworkConfig {
                name: "RSK Mainnet".to_string(),
                rpc_url: "https://public-node.rsk.co".to_string(),
                explorer_url: "https://explorer.rsk.co".to_string(),
            },
            Network::RootStockTestnet => NetworkConfig {
                name: "RSK Testnet".to_string(),
                rpc_url: "https://public-node.testnet.rsk.co".to_string(),
                explorer_url: "https://explorer.testnet.rsk.co".to_string(),
            },
        }
    }

    /// Get RPC URL with API key preference: RSK RPC > Alchemy > Public nodes
    pub fn get_rpc_url_with_key(
        &self,
        rsk_api_key: Option<&str>,
        alchemy_api_key: Option<&str>,
    ) -> String {
        match self {
            Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => {
                // Prefer RSK RPC API if available
                if let Some(key) = rsk_api_key {
                    format!("https://rpc.rootstock.io/{}", key)
                }
                // Fall back to Alchemy if available
                else if let Some(key) = alchemy_api_key {
                    format!("https://rootstock-mainnet.g.alchemy.com/v2/{}", key)
                }
                // Default to public node
                else {
                    "https://public-node.rsk.co".to_string()
                }
            }
            Network::Testnet | Network::AlchemyTestnet | Network::RootStockTestnet => {
                // Prefer RSK RPC API if available
                if let Some(key) = rsk_api_key {
                    format!("https://rpc.testnet.rootstock.io/{}", key)
                }
                // Fall back to Alchemy if available
                else if let Some(key) = alchemy_api_key {
                    format!("https://rootstock-testnet.g.alchemy.com/v2/{}", key)
                }
                // Default to public node
                else {
                    "https://public-node.testnet.rsk.co".to_string()
                }
            }
            Network::Regtest => "http://localhost:4444".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mainnet" => Some(Network::Mainnet),
            "testnet" => Some(Network::Testnet),
            "regtest" => Some(Network::Regtest),
            "alchemy-mainnet" => Some(Network::AlchemyMainnet),
            "alchemy-testnet" => Some(Network::AlchemyTestnet),
            "rootstock-mainnet" => Some(Network::RootStockMainnet),
            "rootstock-testnet" => Some(Network::RootStockTestnet),
            _ => None,
        }
    }
}
