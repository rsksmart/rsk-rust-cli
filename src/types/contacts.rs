use crate::types::transaction::RskTransaction;
use anyhow::Result;
use colored::Colorize;
use alloy::primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactTransactionStats {
    pub total_transactions: u64,
    pub total_volume: U256,
    pub last_transaction: Option<chrono::DateTime<chrono::Local>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub address: Address,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Local>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_stats: Option<ContactTransactionStats>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_transactions: Vec<B256>, // Transaction hashes
}

impl Contact {
    pub fn new(name: String, address: Address, notes: Option<String>, tags: Vec<String>) -> Self {
        Self {
            name,
            address,
            notes,
            tags,
            created_at: chrono::Local::now(),
            transaction_stats: Some(ContactTransactionStats {
                total_transactions: 0,
                total_volume: U256::ZERO,
                last_transaction: None,
            }),
            recent_transactions: Vec::new(),
        }
    }

    pub fn update_transaction_stats(&mut self, tx: &RskTransaction, _is_incoming: bool) {
        // Ensure transaction_stats is initialized
        if self.transaction_stats.is_none() {
            self.transaction_stats = Some(ContactTransactionStats {
                total_transactions: 0,
                total_volume: U256::ZERO,
                last_transaction: None,
            });
        }

        if let Some(stats) = &mut self.transaction_stats {
            stats.total_transactions += 1;
            stats.total_volume = stats.total_volume.saturating_add(tx.value);
            stats.last_transaction = Some(chrono::Local::now());

            // Keep only the most recent 10 transactions
            if self.recent_transactions.len() >= 10 {
                self.recent_transactions.remove(0);
            }
            self.recent_transactions.push(tx.hash);
        }
    }

