use crate::commands::wallet::{WalletAction, WalletCommand};
use crate::utils::secrets::SecretPassword;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use console::style;
use std::str::FromStr;
use zeroize::{Zeroize, Zeroizing};

/// Validates password strength
fn validate_password(password: &str) -> Result<inquire::validator::Validation, Box<dyn std::error::Error + Send + Sync>> {
    if password.len() < 8 {
        return Ok(inquire::validator::Validation::Invalid("Password must be at least 8 characters long".into()));
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Ok(inquire::validator::Validation::Invalid("Password must contain at least one lowercase letter".into()));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Ok(inquire::validator::Validation::Invalid("Password must contain at least one uppercase letter".into()));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Ok(inquire::validator::Validation::Invalid("Password must contain at least one number".into()));
    }
    if !password.chars().any(|c| c.is_ascii_punctuation()) {
        return Ok(inquire::validator::Validation::Invalid("Password must contain at least one symbol (!@#$%^&* etc.)".into()));
    }
    Ok(inquire::validator::Validation::Valid)
}

/// Displays the wallet management menu
pub async fn wallet_menu() -> Result<()> {
    loop {
        let options = vec![
            String::from("📝 Create New Wallet"),
            String::from("📤 Import Wallet"),
            String::from("📋 List Wallets"),
            String::from("🔄 Switch Wallet"),
            String::from("✏️ Rename Wallet"),
            String::from("🔑 Export Private Key"),
            String::from("💾 Backup Wallet"),
            String::from("🗑️ Delete Wallet"),
            String::from("🏠 Back to Main Menu"),
        ];

        let selection = inquire::Select::new("Wallet Management", options)
            .prompt()
            .map_err(|_| anyhow::anyhow!("Failed to get selection"))?;

        let result = match selection.as_str() {
            "📝 Create New Wallet" => create_wallet().await,
            "📤 Import Wallet" => import_wallet().await,
            "📋 List Wallets" => list_wallets().await,
            "🔄 Switch Wallet" => switch_wallet().await,
            "✏️ Rename Wallet" => rename_wallet().await,
            "🔑 Export Private Key" => export_private_key().await,
            "💾 Backup Wallet" => backup_wallet().await,
            "🗑️ Delete Wallet" => delete_wallet().await,
            _ => break,
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
        }
    }
    Ok(())
}

/// Creates a new wallet with the given name and prompts for a password
async fn create_wallet() -> Result<()> {
    println!("\n{}", style("🆕 Create New Wallet").bold());
    println!("{}", "=".repeat(30));

    let name = inquire::Text::new("Wallet name:")
        .with_help_message("Enter a name for your new wallet")
        .prompt()?;

    // let _password = inquire::Password::new("Enter password:")
    //     .with_display_toggle_enabled()
    //     .with_display_mode(inquire::PasswordDisplayMode::Masked)
    //     .with_custom_confirmation_error_message("The passwords don't match.")
    //     .with_custom_confirmation_message("Please confirm your password:")
    //     .with_formatter(&|_| String::from("Password received"))
    //     .without_confirmation()
    //     .prompt()?;

    create_wallet_with_name(&name).await
}

/// Creates a new wallet with the given name without interactive prompts
/// This is used during initial setup
pub async fn create_wallet_with_name(name: &str) -> Result<()> {
    println!("\n{}", style("🔐 Create New Wallet").bold().blue());
    println!("{}", "-".repeat(30));

    println!(
        "\n{}",
        style("Please set a strong password to secure your wallet.").dim()
    );
    println!(
        "{}",
        style("This password will be required to access your wallet.").dim()
    );

    let password_str = inquire::Password::new("Enter password:")
        .with_display_toggle_enabled()
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_custom_confirmation_error_message("The passwords don't match.")
        .with_custom_confirmation_message("Please confirm your password:")
        .with_formatter(&|_| String::from("✓ Password set"))
        .with_validator(validate_password)
        .prompt()?;

    println!(
        "\n{}",
        style("⏳ Creating your wallet. This may take a few seconds...").dim()
    );

    let secret_password = SecretPassword::new(password_str);
    // Call the credential-accepting helper directly; never route through CLI args.
    let result = crate::commands::wallet::create_wallet_with_credentials(name, &secret_password);
    result
}

