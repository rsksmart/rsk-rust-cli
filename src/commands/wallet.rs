use crate::types::wallet::{Wallet, WalletData};
use crate::utils::{constants, helper::Config, secrets::SecretPassword, table::TableBuilder};
use anyhow::{Result, anyhow};
use clap::Parser;
use colored::Colorize;
use alloy::signers::local::PrivateKeySigner;
use rpassword::prompt_password;
use zeroize::{Zeroize, Zeroizing};

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct WalletCommand {
    #[command(subcommand)]
    pub action: WalletAction,
}

#[derive(Parser)]
pub enum WalletAction {
    /// Create a new wallet (password is prompted interactively — never passed as an argument)
    Create {
        name: String,
    },
    /// Import a wallet by private key (key and password are prompted interactively)
    Import {
        name: String,
    },
    List,
    Switch {
        name: String,
    },
    Rename {
        old_name: String,
        new_name: String,
    },
    Backup {
        name: String,
        path: PathBuf,
    },
    Delete {
        name: String,
    },
}

// Sensitive fields (password, private key) are never stored in WalletAction —
// they are prompted interactively via rpassword and zeroized immediately after use.
impl std::fmt::Debug for WalletAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletAction::Create { name } => {
                f.debug_struct("Create").field("name", name).finish()
            }
            WalletAction::Import { name } => {
                f.debug_struct("Import").field("name", name).finish()
            }
            WalletAction::List => write!(f, "List"),
            WalletAction::Switch { name } => {
                f.debug_struct("Switch").field("name", name).finish()
            }
            WalletAction::Rename { old_name, new_name } => {
                f.debug_struct("Rename")
                    .field("old_name", old_name)
                    .field("new_name", new_name)
                    .finish()
            }
            WalletAction::Backup { name, path } => {
                f.debug_struct("Backup")
                    .field("name", name)
                    .field("path", path)
                    .finish()
            }
            WalletAction::Delete { name } => {
                f.debug_struct("Delete").field("name", name).finish()
            }
        }
    }
}

impl WalletCommand {
    pub async fn execute(&self) -> Result<()> {
        let config = Config::default(); // Use default config
        match &self.action {
            WalletAction::Create { name } => {
                self.create_wallet(&config, name).await?
            }
            WalletAction::Import { name } => {
                self.import_wallet(&config, name).await?
            }
            WalletAction::List => self.list_wallets(&config)?,
            WalletAction::Switch { name } => self.switch_wallet(name)?,
            WalletAction::Rename { old_name, new_name } => {
                self.rename_wallet(&config, old_name, new_name)?
            }
            WalletAction::Backup { name, path } => self.backup_wallet(&config, name, path)?,
            WalletAction::Delete { name } => self.delete_wallet(&config, name)?,
        }
        Ok(())
    }

