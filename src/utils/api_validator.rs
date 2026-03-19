use crate::api::{ApiProvider, ApiKey};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
    NetworkError(String),
}

pub async fn validate_api_key(api_key: &ApiKey) -> Result<ValidationResult> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    match api_key.provider {
        ApiProvider::RskRpc => validate_rsk_key(&client, api_key).await,
        ApiProvider::Alchemy => validate_alchemy_rsk_key(&client, api_key).await,
        ApiProvider::Custom(_) => Ok(ValidationResult::Valid),
    }
}

async fn validate_rsk_key(client: &Client, api_key: &ApiKey) -> Result<ValidationResult> {
    let base_url = match api_key.network.as_str() {
        "mainnet" => "https://public-node.rsk.co",
        "testnet" => "https://public-node.testnet.rsk.co",
        _ => return Ok(ValidationResult::Invalid("Unsupported Rootstock network".to_string())),
    };

    let url = format!("{}?apikey={}", base_url, api_key.key);
    
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    match client.post(&url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => {
                        if json.get("error").is_some() {
                            let error_msg = json["error"]["message"].as_str()
                                .unwrap_or("Invalid API key");
                            Ok(ValidationResult::Invalid(error_msg.to_string()))
                        } else if json.get("result").is_some() {
                            Ok(ValidationResult::Valid)
                        } else {
                            Ok(ValidationResult::Invalid("Unexpected response".to_string()))
                        }
                    }
                    Err(_) => Ok(ValidationResult::Invalid("Invalid response format".to_string()))
                }
            } else if response.status() == 401 || response.status() == 403 {
                Ok(ValidationResult::Invalid("Invalid or expired API key".to_string()))
            } else {
                Ok(ValidationResult::Invalid(format!("HTTP {}", response.status())))
            }
        }
        Err(e) => Ok(ValidationResult::NetworkError(e.to_string()))
    }
}

async fn validate_alchemy_rsk_key(client: &Client, api_key: &ApiKey) -> Result<ValidationResult> {
    // Alchemy for Rootstock (if they support it)
    let network_suffix = match api_key.network.as_str() {
        "mainnet" => "rsk-mainnet",
        "testnet" => "rsk-testnet", 
        _ => return Ok(ValidationResult::Invalid("Unsupported network".to_string())),
    };

    let url = format!("https://{}.g.alchemy.com/v2/{}", network_suffix, api_key.key);
    
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    match client.post(&url).json(&payload).send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(json) => {
                        if json.get("error").is_some() {
                            Ok(ValidationResult::Invalid("Invalid API key".to_string()))
                        } else if json.get("result").is_some() {
                            Ok(ValidationResult::Valid)
                        } else {
                            Ok(ValidationResult::Invalid("Unexpected response".to_string()))
                        }
                    }
                    Err(_) => Ok(ValidationResult::Invalid("Invalid response format".to_string()))
                }
            } else if response.status() == 401 {
                Ok(ValidationResult::Invalid("Invalid API key".to_string()))
            } else {
                Ok(ValidationResult::Invalid(format!("HTTP {}", response.status())))
            }
        }
        Err(e) => Ok(ValidationResult::NetworkError(e.to_string()))
    }
}

pub fn validate_api_key_format(provider: &ApiProvider, key: &str) -> Result<()> {
    match provider {
        ApiProvider::RskRpc => {
            if key.is_empty() {
                return Err(anyhow!("RSK RPC API key cannot be empty"));
            }
        }
        ApiProvider::Alchemy => {
            if key.len() < 32 {
                return Err(anyhow!("Alchemy API key should be at least 32 characters"));
            }
            if !key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                return Err(anyhow!("Alchemy API key contains invalid characters"));
            }
        }
        ApiProvider::Custom(_) => {}
    }
    Ok(())
}
