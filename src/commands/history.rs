use crate::types::transaction::{RskTransaction, TransactionStatus};
use crate::types::wallet::WalletData;
use crate::utils::alchemy::AlchemyClient;
use crate::utils::api_validator::validate_api_key_format;
use crate::utils::{constants, table::TableBuilder};
use crate::api::ApiProvider;
use anyhow::Result;
use chrono::TimeZone;
use clap::Parser;
use colored::Colorize;
use console::style;
use alloy::primitives::Address;
use std::fs;
use std::str::FromStr;

/// Show the transaction history for an address or the current wallet
#[derive(Parser, Clone)]
pub struct HistoryCommand {
    /// Address to check transaction history for
    #[arg(short, long)]
    pub address: Option<String>,

    /// Contact name to check transaction history for
    #[arg(short, long)]
    pub contact: Option<String>,

    /// Number of transactions to show
    #[arg(short, long, default_value = "10")]
    pub limit: u32,

    /// Show detailed transaction information
    #[arg(short, long)]
    pub detailed: bool,

    /// Filter by transaction status (pending/success/failed)
    #[arg(short, long)]
    pub status: Option<String>,

    /// Filter by token address
    #[arg(short, long)]
    pub token: Option<String>,

    /// Start date for filtering (YYYY-MM-DD)
    #[arg(short, long)]
    pub from: Option<String>,

    /// End date for filtering (YYYY-MM-DD)
    #[arg(short, long)]
    pub to: Option<String>,

    /// Sort by field (timestamp, value, gas)
    #[arg(short, long, default_value = "timestamp")]
    pub sort_by: String,

    /// Sort order (asc/desc)
    #[arg(long, default_value = "desc")]
    pub sort_order: String,

    /// Export transactions to CSV file
    #[arg(long)]
    pub export_csv: Option<String>,

    /// Show only incoming transactions
    #[arg(short, long)]
    pub incoming: bool,

    /// Show only outgoing transactions
    #[arg(short, long)]
    pub outgoing: bool,

    /// Alchemy API key (if not already saved)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Network to query (mainnet | testnet). Defaults to mainnet.
    #[arg(long, default_value = "mainnet")]
    pub network: String,
}

// Custom Debug implementation that redacts the API key
impl std::fmt::Debug for HistoryCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryCommand")
            .field("address", &self.address)
            .field("contact", &self.contact)
            .field("limit", &self.limit)
            .field("detailed", &self.detailed)
            .field("status", &self.status)
            .field("token", &self.token)
            .field("from", &self.from)
            .field("to", &self.to)
            .field("sort_by", &self.sort_by)
            .field("sort_order", &self.sort_order)
            .field("export_csv", &self.export_csv)
            .field("incoming", &self.incoming)
            .field("outgoing", &self.outgoing)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("network", &self.network)
            .finish()
    }
}

