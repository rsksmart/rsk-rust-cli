use crate::utils::alchemy::AlchemyClient;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use alloy::primitives::{Address, Bytes, B256, U64, U256};
use alloy::providers::{Provider, ProviderBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RskTransaction {
    // Core transaction fields
    pub hash: B256,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_price: U256,
    pub gas: U256,
    pub nonce: U256,
    pub input: Option<Bytes>,
    pub block_number: Option<U64>,
    pub transaction_index: Option<U64>,

    // Additional fields
    pub timestamp: SystemTime,
    pub status: TransactionStatus,
    pub token_address: Option<Address>,

    // Additional metadata
    pub confirms: Option<U64>,
    pub cumulative_gas_used: Option<U256>,
    pub logs: Option<Vec<alloy::rpc::types::Log>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Success,
    Failed,
    Unknown,
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub transaction_hash: B256,
    pub status: TransactionStatus,
    pub gas_used: U256,
    pub block_number: Option<U256>,
    pub block_hash: Option<B256>,
    pub cumulative_gas_used: U256,
}

impl RskTransaction {
    /// Converts the transaction to a CSV record
    pub fn to_csv_record(&self) -> csv::StringRecord {
        let timestamp = self
            .timestamp
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let datetime: DateTime<Utc> =
            DateTime::from_timestamp(timestamp as i64, 0).unwrap_or_default();
        let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        let to_address = self.to.map(|a| format!("0x{:x}", a)).unwrap_or_default();
        let token_address = self
            .token_address
            .map(|a| format!("0x{:x}", a))
            .unwrap_or_default();

        let status = match self.status {
            TransactionStatus::Success => "Success",
            TransactionStatus::Failed => "Failed",
            TransactionStatus::Pending => "Pending",
            TransactionStatus::Unknown => "Unknown",
        };

        let mut record = csv::StringRecord::new();
        record.push_field(&format!("0x{:x}", self.hash));
        record.push_field(&formatted_time);
        record.push_field(&format!("0x{:x}", self.from));
        record.push_field(&to_address);
        record.push_field(&self.value.to_string());
        record.push_field(&token_address);
        record.push_field(&self.gas_price.to_string());
        record.push_field(&self.gas.to_string());
        record.push_field(status);
        record.push_field(&self.block_number.map(|n| n.to_string()).unwrap_or_default());

        record
    }

    pub async fn from_alchemy_transfer(
        transfer: &Value,
        _wallet_address: &Address,
        alchemy_client: &AlchemyClient,
    ) -> Result<Self> {
        // Parse hash
        let hash = transfer["hash"]
            .as_str()
            .and_then(|s| B256::from_str(s).ok())
            .ok_or_else(|| {
                anyhow!(
                    "Invalid or missing transaction hash in transfer: {:?}",
                    transfer["hash"]
                )
            })?;

        // Parse addresses
        let from = transfer["from"]
            .as_str()
            .and_then(|s| Address::from_str(s).ok())
            .ok_or_else(|| anyhow!("Invalid 'from' address in transfer"))?;

        let to = transfer["to"]
            .as_str()
            .and_then(|s| Address::from_str(s).ok());

        // Handle value (can be number or hex string)
        let value = if let Some(num) = transfer["value"].as_u64() {
            U256::from(num)
        } else if let Some(hex_str) = transfer["value"].as_str() {
            U256::from_str_radix(hex_str.trim_start_matches("0x"), 16)?
        } else {
            U256::ZERO
        };

        // Get transaction receipt for status and gas used
        let rpc_url = alchemy_client.get_base_url();
        let receipt = Self::get_transaction_receipt(&hash, &rpc_url).await?;
        let (status, gas_used) = match receipt {
            Some(r) => (r.status, r.gas_used),
            None => (TransactionStatus::Pending, U256::ZERO),
        };

        // Get block number and timestamp
        let (block_number, timestamp) = if let Some(block_num) = transfer["blockNum"]
            .as_str()
            .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        {
            if let Some(block) = alchemy_client
                .get_block_by_number(block_num.to::<u64>())
                .await?
            {
                let timestamp = block
                    .get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                    .map(|t| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(t));
                (Some(block_num), timestamp.unwrap_or_else(SystemTime::now))
            } else {
                (Some(block_num), SystemTime::now())
            }
        } else {
            (None, SystemTime::now())
        };

        // Try to get timestamp from metadata first
        let timestamp = if let Some(timestamp_str) = transfer["metadata"]["blockTimestamp"].as_str()
        {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64)
            } else {
                timestamp
            }
        } else {
            timestamp
        };

        // Determine token address for ERC20 transfers
        let token_address = if transfer["category"].as_str() == Some("erc20") {
            transfer["rawContract"]["address"]
                .as_str()
                .and_then(|s| Address::from_str(s).ok())
        } else {
            None
        };

        // Get gas price if available
        let gas_price = transfer["gasPrice"]
            .as_str()
            .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .or_else(|| {
                transfer["effectiveGasPrice"]
                    .as_str()
                    .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            });

        // Get nonce if available
        let nonce = transfer["nonce"]
            .as_str()
            .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or_default();

        Ok(Self {
            hash,
            from,
            to,
            value,
            gas_price: gas_price.unwrap_or_default(),
            gas: gas_used,
            nonce,
            input: None, // Could be populated from raw transaction if needed
            block_number: block_number.map(|n| U64::from(n.to::<u64>())),
            transaction_index: None, // Could be populated from raw transaction
            timestamp,
            status,
            token_address,
            confirms: None, // Would need to be calculated from current block
            cumulative_gas_used: Some(gas_used), // From receipt if available
            logs: None,     // Could be populated from receipt if needed
        })
    }

    async fn get_transaction_receipt(
        hash: &B256,
        rpc_url: &str,
    ) -> Result<Option<TransactionReceipt>> {
        let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);
        let receipt = provider.get_transaction_receipt(*hash).await?;

        Ok(receipt.map(|r| TransactionReceipt {
            transaction_hash: r.transaction_hash,
            status: if r.status() {
                TransactionStatus::Success
            } else {
                TransactionStatus::Failed
            },
            gas_used: U256::from(r.gas_used),
            block_number: r.block_number.map(U256::from),
            block_hash: r.block_hash,
            cumulative_gas_used: U256::from(r.inner.cumulative_gas_used()),
        }))
    }
}
