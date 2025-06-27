use crate::commands::api::SetApiKeyCommand;
use crate::commands::contacts::ContactsCommand;
use crate::commands::tokens::{TokenAddCommand, TokenListCommand, TokenRemoveCommand};
use crate::commands::wallet::WalletCommand;
use clap::Parser;

#[derive(Parser, Debug)]
pub enum Commands {
    /// Manage wallets
    Wallet(WalletCommand),
    /// Manage contacts
    Contacts(ContactsCommand),
    /// Show transaction history
    History {
        #[arg(short, long, default_value = "10")]
        limit: usize,
        #[arg(short, long)]
        address: Option<String>,
        #[arg(short, long)]
        token: Option<String>,
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        incoming: bool,
        #[arg(short, long)]
        outgoing: bool,
        /// Alchemy API key (optional, saved in wallet after first use)
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long, default_value = "mainnet")]
        network: String,
    },
    /// Check balance of an address
    Balance {
        /// Network to use (mainnet/testnet)
        #[arg(long, default_value = "mainnet")]
        network: String,
        /// Token symbol to check balance for (e.g., RBTC, RIF, DoC)
        #[arg(long)]
        token: Option<String>,
        /// Address to check balance for (optional if using default wallet)
        #[arg(long)]
        address: Option<String>,
    },
    /// Transfer RBTC or tokens
    Transfer {
        /// Address to send to
        #[arg(long, required = true)]
        address: String,
        /// Amount to send (in RBTC or token units)
        #[arg(long, required = true)]
        value: f64,
        /// Token address (for ERC20 transfers)
        #[arg(long)]
        token: Option<String>,
        #[arg(short, long, default_value = "mainnet")]
        network: String,
    },

    SetApiKey(SetApiKeyCommand),

    /// Add a new token to the registry
    TokenAdd(TokenAddCommand),

    /// Remove a token from the registry
    TokenRemove(TokenRemoveCommand),

    /// List tokens in the registry
    TokenList(TokenListCommand),
}