async fn import_wallet() -> Result<()> {
    println!("\n{}", style("📤 Import Wallet").bold().blue());
    println!("{}", "-".repeat(30));

    println!(
        "\n{}",
        style("Please enter the private key of the wallet you want to import.").dim()
    );
    println!(
        "{}",
        style("This should start with '0x' followed by 64 hexadecimal characters.").dim()
    );

    let mut private_key = inquire::Text::new("Private key (0x...):")
        .with_help_message("The private key of the wallet to import (will be masked)")
        .with_validator(|input: &str| {
            if !input.starts_with("0x") {
                return Ok(inquire::validator::Validation::Invalid("Private key must start with '0x'".into()));
            }
            if input.len() != 66 {
                return Ok(inquire::validator::Validation::Invalid("Private key must be 66 characters (0x + 64 hex chars)".into()));
            }
            if !input[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(inquire::validator::Validation::Invalid("Private key must contain only hexadecimal characters".into()));
            }
            Ok(inquire::validator::Validation::Valid)
        })
        .with_formatter(&|input| {
            if input.is_empty() {
                String::new()
            } else if input.len() <= 2 {
                input.to_string()
            } else {
                format!("0x{}", "*".repeat(input.len() - 2))
            }
        })
        .prompt()?;

    let name = inquire::Text::new("Wallet name:")
        .with_help_message("A name to identify this wallet in the app")
        .prompt()?;

    println!(
        "\n{}",
        style("Please set a strong password to secure your imported wallet.").dim()
    );
    println!(
        "{}",
        style("This password will be required to access your wallet.").dim()
    );

    let password_str = inquire::Password::new("Enter password:")
        .with_display_toggle_enabled()
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_custom_confirmation_error_message("The passwords don't match.")
        .with_custom_confirmation_message("Please confirm your password:")
        .with_formatter(&|_| String::from("✓ Password set"))
        .with_validator(validate_password)
        .prompt()?;

    println!(
        "\n{}",
        style("⏳ Importing your wallet. This may take a few seconds...").dim()
    );

    // Parse the private key, then zeroize the raw string immediately.
    let mut raw = Zeroizing::new(private_key.trim().trim_start_matches("0x").to_string());
    private_key.zeroize();
    let signer = PrivateKeySigner::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
    raw.zeroize();

    let secret_password = SecretPassword::new(password_str);
    // Call the credential-accepting helper directly; never route through CLI args.
    let result = crate::commands::wallet::import_wallet_with_credentials(signer, &name, &secret_password);

    match result {
        Ok(_) => {
            println!("\n{}", style("✅ Wallet imported successfully!").green());
        }
        Err(e) => {
            println!("\n{}", style(&format!("❌ Failed to import wallet: {}", e)).red());
            return Err(e);
        }
    }

    Ok(())
}

async fn list_wallets() -> Result<()> {
    let cmd = WalletCommand {
        action: WalletAction::List,
    };
    cmd.execute().await
}

async fn switch_wallet() -> Result<()> {
    println!("\n{}", style("🔄 Switch Wallet").bold());
    println!("{}", "=".repeat(30));

    let cmd = WalletCommand {
        action: WalletAction::List,
    };

    // List wallets and let user select one
    cmd.execute().await?;

    let wallet_name = inquire::Text::new("Enter the name of the wallet to switch to:")
        .with_help_message("Enter the exact name of the wallet to switch to")
        .prompt()?;

    let switch_cmd = WalletCommand {
        action: WalletAction::Switch { name: wallet_name },
    };

    switch_cmd.execute().await?;

    Ok(())
}

async fn rename_wallet() -> Result<()> {
    println!("\n{}", style("✏️ Rename Wallet").bold());
    println!("{}", "=".repeat(30));

    // First, list all wallets
    let list_cmd = WalletCommand {
        action: WalletAction::List,
    };
    list_cmd.execute().await?;

    // Get current wallet name
    let old_name = inquire::Text::new("Enter the name of the wallet to rename:")
        .with_help_message("Enter the exact name of the wallet to rename")
        .prompt()?;

    // Get new wallet name
    let new_name = inquire::Text::new("Enter the new name for the wallet:")
        .with_help_message("Enter the new name for the wallet")
        .prompt()?;

    let rename_cmd = WalletCommand {
        action: WalletAction::Rename {
            old_name: old_name.clone(),
            new_name: new_name.clone(),
        },
    };

    rename_cmd.execute().await?;

    println!(
        "\n{} {} {}",
        style("✅ Wallet").green(),
        style(&old_name).bold(),
        style(format!("renamed to {}", new_name)).green()
    );

    Ok(())
}

