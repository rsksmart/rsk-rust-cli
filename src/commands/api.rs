use crate::types::wallet::WalletData;
use crate::utils::constants;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::fs;

#[derive(Parser, Debug)]
pub struct SetApiKeyCommand {
    /// Alchemy API key to set
    #[arg(long, required = true)]
    pub api_key: String,
}

impl SetApiKeyCommand {
    pub async fn execute(&self) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        let mut wallet_data = if wallet_file.exists() {
            let data = fs::read_to_string(&wallet_file)?;
            serde_json::from_str::<WalletData>(&data)?
        } else {
            WalletData::new()
        };

        wallet_data.api_key = Some(self.api_key.clone());
        fs::write(&wallet_file, serde_json::to_string_pretty(&wallet_data)?)?;
        println!("{}: API key set successfully", "Success".green().bold());
        Ok(())
    }
}
