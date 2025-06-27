use crate::commands::wallet::{WalletAction, WalletCommand};
use anyhow::Result;
use console::style;

/// Displays the wallet management menu
pub async fn wallet_menu() -> Result<()> {
    loop {
        let options = vec![
            String::from("📝 Create New Wallet"),
            String::from("📤 Import Wallet"),
            String::from("📋 List Wallets"),
            String::from("🔄 Switch Wallet"),
            String::from("✏️ Rename Wallet"),
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
    
    println!("\n{}", style("Please set a strong password to secure your wallet.").dim());
    println!("{}", style("This password will be required to access your wallet.").dim());
    
    let password = inquire::Password::new("Enter password:")
        .with_display_toggle_enabled()
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_custom_confirmation_error_message("The passwords don't match.")
        .with_custom_confirmation_message("Please confirm your password:")
        .with_formatter(&|_| String::from("✓ Password set"))
        .prompt()?;

    println!("\n{}", style("⏳ Creating your wallet. This may take a few seconds...").dim());
    
    let cmd = WalletCommand {
        action: WalletAction::Create { 
            name: name.to_string(),
            password: password.clone()
        },
    };

    cmd.execute().await?;
    Ok(())
}

async fn import_wallet() -> Result<()> {
    println!("\n{}", style("📤 Import Wallet").bold().blue());
    println!("{}", "-".repeat(30));
    
    println!("\n{}", style("Please enter the private key of the wallet you want to import.").dim());
    println!("{}", style("This should start with '0x' followed by 64 hexadecimal characters.").dim());
    
    let private_key = inquire::Password::new("Private key (0x...):")
        .with_display_mode(inquire::PasswordDisplayMode::Hidden)
        .with_help_message("The private key of the wallet to import")
        .prompt()?;

    let name = inquire::Text::new("Wallet name:")
        .with_help_message("A name to identify this wallet in the app")
        .prompt()?;
        
    println!("\n{}", style("Please set a strong password to secure your imported wallet.").dim());
    println!("{}", style("This password will be required to access your wallet.").dim());
    
    let password = inquire::Password::new("Enter password:")
        .with_display_toggle_enabled()
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_custom_confirmation_error_message("The passwords don't match.")
        .with_custom_confirmation_message("Please confirm your password:")
        .with_formatter(&|_| String::from("✓ Password set"))
        .prompt()?;

    println!("\n{}", style("⏳ Importing your wallet. This may take a few seconds...").dim());

    let cmd = WalletCommand {
        action: WalletAction::Import {
            private_key: private_key.clone(),
            name: name.clone(),
            password: password.clone(),
        },
    };

    cmd.execute().await?;

    println!("\n{}", style("✅ Wallet imported successfully!").green());
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
