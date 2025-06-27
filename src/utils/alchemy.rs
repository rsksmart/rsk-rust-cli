// src/utils/alchemy.rs
use anyhow::{Result, anyhow};
use serde_json::Value;

pub struct AlchemyClient {
    client: reqwest::Client,
    api_key: String,
    is_testnet: bool,
}

impl AlchemyClient {
    pub fn new(api_key: String, is_testnet: bool) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            is_testnet,
        }
    }

    pub fn get_base_url(&self) -> String {
        let network = if self.is_testnet {
            "testnet"
        } else {
            "mainnet"
        };
        format!(
            "https://rootstock-{}.g.alchemy.com/v2/{}",
            network, self.api_key
        )
    }

    pub async fn get_asset_transfers(
        &self,
        address: &str,
        limit: u32,
        from_block: Option<&str>,
        to_block: Option<&str>,
    ) -> Result<Value> {
        let url = self.get_base_url();

        let params = serde_json::json!([{
            "fromBlock": from_block.unwrap_or("0x0"),
            "toBlock": to_block.unwrap_or("latest"),
            "fromAddress": address,
            "category": ["external", "erc20"],
            "withMetadata": true,
            "excludeZeroValue": false,
            "maxCount": format!("0x{:x}", limit),
        }]);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "alchemy_getAssetTransfers",
                "params": params
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("Alchemy API error: {}", error));
        }

        Ok(response)
    }

    pub async fn get_block_by_number(&self, block_number: u64) -> Result<Option<Value>> {
        let url = self.get_base_url();
        let block_number_hex = format!("0x{:x}", block_number);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "eth_getBlockByNumber",
                "params": [block_number_hex, false]  // false to get transaction hashes only
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(error) = response.get("error") {
            return Err(anyhow!("Alchemy API error: {}", error));
        }

        Ok(response
            .get("result")
            .and_then(|r| if r.is_null() { None } else { Some(r.clone()) }))
    }
}
