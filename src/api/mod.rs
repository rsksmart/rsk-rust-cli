use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiProvider {
    /// Alchemy API - Used for transaction history and advanced queries
    Alchemy,
    /// RSK RPC API - Primary RPC for blockchain operations (balances, transactions, etc.)
    RskRpc,
    /// Custom API provider
    Custom(String),
}

impl fmt::Display for ApiProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiProvider::Alchemy => write!(f, "Alchemy"),
            ApiProvider::RskRpc => write!(f, "RSK RPC"),
            ApiProvider::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: crate::utils::secrets::SecretString,
    pub network: String, // "mainnet", "testnet", etc.
    pub provider: ApiProvider,
    pub name: Option<String>,
}

impl zeroize::Zeroize for ApiKey {
    fn zeroize(&mut self) {
        self.key.expose_mut().zeroize();
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ApiManager {
    keys: HashMap<String, ApiKey>, // keyed by a unique identifier
}

impl ApiManager {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    pub fn add_key(&mut self, key: ApiKey) -> String {
        let id = format!("{:?}-{}", key.provider, key.network).to_lowercase();
        self.keys.insert(id.clone(), key);
        id
    }

    pub fn get_key(&self, provider: &ApiProvider, network: &str) -> Option<&ApiKey> {
        let id = format!("{:?}-{}", provider, network).to_lowercase();
        self.keys.get(&id)
    }

    pub fn remove_key(&mut self, provider: &ApiProvider, network: &str) -> Option<ApiKey> {
        let id = format!("{:?}-{}", provider, network).to_lowercase();
        self.keys.remove(&id)
    }

    pub fn list_keys(&self) -> Vec<&ApiKey> {
        self.keys.values().collect()
    }
}

// Integration with the existing config system
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ApiConfig {
    pub default_provider: Option<ApiProvider>,
    pub keys: Vec<ApiKey>,
}
