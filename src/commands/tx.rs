use anyhow::Context;
use clap::Parser;
use console::style;
use serde_json::Value;

use crate::{api::ApiProvider, config::ConfigManager, types::network::Network};

/// Command to check transaction status
#[derive(Debug, Parser)]
pub struct TxCommand {
    /// Transaction hash to check
    #[arg(short, long)]
    pub tx_hash: String,

    /// Use testnet
    #[arg(long)]
    pub testnet: bool,

    /// Alchemy API key (optional, will use saved key if not provided)
    #[arg(long)]
    pub api_key: Option<String>,
}

impl TxCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let network = if self.testnet {
            Network::RootStockTestnet
        } else {
            Network::RootStockMainnet
        };

        // Load config
        let config = ConfigManager::new()?.load()?;

        // Get API key and determine endpoint
        let (api_key, url) = if let Some(key) = &self.api_key {
            // Use provided API key with Alchemy
            let alchemy_url = if self.testnet {
                "https://rootstock-testnet.g.alchemy.com/v2"
            } else {
                "https://rootstock-mainnet.g.alchemy.com/v2"
            };
            (key.clone(), alchemy_url.to_string())
        } else if let Some(rsk_key) = config.get_api_key(&ApiProvider::RskRpc) {
            // Use RSK RPC endpoint
            let rsk_url = if self.testnet {
                "https://public-node.testnet.rsk.co"
            } else {
                "https://public-node.rsk.co"
            };
            (rsk_key.to_string(), rsk_url.to_string())
        } else if let Some(alchemy_key) = config.get_api_key(&ApiProvider::Alchemy) {
            // Fall back to Alchemy
            let alchemy_url = if self.testnet {
                "https://rootstock-testnet.g.alchemy.com/v2"
            } else {
                "https://rootstock-mainnet.g.alchemy.com/v2"
            };
            (alchemy_key.to_string(), alchemy_url.to_string())
        } else {
            anyhow::bail!(
                "No API key found for {}. Please set up RSK RPC or Alchemy API key using 'wallet config'.",
                network
            );
        };

        // Get receipt first as it contains the status
        let receipt = self
            .get_transaction_receipt(&client, &url, &api_key, &self.tx_hash)
            .await?;

        // Get transaction details for additional info
        let tx_details = self
            .get_transaction_details(&client, &url, &api_key, &self.tx_hash)
            .await?;

        // Display the information
        self.display_transaction_info(&tx_details, &receipt)?;

        Ok(())
    }

    async fn get_transaction_receipt(
        &self,
        client: &reqwest::Client,
        url: &str,
        api_key: &str,
        tx_hash: &str,
    ) -> anyhow::Result<Value> {
        let params = serde_json::json!([tx_hash]);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionReceipt",
            "params": params
        });

        let mut request_builder = client.post(url).json(&request);
        
        // Add authorization header only for Alchemy endpoints
        if url.contains("alchemy.com") {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request_builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?
            .json::<Value>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("Alchemy API error: {}", error);
        }

        response["result"]
            .as_object()
            .cloned()
            .map(Value::Object)
            .context("Invalid transaction receipt response")
    }

    async fn get_transaction_details(
        &self,
        client: &reqwest::Client,
        url: &str,
        api_key: &str,
        tx_hash: &str,
    ) -> anyhow::Result<Value> {
        let params = serde_json::json!([tx_hash]);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionByHash",
            "params": params
        });

        let mut request_builder = client.post(url).json(&request);
        
        // Add authorization header only for Alchemy endpoints
        if url.contains("alchemy.com") {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
        }
        
        let response = request_builder
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?
            .json::<Value>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if let Some(error) = response.get("error") {
            anyhow::bail!("Alchemy API error: {}", error);
        }

        response["result"]
            .as_object()
            .cloned()
            .map(Value::Object)
            .context("Invalid transaction details response")
    }

    fn display_transaction_info(&self, tx_details: &Value, receipt: &Value) -> anyhow::Result<()> {
        // Extract values with defaults
        let block_number = receipt["blockNumber"]
            .as_str()
            .and_then(|hex| {
                u64::from_str_radix(hex.trim_start_matches("0x"), 16)
                    .ok()
                    .map(|num| num.to_string())
            })
            .unwrap_or_else(|| "pending".to_string());

        let from = tx_details["from"].as_str().unwrap_or("unknown").to_string();

        let to = tx_details["to"]
            .as_str()
            .unwrap_or("contract creation")
            .to_string();

        let _value = tx_details["value"]
            .as_str()
            .and_then(|v| {
                // Parse hex string to U256
                let value_wei =
                    alloy::primitives::U256::from_str_radix(v.trim_start_matches("0x"), 16).ok()?;
                // Convert wei to RBTC (1e18 wei = 1 RBTC)
                let value_rbtc = value_wei.to::<u128>() as f64 / 1e18;
                Some(format!("{:.8} RBTC", value_rbtc))
            })
            .unwrap_or_else(|| "0 RBTC".to_string());

        let _gas_price = tx_details["gasPrice"]
            .as_str()
            .and_then(|v| {
                // Parse hex string to U256
                let price_wei =
                    alloy::primitives::U256::from_str_radix(v.trim_start_matches("0x"), 16).ok()?;
                // Convert wei to gwei (1e9 wei = 1 gwei)
                let price_gwei = price_wei.to::<u128>() as f64 / 1e9;
                Some(format!("{:.2} Gwei", price_gwei))
            })
            .unwrap_or_else(|| "N/A".to_string());

        let _gas_used = receipt["gasUsed"]
            .as_str()
            .and_then(|v| {
                // Parse hex string to U256
                alloy::primitives::U256::from_str_radix(v.trim_start_matches("0x"), 16)
                    .ok()
                    .map(|v| v.to_string())
            })
            .unwrap_or_else(|| "N/A".to_string());

        let status = match receipt["status"].as_str() {
            Some("0x1") | Some("0x01") => format!("{}", style("✓ Success").green().bold()),
            Some("0x0") | Some("0x00") => format!("{}", style("✗ Failed").red().bold()),
            _ => "⏳ Pending".to_string(),
        };

        // Display the information
        println!("\n{}\n", style("Transaction Details").bold().underlined());
        println!("{}", "-".repeat(60));

        println!("{}", style(format!("  Hash: {}", self.tx_hash)).dim());
        
        // Show timestamp if available
        if let Some(timestamp) = tx_details.get("timestamp").and_then(|t| t.as_str()) {
            if let Ok(timestamp_num) = u64::from_str_radix(timestamp.trim_start_matches("0x"), 16) {
                let datetime = chrono::DateTime::from_timestamp(timestamp_num as i64, 0)
                    .unwrap_or_else(|| chrono::Utc::now());
                println!("{}", style(format!("  Timestamp: {} UTC", datetime.format("%Y-%m-%d %H:%M:%S"))).dim());
            }
        }
        
        println!("{}", style(format!("  Block: {}", block_number)).dim());
        println!("{}", style(format!("  From: {}", from)).dim());
        println!("{}", style(format!("  To: {}", to)).dim());
        
        // Show value in RBTC
        if let Some(value_hex) = tx_details.get("value").and_then(|v| v.as_str()) {
            if let Ok(value_wei) = u128::from_str_radix(value_hex.trim_start_matches("0x"), 16) {
                let value_rbtc = value_wei as f64 / 1e18;
                if value_rbtc > 0.0 {
                    println!("{}", style(format!("  Value: {} RBTC", value_rbtc)).dim());
                }
            }
        }
        
        // Show gas information
        if let Some(gas_used_hex) = receipt.get("gasUsed").and_then(|g| g.as_str()) {
            if let Ok(gas_used) = u64::from_str_radix(gas_used_hex.trim_start_matches("0x"), 16) {
                println!("{}", style(format!("  Gas Used: {}", gas_used)).dim());
            }
        }
        
        if let Some(gas_price_hex) = tx_details.get("gasPrice").and_then(|g| g.as_str()) {
            if let Ok(gas_price_wei) = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16) {
                let gas_price_gwei = gas_price_wei as f64 / 1e9;
                println!("{}", style(format!("  Gas Price: {} Gwei", gas_price_gwei)).dim());
            }
        }
        
        // Calculate transaction fee
        if let (Some(gas_used_hex), Some(gas_price_hex)) = (
            receipt.get("gasUsed").and_then(|g| g.as_str()),
            tx_details.get("gasPrice").and_then(|g| g.as_str())
        ) {
            if let (Ok(gas_used), Ok(gas_price)) = (
                u128::from_str_radix(gas_used_hex.trim_start_matches("0x"), 16),
                u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
            ) {
                let fee_wei = gas_used * gas_price;
                let fee_rbtc = fee_wei as f64 / 1e18;
                println!("{}", style(format!("  Transaction Fee: {} RBTC", fee_rbtc)).dim());
            }
        }
        
        // Show nonce
        if let Some(nonce_hex) = tx_details.get("nonce").and_then(|n| n.as_str()) {
            if let Ok(nonce) = u64::from_str_radix(nonce_hex.trim_start_matches("0x"), 16) {
                println!("{}", style(format!("  Nonce: {}", nonce)).dim());
            }
        }

        println!("\n{}", style("Status").bold().underlined());
        println!("{}", "-".repeat(60));
        println!("\n{}", style(format!("  Status: {}", status)).dim());

        // If there's a contract address, show it
        if let Some(contract_addr) = receipt["contractAddress"].as_str() {
            if !contract_addr.is_empty() {
                println!("\n{}", style("Contract Creation").bold().underlined());
                println!("{}", "-".repeat(60));
                println!("{}", style(format!("  Contract: {}", contract_addr)).dim());
            }
        }

        // Show logs if any
        if let Some(logs) = receipt["logs"].as_array() {
            if !logs.is_empty() {
                println!(
                    "\n{}",
                    style(format!("  Logs ({}):", logs.len()))
                        .bold()
                        .underlined()
                );
                for log in logs {
                    if let Some(topic) = log["topics"].as_array().and_then(|t| t[0].as_str()) {
                        println!("  - {}", topic);
                    }
                }
            }
        }

        // Add explorer URL
        let explorer_url = if self.testnet {
            format!(
                "https://explorer.testnet.rsk.co/tx/{}",
                self.tx_hash.trim_start_matches("0x")
            )
        } else {
            format!(
                "https://explorer.rsk.co/tx/{}",
                self.tx_hash.trim_start_matches("0x")
            )
        };

        println!(
            "\n{} {}",
            style("ℹ️  Tip:").blue().bold(),
            style("Use a block explorer for more detailed information").dim()
        );

        println!(
            "\n🔗 View on Explorer: {}",
            style(explorer_url).blue().underlined()
        );

        Ok(())
    }
}
