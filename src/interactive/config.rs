use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Password, Select, theme::ColorfulTheme};

// Import config and API types
use crate::api::ApiProvider;
use crate::config::ConfigManager;
use crate::types::network::Network;

// This module provides configuration management functionality

pub async fn show_config_menu() -> Result<()> {
    let config_manager = ConfigManager::new()?;

    loop {
        // Reload config in each iteration to show current state
        let config = config_manager.load()?;

        clearscreen::clear().ok();

        println!(
            "\n{}",
            style("⚙️  Configuration").bold().blue().underlined()
        );
        println!("{}\n", "-".repeat(40));

        // Show current settings
        println!("  {}", style("Current Settings:").bold());
        println!("  • Network: {}", style(config.default_network).cyan());

        // Show current API key status
        let providers = [
            (ApiProvider::RskRpc, "RSK RPC (for blockchain operations)"),
            (ApiProvider::Alchemy, "Alchemy (for transaction history)"),
        ];

        println!("  {}", style("API Keys:").bold());

        for (provider, name) in &providers {
            let status = if let Some(_key) = config.get_api_key(provider) {
                format!("{} (set)", style(name).green().bold())
            } else {
                format!("{} (not set)", style(name).dim())
            };
            println!("    • {}: {}", name, status);
        }

        // Show default wallet if set
        if let Some(wallet) = &config.default_wallet {
            println!("  • Default Wallet: {}", style(wallet).dim());
        }

        let options = vec![
            format!("{}  Change Network", style("🌐").bold().blue()),
            format!("{}  Manage API Keys", style("🔑").bold().green()),
            format!("{}  Clear Cache & Reset", style("🧹").bold().red()),
            format!("{}  Back to Main Menu", style("⬅️").bold().blue()),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("\nWhat would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => change_network(&config_manager).await?,
            1 => manage_api_keys(&config_manager).await?,
            2 => {
                let confirm = Confirm::new()
                    .with_prompt("⚠️  WARNING: This will delete ALL wallet data and cannot be undone! Continue?")
                    .default(false)
                    .interact()?;

                if confirm {
                    config_manager.clear_cache()?;
                    println!("\n✅ Cache and all wallet data have been cleared successfully.");
                    println!("Please restart the wallet to complete the reset process.");
                    std::process::exit(0);
                } else {
                    println!("\nOperation cancelled. No data was deleted.");
                }
            }
            3 => break,
            _ => {}
        }
    }

    Ok(())
}

async fn manage_api_keys(config_manager: &ConfigManager) -> Result<()> {
    loop {
        let config = config_manager.load()?;
        clearscreen::clear().ok();

        println!(
            "\n{}",
            style("🔑 API Key Management").bold().blue().underlined()
        );
        println!("{}\n", "-".repeat(40));

        // Show current API keys
        println!("  {}", style("Current API Keys:").bold());

        if config.api.keys.is_empty() {
            println!("  No API keys configured");
        } else {
            for (i, key) in config.api.keys.iter().enumerate() {
                let name = key.name.as_deref().unwrap_or("Unnamed");
                println!(
                    "  {}. {} - {} ({})",
                    i + 1,
                    style(name).bold(),
                    key.provider,
                    key.network
                );
            }
        }

        let options = vec![
            format!("{}  Add API Key", style("+").bold().green()),
            format!("{}  Remove API Key", style("-").bold().red()),
            format!("{}  Back to Configuration", style("⬅️").bold().blue()),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("\nWhat would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => add_api_key(config_manager).await?,
            1 => remove_api_key(config_manager).await?,
            2 => break,
            _ => {}
        }
    }

    Ok(())
}

async fn add_api_key(config_manager: &ConfigManager) -> Result<()> {
    let mut config = config_manager.load()?;

    // Select provider
    let providers = [
        (ApiProvider::RskRpc, "RSK RPC (for blockchain operations)"),
        (ApiProvider::Alchemy, "Alchemy (for transaction history)"),
    ];

    let provider_names: Vec<_> = providers.iter().map(|(_, name)| *name).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select API provider:")
        .items(&provider_names)
        .default(0)
        .interact()?;

    let (provider, _) = &providers[selection];

    // Get API key
    let key: String = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your API key")
        .interact()?;

    // Get optional name
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter a name for this key (optional)")
        .allow_empty(true)
        .interact_text()?;

    let name = if name.trim().is_empty() {
        None
    } else {
        Some(name.trim().to_string())
    };

    // Clone the provider since we're borrowing from the array
    let provider = (*provider).clone();

    // Save the API key
    let message = config.set_api_key(provider, key, name);
    config_manager.save(&config)?;

    println!("\n{}", style(message).green().bold());
    println!("\n{}", style("Press Enter to continue...").dim());
    let _ = std::io::stdin().read_line(&mut String::new());

    Ok(())
}

async fn remove_api_key(config_manager: &ConfigManager) -> Result<()> {
    let mut config = config_manager.load()?;

    if config.api.keys.is_empty() {
        println!("\n{}", style("No API keys to remove").yellow().bold());
        return Ok(());
    }

    // Show list of keys to remove
    let key_names: Vec<String> = config
        .api
        .keys
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let name = key.name.as_deref().unwrap_or("Unnamed");
            format!("{} - {} ({})", i + 1, name, key.provider)
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select API key to remove:")
        .items(&key_names)
        .interact()?;

    let removed_key = config.api.keys.remove(selection);
    config_manager.save(&config)?;

    println!(
        "\n{} Removed API key for {} ({})",
        style("✓").green().bold(),
        removed_key.provider,
        removed_key.network
    );

    println!("\n{}", style("Press Enter to continue...").dim());
    let _ = std::io::stdin().read_line(&mut String::new());

    Ok(())
}

async fn change_network(config_manager: &ConfigManager) -> Result<()> {
    let mut config = config_manager.load()?;

    // Define all available networks with their display names
    let networks = [
        Network::Mainnet,
        Network::Testnet,
        Network::Regtest,
        Network::AlchemyMainnet,
        Network::AlchemyTestnet,
        Network::RootStockMainnet,
        Network::RootStockTestnet,
    ];

    let network_descriptions = [
        "Mainnet (Production, real RSK)",
        "Testnet (Test network, free test tokens)",
        "Regtest (Local development)",
        "Alchemy Mainnet (Production, Alchemy RPC)",
        "Alchemy Testnet (Test network, Alchemy RPC)",
        "Rootstock Mainnet (Production, Rootstock RPC)",
        "Rootstock Testnet (Test network, Rootstock RPC)",
    ];

    let current_network = config.default_network;

    // Find the current network's index
    let current_index = networks
        .iter()
        .position(|&n| n == current_network)
        .unwrap_or(0);

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select network:")
        .items(&network_descriptions)
        .default(current_index)
        .interact()?;

    let selected_network = networks[selection];

    // Always update the network, even if it's the same, to ensure consistency
    config.default_network = selected_network;

    // Save the updated config
    config_manager.save(&config)?;

    println!(
        "\n{} Network changed to: {}",
        style("✓").green().bold(),
        style(selected_network).bold()
    );

    // Show a brief confirmation before returning to menu
    println!("\n{}", style("Press Enter to continue...").dim());
    let _ = std::io::stdin().read_line(&mut String::new());

    Ok(())
}