    pub fn get_transaction_history<'a>(
        &self,
        transactions: &'a [RskTransaction],
    ) -> Vec<&'a RskTransaction> {
        transactions
            .iter()
            .filter(|tx| {
                tx.from == self.address
                    || tx.to == Some(self.address)
                    || tx.to.as_ref().is_some_and(|to| *to == self.address)
            })
            .collect()
    }

    pub fn get_total_volume(&self) -> U256 {
        self.transaction_stats
            .as_ref()
            .map(|s| s.total_volume)
            .unwrap_or(U256::ZERO)
    }

    pub fn get_total_transactions(&self) -> u64 {
        self.transaction_stats
            .as_ref()
            .map(|s| s.total_transactions)
            .unwrap_or(0)
    }

    /// Get recent transactions for this contact
    pub fn get_recent_transactions<'a>(
        &'a self,
        all_transactions: &'a [RskTransaction],
        limit: Option<usize>,
    ) -> Vec<&'a RskTransaction> {
        // If we have recent_transactions hashes, use them for faster lookups
        if !self.recent_transactions.is_empty() {
            let mut txs: Vec<_> = self
                .recent_transactions
                .iter()
                .filter_map(|hash| all_transactions.iter().find(|tx| tx.hash == *hash))
                .collect();

            // Sort by block number and transaction index (newest first)
            txs.sort_by(|a, b| {
                let a_block = a.block_number.unwrap_or_default();
                let b_block = b.block_number.unwrap_or_default();
                let a_index = a.transaction_index.unwrap_or_default();
                let b_index = b.transaction_index.unwrap_or_default();

                b_block.cmp(&a_block).then(b_index.cmp(&a_index))
            });

            if let Some(limit) = limit {
                txs.truncate(limit);
            }

            return txs;
        }

        // Fallback to filtering all transactions if no recent_transactions available
        let mut txs: Vec<_> = all_transactions
            .iter()
            .filter(|tx| {
                tx.from == self.address
                    || tx.to == Some(self.address)
                    || tx.to.as_ref().is_some_and(|to| *to == self.address)
            })
            .collect();

        // Sort by block number and transaction index (newest first)
        txs.sort_by(|a, b| {
            let a_block = a.block_number.unwrap_or_default();
            let b_block = b.block_number.unwrap_or_default();
            let a_index = a.transaction_index.unwrap_or_default();
            let b_index = b.transaction_index.unwrap_or_default();

            b_block.cmp(&a_block).then(b_index.cmp(&a_index))
        });

        if let Some(limit) = limit {
            txs.truncate(limit);
        }

        txs
    }

    /// Get total volume between this contact and another address
    pub fn get_volume_between(
        &self,
        other_address: Address,
        transactions: &[RskTransaction],
    ) -> (U256, U256) {
        let mut sent = U256::ZERO;
        let mut received = U256::ZERO;

        for tx in transactions {
            if tx.from == self.address && tx.to == Some(other_address) {
                sent = sent.saturating_add(tx.value);
            } else if tx.from == other_address && tx.to == Some(self.address) {
                received = received.saturating_add(tx.value);
            }
        }

        (sent, received)
    }

    /// Check if this contact has any transaction history
    pub fn has_transaction_history(&self) -> bool {
        self.transaction_stats
            .as_ref()
            .map(|s| s.total_transactions > 0)
            .unwrap_or(false)
    }

    /// Get the last transaction timestamp if available
    pub fn last_transaction_time(&self) -> Option<&chrono::DateTime<chrono::Local>> {
        self.transaction_stats
            .as_ref()
            .and_then(|s| s.last_transaction.as_ref())
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Contact name cannot be empty"));
        }
        if self.address == Address::ZERO {
            return Err(anyhow::anyhow!("Contact address cannot be zero"));
        }
        if self.notes.as_ref().is_some_and(|n| n.is_empty()) {
            return Err(anyhow::anyhow!("Notes cannot be empty if provided"));
        }
        if self.tags.iter().any(|tag| tag.is_empty()) {
            return Err(anyhow::anyhow!("Tags cannot be empty"));
        }
        if self.tags.len() > 5 {
            return Err(anyhow::anyhow!("A contact can have a maximum of 5 tags"));
        }
        if self.created_at.timestamp() > chrono::Local::now().timestamp() {
            return Err(anyhow::anyhow!(
                "Created at timestamp cannot be in the future"
            ));
        }
        if self.created_at.timestamp() < 0 {
            return Err(anyhow::anyhow!("Created at timestamp cannot be negative"));
        }
        if let Some(stats) = &self.transaction_stats {
            if let Some(last_tx) = stats.last_transaction {
                if last_tx.timestamp() > chrono::Local::now().timestamp() {
                    return Err(anyhow::anyhow!(
                        "Last transaction timestamp cannot be in the future"
                    ));
                }
            }
        }

        if self.created_at.timestamp() < 1_000_000_000 {
            return Err(anyhow::anyhow!("Created at timestamp is too old"));
        }
        if self.created_at.timestamp() > chrono::Local::now().timestamp() + 60 * 60 * 24 * 365 {
            return Err(anyhow::anyhow!(
                "Created at timestamp is too far in the future"
            ));
        }
        if self.created_at.timestamp() < chrono::Local::now().timestamp() - 60 * 60 * 24 * 365 {
            return Err(anyhow::anyhow!(
                "Created at timestamp is too far in the past"
            ));
        }
        Ok(())
    }
}

impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tx_info = if let Some(stats) = &self.transaction_stats {
            format!(
                " ({} txs, {} wei)",
                stats.total_transactions, stats.total_volume
            )
        } else {
            String::new()
        };

        let last_tx = self
            .last_transaction_time()
            .map(|dt| format!(", last tx: {}", dt.format("%Y-%m-%d %H:%M")))
            .unwrap_or_default();

        let notes = self.notes.as_deref().unwrap_or("").to_string();
        let notes_display = if notes.is_empty() { "" } else { "\n  " };

        let tags_display = if self.tags.is_empty() {
            String::new()
        } else {
            format!("\n  Tags: {}", self.tags.join(", "))
        };

        // Format the main contact info
        write!(
            f,
            "{}{}{}{}{}",
            self.name.bold().green(),
            tx_info,
            last_tx,
            notes_display,
            notes
        )?;

        // Add tags if any
        if !self.tags.is_empty() {
            write!(f, "{}", tags_display.blue())?;
        }

        // Add address at the end
        write!(
            f,
            "\n  {}",
            format!("0x{}", &self.address.to_string()[2..]).on_green()
        )?;

        // Add notes if any (this was already handled in the main format)

        Ok(())
    }
}