    async fn create_wallet(&self, _config: &Config, name: &str) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        if wallet_file.exists() {
            let data = fs::read_to_string(&wallet_file)?;
            let wallet_data = serde_json::from_str::<WalletData>(&data)?;
            if wallet_data.get_wallet_by_name(name).is_some() {
                return Err(anyhow!("Wallet with name '{}' already exists", name));
            }
        }
        // Prompt interactively — never accept password as a CLI argument (shell history risk).
        let pwd = Zeroizing::new(prompt_password("Enter wallet password: ")?);
        let confirm = Zeroizing::new(prompt_password("Confirm wallet password: ")?);
        if *pwd != *confirm {
            return Err(anyhow!("Passwords do not match"));
        }
        let secret_password = SecretPassword::new(pwd.as_str().to_string());
        create_wallet_with_credentials(name, &secret_password)
    }

    async fn import_wallet(
        &self,
        _config: &Config,
        name: &str,
    ) -> Result<()> {
        // Prompt interactively — never accept private key or password as CLI arguments
        // (they would appear in shell history and `ps aux`).
        let mut raw_key = Zeroizing::new(prompt_password("Enter private key (input hidden): ")?);
        // Strip optional 0x prefix and whitespace before parsing.
        let trimmed = raw_key.trim().trim_start_matches("0x").to_string();
        raw_key.zeroize();
        let mut trimmed = Zeroizing::new(trimmed);
        let signer = PrivateKeySigner::from_str(&trimmed)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        trimmed.zeroize();

        let pwd = Zeroizing::new(prompt_password("Enter wallet password: ")?);
        let confirm = Zeroizing::new(prompt_password("Confirm wallet password: ")?);
        if *pwd != *confirm {
            return Err(anyhow!("Passwords do not match"));
        }
        let secret_password = SecretPassword::new(pwd.as_str().to_string());
        import_wallet_with_credentials(signer, name, &secret_password)
    }

    fn list_wallets(&self, _config: &Config) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        if !wallet_file.exists() {
            println!("No wallets found");
            return Ok(());
        }
        let data = fs::read_to_string(&wallet_file)?;
        let wallet_data = serde_json::from_str::<WalletData>(&data)?;
        let wallets = wallet_data.list_wallets();
        let mut table = TableBuilder::new();
        table.add_row(&["Name", "Address", "Created At", "Current"]);
        for wallet in wallets {
            let is_current = if let Some(current) = wallet_data.get_current_wallet() {
                current.address == wallet.address
            } else {
                false
            };
            table.add_row(&[
                &wallet.name,
                &format!("0x{:x}", wallet.address),
                &wallet.created_at,
                if is_current { "✓" } else { "" },
            ]);
        }
        table.print();
        Ok(())
    }

    fn switch_wallet(&self, name: &str) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        let data = fs::read_to_string(&wallet_file)?;
        let mut wallet_data = serde_json::from_str::<WalletData>(&data)?;
        let wallet_address = wallet_data
            .get_wallet_by_name(name)
            .ok_or_else(|| anyhow!("Wallet '{}' not found", name))?
            .address;
        let _ = wallet_data.switch_wallet(&format!("0x{:x}", wallet_address));
        crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
        println!("{}", format!("✅ Switched to wallet: {}", name).green());
        println!("Address: 0x{:x}", wallet_address);
        Ok(())
    }

    fn rename_wallet(&self, _config: &Config, old_name: &str, new_name: &str) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        if !wallet_file.exists() {
            return Err(anyhow!("No wallets found"));
        }
        let data = fs::read_to_string(&wallet_file)?;
        let mut wallet_data = serde_json::from_str::<WalletData>(&data)?;
        let wallet = wallet_data
            .get_wallet_by_name(old_name)
            .ok_or_else(|| anyhow!("Wallet '{}' not found", old_name))?;
        if new_name.is_empty() {
            return Err(anyhow!("New wallet name cannot be empty"));
        }
        if wallet_data.get_wallet_by_name(new_name).is_some() {
            return Err(anyhow!("Wallet with name '{}' already exists", new_name));
        }
        let address = format!("0x{:x}", wallet.address);
        if let Some(wallet) = wallet_data.wallets.get_mut(&address) {
            wallet.name = new_name.to_string();
        } else {
            return Err(anyhow!("Failed to rename wallet '{}'", old_name));
        }
        crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
        println!(
            "{}",
            format!("✅ Wallet renamed from '{}' to '{}'", old_name, new_name).green()
        );
        println!("Address: {}", address);
        Ok(())
    }

    fn backup_wallet(&self, _config: &Config, name: &str, path: &Path) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        if !wallet_file.exists() {
            return Err(anyhow!("No wallets found"));
        }
        let data = fs::read_to_string(&wallet_file)?;
        let wallet_data = serde_json::from_str::<WalletData>(&data)?;
        if name.ends_with(".json") {
            return Err(anyhow!(
                "Invalid wallet name '{}'. Use --name for the wallet name and --path for the filename.",
                name
            ));
        }
        let wallet = wallet_data
            .get_wallet_by_name(name)
            .ok_or_else(|| anyhow!("Wallet '{}' not found", name))?;
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write backup file with secure permissions (0o600)
        fs::write(path, serde_json::to_string_pretty(&wallet)?)?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)?;
        }
        
        println!("{}", "✅ Backup created successfully".green());
        println!("Backup saved at: {}", path.display());
        Ok(())
    }

    fn delete_wallet(&self, _config: &Config, name: &str) -> Result<()> {
        let wallet_file = constants::wallet_file_path();
        let data = fs::read_to_string(&wallet_file)?;
        let mut wallet_data = serde_json::from_str::<WalletData>(&data)?;
        let wallet = wallet_data
            .get_wallet_by_name(name)
            .ok_or_else(|| anyhow!("Wallet '{}' not found", name))?;
        let address = format!("0x{:x}", wallet.address);
        if wallet_data.current_wallet == address {
            return Err(anyhow!(
                "Cannot delete currently selected wallet. Please switch to a different wallet first."
            ));
        }
        let _ = wallet_data.remove_wallet(&address);
        crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
        println!("{}", format!("✅ Deleted wallet: {}", name).green());
        println!("Address: {}", address);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Public helpers — called by the interactive UI (interactive/wallet.rs) so it
// can supply pre-validated credentials without constructing a WalletCommand.
// Credentials are accepted via typed wrappers that zeroize on drop.
// ---------------------------------------------------------------------------

/// Create a new random wallet with a pre-validated password (e.g. from the TUI).
pub fn create_wallet_with_credentials(name: &str, password: &SecretPassword) -> Result<()> {
    let wallet_file = constants::wallet_file_path();
    if wallet_file.exists() {
        let data = fs::read_to_string(&wallet_file)?;
        let wallet_data = serde_json::from_str::<WalletData>(&data)?;
        if wallet_data.get_wallet_by_name(name).is_some() {
            return Err(anyhow!("Wallet with name '{}' already exists", name));
        }
    }
    let signer = PrivateKeySigner::random();
    let wallet = Wallet::new(signer, name, password)?;
    let mut wallet_data = if wallet_file.exists() {
        let data = fs::read_to_string(&wallet_file)?;
        serde_json::from_str::<WalletData>(&data)?
    } else {
        WalletData::new()
    };
    let _ = wallet_data.add_wallet(wallet.clone());
    crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
    println!("{}", "🎉 Wallet created successfully".green());
    println!("Address: {:?}", wallet.address());
    println!("Wallet saved at: {}", wallet_file.display());
    Ok(())
}

/// Import a wallet from a pre-parsed signer with a pre-validated password (e.g. from the TUI).
pub fn import_wallet_with_credentials(
    signer: PrivateKeySigner,
    name: &str,
    password: &SecretPassword,
) -> Result<()> {
    let wallet = Wallet::new(signer, name, password)?;
    let wallet_file = constants::wallet_file_path();
    let mut wallet_data = if wallet_file.exists() {
        let data = fs::read_to_string(&wallet_file)?;
        serde_json::from_str::<WalletData>(&data)?
    } else {
        WalletData::new()
    };
    let _ = wallet_data.add_wallet(wallet);
    crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
    println!("{}", "✅ Wallet imported successfully".green());
    println!("Wallet saved at: {}", wallet_file.display());
    Ok(())
}
