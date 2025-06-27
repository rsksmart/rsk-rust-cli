use thiserror::Error;
use ethers::prelude::ProviderError;
use std::fmt;

#[derive(Error, Debug)]
pub enum RskCliError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Wallet error: {0}")]
    WalletError(String),

    #[error("Invalid address format")]
    InvalidAddress,

    #[error("Invalid private key")]
    InvalidPrivateKey,

    #[error("RPC connection error: {0}")]
    RpcError(#[from] ProviderError),

    #[error("Invalid network configuration")]
    InvalidNetworkConfig,

    #[error("Insufficient funds")]
    InsufficientFunds,
}

impl fmt::Display for RskCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RskCliError>;