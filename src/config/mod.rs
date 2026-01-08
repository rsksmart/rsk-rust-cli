mod config;
mod doctor;
mod setup;

// Re-export types from the config module
pub use config::{Config, ConfigManager};

// Re-export Network from the types module
pub use crate::types::network::Network;

// Re-export setup and doctor functions
pub use doctor::run_doctor;
pub use setup::run_setup_wizard;

// API Documentation URLs
pub const RSK_RPC_DOCS_URL: &str = "https://dev.rootstock.io/developers/rpc-api/";
pub const ALCH_MAINNET_URL: &str = "https://dashboard.alchemy.com/apps/create?referrer=/apps";
pub const ALCH_TESTNET_URL: &str =
    "https://dashboard.alchemy.com/apps/create?referrer=/apps&chain=rsk-testnet";
pub const DOCS_URL: &str = "https://github.com/rsksmart/rsk-rust-cli/wiki";