impl HistoryCommand {
    pub async fn execute(&self) -> Result<()> {
        // 1. Load config and resolve API key
        // let config = Config::load()?;
        let wallet_file = constants::wallet_file_path();
        let mut stored_api_key: Option<String> = None;

        // If export is requested, ensure we have a filename
        if let Some(filename) = &self.export_csv {
            if !filename.ends_with(".csv") {
                return Err(anyhow::anyhow!("Export filename must end with .csv"));
            }
        }

        // Try to load API key from wallet file
        if wallet_file.exists() {
            let data = fs::read_to_string(&wallet_file)?;
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(api_key) = val["alchemyApiKey"].as_str() {
                    stored_api_key = Some(api_key.to_string());
                }

                // Persist CLI key if supplied and not yet saved
                if stored_api_key.is_none() && self.api_key.is_some() {
                    let api_key = self.api_key.clone().unwrap();
                    
                    // Validate API key format before saving
                    if let Err(e) = validate_api_key_format(&ApiProvider::Alchemy, &api_key) {
                        return Err(anyhow::anyhow!("Invalid API key format: {}", e));
                    }
                    
                    val["alchemyApiKey"] = serde_json::Value::String(api_key.clone());
                    crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&val)?)?;
                    stored_api_key = Some(api_key);
                    println!("{}", "Saved Alchemy API key ✅".green());
                }
            }
        }

        let final_api_key = self
            .api_key
            .clone()
            .or(stored_api_key)
            .or(std::env::var("ALCHEMY_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("Alchemy API key missing – supply --api-key once"))?;

        let is_testnet = self.network.to_lowercase() == "testnet";
        if self.network.to_lowercase() != "mainnet" && !is_testnet {
            anyhow::bail!("Invalid network: use 'mainnet' or 'testnet'");
        }

        // 2. Get address to query
        let address = if let Some(addr) = &self.address {
            Address::from_str(addr).map_err(|_| {
                anyhow::anyhow!("Invalid address format. Expected 0x-prefixed hex string")
            })?
        }
        //  else if let Some(contact_name) = &self.contact {
        //     // Handle contact name resolution
        //     let contacts = Contact::load_all()?;
        //     let contact = contacts.iter().find(|c| &c.name == contact_name)
        //         .ok_or_else(|| anyhow::anyhow!("Contact '{}' not found", contact_name))?;
        //     contact.address
        // }
        else {
            // Get current wallet address
            if !wallet_file.exists() {
                anyhow::bail!("No wallets found. Create or import a wallet first.");
            }
            let data = fs::read_to_string(&wallet_file)?;
            let wallet_data = serde_json::from_str::<WalletData>(&data)?;
            wallet_data
                .get_current_wallet()
                .ok_or_else(|| {
                    anyhow::anyhow!("No default wallet selected. Use `wallet switch` first.")
                })?
                .address
        };

        // 3. Initialize Alchemy client and fetch transfers
        let alchemy_client = AlchemyClient::new(final_api_key, is_testnet);
        let response = alchemy_client
            .get_asset_transfers(
                &format!("{:#x}", address),
                self.limit,
                self.from.as_deref(),
                self.to.as_deref(),
            )
            .await?;

        // 4. Process transactions
        let transfers = response["result"]["transfers"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format from Alchemy"))?;

        let mut txs = Vec::new();
        for transfer in transfers {
            // Convert Alchemy transfer to RskTransaction
            let tx =
                RskTransaction::from_alchemy_transfer(transfer, &address, &alchemy_client).await?;
            txs.push(tx);
        }

        // 5. Apply filters
        if self.incoming && self.outgoing {
            anyhow::bail!("Cannot use both --incoming and --outgoing at the same time");
        }
        if self.incoming {
            txs.retain(|tx| tx.to == Some(address));
        } else if self.outgoing {
            txs.retain(|tx| tx.from == address);
        }

        // 6. Handle empty result
        if txs.is_empty() {
            println!("{}", "⚠️  No transactions found.".yellow());
            return Ok(());
        }

        // 7. Sort results
        match (self.sort_by.as_str(), self.sort_order.as_str()) {
            ("timestamp", "asc") => txs.sort_by_key(|t| t.timestamp),
            ("timestamp", _) => txs.sort_by_key(|t| std::cmp::Reverse(t.timestamp)),
            ("value", "asc") => txs.sort_by_key(|t| t.value),
            ("value", _) => txs.sort_by_key(|t| std::cmp::Reverse(t.value)),
            _ => {}
        }

        // 8. Export to CSV if requested
        if let Some(filename) = &self.export_csv {
            let mut wtr = csv::Writer::from_path(filename)?;

            // Write header
            wtr.write_record([
                "Transaction Hash",
                "Timestamp",
                "From",
                "To",
                "Value (wei)",
                "Token Address",
                "Gas Price (wei)",
                "Gas Used",
                "Status",
                "Block Number",
            ])?;

            // Write transactions
            for tx in &txs {
                let record = tx.to_csv_record();
                wtr.write_record(&record)?;
            }

            wtr.flush()?;
            println!(
                "\n{} Exported {} transactions to {}",
                style("✓").green().bold(),
                txs.len(),
                style(filename).cyan()
            );
            return Ok(());
        }

        // 9. Display results in terminal
        let mut table = TableBuilder::new();
        if self.detailed {
            table.add_header(&[
                "TX Hash",
                "From",
                "To",
                "Status",
                "Timestamp",
                "Block",
                "Gas Used",
                "Gas Price",
                "Nonce",
            ]);

            for tx in &txs {
                let status_disp = match tx.status {
                    TransactionStatus::Success => "Success".green(),
                    TransactionStatus::Failed => "Failed".red(),
                    TransactionStatus::Pending => "Pending".yellow(),
                    TransactionStatus::Unknown => "Unknown".yellow(),
                };

                let ts = chrono::Local
                    .timestamp_opt(
                        tx.timestamp
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs() as i64,
                        0,
                    )
                    .unwrap();

                table.add_row(&[
                    &format!("0x{}", &tx.hash.to_string()[2..]),
                    &format!("0x{}", &tx.from.to_string()[2..]),
                    &tx.to
                        .as_ref()
                        .map(|a| format!("0x{}", &a.to_string()[2..]))
                        .unwrap_or_else(|| "-".into()),
                    &status_disp.to_string(),
                    &ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                    // &tx.block_number.to_string(),
                ]);
            }
        } else {
            table.add_header(&["TX Hash", "From", "To", "Status"]);

            for tx in &txs {
                let status_disp = match tx.status {
                    TransactionStatus::Success => "Success".green(),
                    TransactionStatus::Failed => "Failed".red(),
                    TransactionStatus::Pending => "Pending".yellow(),
                    TransactionStatus::Unknown => "Unknown".yellow(),
                };

                table.add_row(&[
                    &format!("0x{}", &tx.hash.to_string()[2..10]),
                    &format!("0x{}", &tx.from.to_string()[2..6]),
                    &tx.to
                        .as_ref()
                        .map(|a| format!("0x{}", &a.to_string()[2..6]))
                        .unwrap_or_else(|| "-".into()),
                    &status_disp.to_string(),
                ]);
            }
        }

        table.print();
        Ok(())
    }
}
