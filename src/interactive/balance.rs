use crate::commands::balance::BalanceCommand;
use crate::commands::tokens::TokenRegistry;
use crate::config::ConfigManager;
use anyhow::{Result, anyhow};
use console::style;
use inquire::Select;

/// Displays the balance checking interface
pub async fn show_balance() -> Result<()> {
    println!("\n{}", style("💰 Check Balance").bold());
    println!("{}", "=".repeat(30));

    // Get the current network from config
    let config = ConfigManager::new()?.load()?;
    let network = config.default_network.to_string().to_lowercase();
    println!("Using network: {}", network);

    // Load token registry
    let registry = TokenRegistry::load()
        .map_err(|e| {
            eprintln!("⚠️  Warning: Could not load token registry: {}", e);
            e
        })
        .unwrap_or_default();

    // Get tokens for the current network
    let mut tokens = registry.list_tokens(Some(&network));

    // Add RBTC as the first option
    tokens.insert(
        0,
        (
            "RBTC (Native)".to_string(),
            crate::commands::tokens::TokenInfo {
                address: "0x0000000000000000000000000000000000000000".to_string(),
                decimals: 18,
            },
        ),
    );

    if tokens.is_empty() {
        return Err(anyhow!("No tokens found for {} network", network));
    }

    // Create a vector of (display_name, token_info) pairs
    let token_choices: Vec<(String, crate::commands::tokens::TokenInfo)> = tokens
        .into_iter()
        .filter(|(_, info)| {
            // Only include tokens that match the current network or are RBTC
            info.address == "0x0000000000000000000000000000000000000000"
                || registry
                    .list_tokens(Some(&network))
                    .iter()
                    .any(|(_, token_info)| token_info.address == info.address)
        })
        .collect();

    // Get just the display names for the selection menu
    let token_display_names: Vec<String> =
        token_choices.iter().map(|(name, _)| name.clone()).collect();

    // Let the user select which token to check
    let selection = Select::new("Select token to check balance:", token_display_names).prompt()?;

    // Find the selected token info
    let (_, token_info) = token_choices
        .into_iter()
        .find(|(name, _)| name == &selection)
        .ok_or_else(|| anyhow!("Selected token not found"))?;

    // Clone the address since we need to use it in the command
    let token_address = token_info.address; // This is a String which is Clone

    // Execute the balance command
    let cmd = BalanceCommand {
        address: None, // Will use default wallet
        token: if token_address == "0x0000000000000000000000000000000000000000" {
            None
        } else {
            Some(token_address)
        },
    };

    cmd.execute().await
}
