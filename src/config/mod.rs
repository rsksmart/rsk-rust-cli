mod config;
mod setup;
mod doctor;

// Re-export types from the config module
pub use config::{Config, ConfigManager};

// Re-export Network from the types module
pub use crate::types::network::Network;

// Re-export setup and doctor functions
pub use setup::run_setup_wizard;
pub use doctor::run_doctor;

pub const ALCH_MAINNET_URL: &str = "https://dashboard.alchemy.com/apps/create?referrer=/apps";
pub const ALCH_TESTNET_URL: &str = "https://dashboard.alchemy.com/apps/create?referrer=/apps&chain=rsk-testnet";
pub const DOCS_URL: &str = "https://github.com/cosmasken/rootstock-wallet/wiki";