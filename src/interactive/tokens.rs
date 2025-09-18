use crate::commands::tokens;
use anyhow::Result;
use console::style;
use inquire::validator::Validation;

/// Displays the token management menu
pub async fn token_menu() -> Result<()> {
    loop {
        let options = vec![
            String::from("➕ Add Token"),
            String::from("🗑️ Remove Token"),
            String::from("📋 List Tokens"),
            String::from("🏠 Back to Main Menu"),
        ];

        let selection = inquire::Select::new("Token Management", options)
            .prompt()
            .map_err(|_| anyhow::anyhow!("Failed to get selection"))?;

        match selection.as_str() {
            "➕ Add Token" => add_token().await?,
            "🗑️ Remove Token" => remove_token().await?,
            "📋 List Tokens" => list_tokens().await?,
            _ => break,
        }
    }
    Ok(())
}

async fn add_token() -> Result<()> {
    println!("\n{}", style("➕ Add Token").bold());
    println!("{}", "=".repeat(30));

    // Select network
    let network = inquire::Select::new(
        "Select network:",
        vec![String::from("mainnet"), String::from("testnet")],
    )
    .prompt()?
    .to_string();

    let symbol = inquire::Text::new("Token symbol (e.g., USDT):")
        .with_help_message("Enter the token's ticker symbol")
        .prompt()?;

    let address = inquire::Text::new("Token contract address (0x...):")
        .with_validator(|input: &str| {
            if input.starts_with("0x") && input.len() == 42 {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Please enter a valid token contract address (0x...)".into(),
                ))
            }
        })
        .prompt()?;

    let decimals = inquire::Text::new("Token decimals (e.g., 18):")
        .with_default("18")
        .with_validator(|input: &str| match input.parse::<u8>() {
            Ok(_) => Ok(Validation::Valid),
            Err(_) => Ok(Validation::Invalid(
                "Please enter a valid number (0-255)".into(),
            )),
        })
        .prompt()?
        .parse::<u8>()?;

    // Save the token to the user's token list
    match tokens::add_token(&network, &symbol, &address, decimals) {
        Ok(_) => {
            println!(
                "\n{} {}",
                style("✅ Token added:").green(),
                style(format!("{} ({}) on {}", symbol, address, network)).bold()
            );
        }
        Err(e) => {
            eprintln!(
                "\n{} {}",
                style("❌ Failed to add token:").red(),
                style(e).bold()
            );
        }
    }

    Ok(())
}

async fn remove_token() -> Result<()> {
    println!("\n{}", style("🗑️ Remove Token").bold());
    println!("{}", "=".repeat(30));

    // Select network
    let network = inquire::Select::new(
        "Select network:",
        vec![String::from("mainnet"), String::from("testnet")],
    )
    .prompt()?
    .to_string();

    // Get token symbol to remove
    let symbol = inquire::Text::new("Token symbol to remove (e.g., USDT):")
        .with_help_message("Enter the token's ticker symbol to remove")
        .prompt()?;

    // Remove the token
    match tokens::remove_token(&network, &symbol) {
        Ok(_) => {
            println!(
                "\n{} {}",
                style("✅ Token removed:").green(),
                style(&symbol).bold()
            );
        }
        Err(e) => {
            eprintln!(
                "\n{} {}",
                style("❌ Failed to remove token:").red(),
                style(e).bold()
            );
        }
    }

    Ok(())
}

async fn list_tokens() -> Result<()> {
    println!("\n{}", style("📋 Your Tokens").bold());
    println!("{}", "=".repeat(30));

    // Select network
    let network = inquire::Select::new(
        "Select network:",
        vec![
            String::from("all"),
            String::from("mainnet"),
            String::from("testnet"),
        ],
    )
    .prompt()?;

    let network_filter = if network == "all" {
        None
    } else {
        Some(network.as_str())
    };

    // List tokens
    match tokens::list_tokens(network_filter) {
        Ok(tokens) => {
            if tokens.is_empty() {
                println!("\nNo tokens found");
            } else {
                println!("\n{:<15} {:<42} DECIMALS", "SYMBOL", "ADDRESS");
                println!("{}", "-".repeat(70));

                for (symbol, info) in tokens {
                    println!("{:<15} {:<42} {}", symbol, info.address, info.decimals);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "\n{} {}",
                style("❌ Failed to list tokens:").red(),
                style(e).bold()
            );
        }
    }

    Ok(())
}