/// Show private key for the current wallet (like MetaMask)
async fn export_private_key() -> Result<()> {
    use dialoguer::Confirm;
    use std::fs;
    
    println!("\n{}", style("🔑 Show Private Key").bold().red());
    println!("{}", "=".repeat(30));
    
    // Security warning
    println!("{}", style("⚠️  WARNING: Never share your private key!").red().bold());
    println!("{}", style("• Anyone with this key can access your funds").yellow());
    println!("{}", style("• Make sure no one is watching your screen").yellow());
    
    let confirm = Confirm::new()
        .with_prompt("I understand the risks, show my private key")
        .default(false)
        .interact()?;
        
    if !confirm {
        return Ok(());
    }
    
    // Load wallet data from file
    let wallet_file = crate::utils::constants::wallet_file_path();
    if !wallet_file.exists() {
        println!("{}", style("❌ No wallets found").red());
        return Ok(());
    }
    
    let data = fs::read_to_string(&wallet_file)?;
    let wallet_data: crate::types::wallet::WalletData = serde_json::from_str(&data)?;
    
    let current_wallet = wallet_data.get_current_wallet().ok_or_else(|| {
        anyhow::anyhow!("No wallet selected")
    })?;
    
    let password_str = inquire::Password::new("Enter wallet password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()?;

    println!(
        "\n{}",
        style("⏳ Decrypting your private key. This may take a few seconds...").dim()
    );

    let secret_password = SecretPassword::new(password_str);
    match current_wallet.decrypt_private_key(&secret_password) {
        Ok(private_key_secret) => {
            // secret_password is automatically zeroized when it goes out of scope
            println!("\n{}", style("Your Private Key:").bold());
            println!("{}", style(private_key_secret.expose()).cyan().bold());
            // private_key_secret is automatically zeroized when it goes out of scope
            println!("\n{}", style("⚠️  Keep this safe and never share it!").red());
        }
        Err(_) => {
            // secret_password is automatically zeroized when it goes out of scope
            println!("{}", style("❌ Incorrect password").red());
        }
    }
    
    Ok(())
}

async fn backup_wallet() -> Result<()> {
    use std::path::PathBuf;

    println!("\n{}", style("💾 Backup Wallet").bold());
    println!("{}", "=".repeat(30));

    // First, list all wallets
    let list_cmd = WalletCommand {
        action: WalletAction::List,
    };
    list_cmd.execute().await?;

    // Get wallet name
    let wallet_name = inquire::Text::new("Enter the name of the wallet to backup:")
        .with_help_message("Enter the exact name of the wallet to backup")
        .prompt()?;

    // Get backup directory
    let backup_path = inquire::Text::new(
        "Enter the directory to save the backup (leave empty for current directory):",
    )
    .with_help_message("Enter the directory path or press Enter for current directory")
    .with_default(".")
    .prompt()?;

    let backup_path = PathBuf::from(backup_path);

    let backup_cmd = WalletCommand {
        action: WalletAction::Backup {
            name: wallet_name.clone(),
            path: backup_path,
        },
    };

    backup_cmd.execute().await?;

    println!(
        "\n{} {}",
        style("✅ Wallet backup created for:").green(),
        style(wallet_name).bold()
    );

    Ok(())
}

async fn delete_wallet() -> Result<()> {
    println!("\n{}", style("🗑️ Delete Wallet").bold());
    println!("{}", "=".repeat(30));

    // First, list all wallets
    let list_cmd = WalletCommand {
        action: WalletAction::List,
    };
    list_cmd.execute().await?;

    // Get wallet name to delete
    let wallet_name = inquire::Text::new("Enter the name of the wallet to delete:")
        .with_help_message("Enter the exact name of the wallet to delete")
        .prompt()?;

    let confirmed = inquire::Confirm::new(&format!(
        "⚠️ Are you sure you want to delete wallet '{}'? This action cannot be undone.",
        wallet_name
    ))
    .with_default(false)
    .prompt()?;

    if confirmed {
        let delete_cmd = WalletCommand {
            action: WalletAction::Delete {
                name: wallet_name.clone(),
            },
        };

        delete_cmd.execute().await?;
        println!(
            "\n{} {}",
            style("✅ Wallet deleted:").green(),
            style(wallet_name).bold()
        );
    } else {
        println!("\n{}", style("❌ Deletion cancelled").yellow());
    }

    Ok(())
}
