use crate::commands::history::HistoryCommand;
use crate::commands::tokens::{TokenRegistry, list_tokens};
use crate::config::ConfigManager;
use anyhow::{Result, Context};
use console::style;
use inquire::{Select, Text, validator::Validation, Confirm};

/// Shows the transaction history in an interactive way
pub async fn show_history() -> Result<()> {
    println!("\n{}", style("📜 Transaction History").bold());
    println!("{}", "=".repeat(30));

    // Load config and get current network
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;
    
    // Network selection
    let network_options = vec!["mainnet", "testnet"];
    let network_selection = Select::new("Select network:", network_options)
        .with_starting_cursor(if config.default_network.to_string().to_lowercase().contains("testnet") { 1 } else { 0 })
        .prompt()?;

    // Default values for the history command
    let mut command = HistoryCommand {
        address: None,
        contact: None,
        limit: 10,
        detailed: false,
        status: None,
        token: None,
        from: None,
        to: None,
        sort_by: "timestamp".to_string(),
        sort_order: "desc".to_string(),
        incoming: false,
        outgoing: false,
        export_csv: None,
        api_key: match network_selection {
            "mainnet" => config.alchemy_mainnet_key.clone(),
            "testnet" => config.alchemy_testnet_key.clone(),
            _ => None,
        },
        network: network_selection.to_string(),
    };

    // Load available tokens for the selected network
    let registry = TokenRegistry::load()
        .map_err(|e| anyhow::anyhow!("Failed to load token registry: {}", e))?;
    let tokens = registry.list_tokens(Some(network_selection));
    let mut token_options = vec!["RBTC (Native)".to_string()];
    token_options.extend(tokens.into_iter().map(|(symbol, _info)| symbol));

    // Main history menu loop
    loop {
        // Show current filters
        println!(
            "\n{}{}",
            style("Current Filters:").bold().blue(),
            " ".repeat(15)
        );
        println!("Network: {}", command.network);
        println!(
            "Token: {}",
            command.token.as_deref().unwrap_or("All Tokens")
        );
        if let Some(status) = &command.status {
            println!("Status: {}", status);
        }
        if command.incoming {
            println!("Showing: Incoming transactions");
        }
        if command.outgoing {
            println!("Showing: Outgoing transactions");
        }
        println!("Limit: {} transactions", command.limit);
        println!("{}", "-".repeat(40));

        // Check if we have an API key, prompt if not
        if command.api_key.is_none() {
            println!("\n{}", style("⚠️  Alchemy API Key Required").yellow().bold());
            println!("Transaction history requires an Alchemy API key.");
            
            let should_add_key = Confirm::new("Would you like to add an API key now?")
                .with_default(true)
                .prompt()
                .unwrap_or(false);

            if should_add_key {
                let api_key = Text::new("Enter your Alchemy API key:")
                    .with_help_message("Get one at https://www.alchemy.com/")
                    .prompt()?;
                
                if !api_key.trim().is_empty() {
                    // Save the API key using ConfigManager
                    let mut config = config_manager.load()?;
                    match network_selection {
                        "mainnet" => config.alchemy_mainnet_key = Some(api_key.trim().to_string()),
                        "testnet" => config.alchemy_testnet_key = Some(api_key.trim().to_string()),
                        _ => {}
                    }
                    config_manager.save(&config)?;
                    
                    println!("\n{}", style("✅ API key saved successfully!").green());
                    command.api_key = Some(api_key.trim().to_string());
                } else {
                    println!("\n{}", style("❌ No API key provided. Cannot fetch transaction history.").red());
                    println!("You can add an API key later from the Configuration menu.");
                    return Ok(());
                }
            } else {
                println!("\n{}", style("⚠️  Transaction history requires an API key.").yellow());
                println!("You can add an API key later from the Configuration menu.");
                return Ok(());
            }
        }

        // Execute the command and show results
        match command.execute().await {
            Ok(_) => {}
            Err(e) => {
                if e.to_string().contains("API key") {
                    println!("\n{}", style("❌ Error: Invalid or missing Alchemy API key").red());
                    println!("Please check your API key and try again.");
                    println!("You can update your API key in the Configuration menu.");
                    return Ok(());
                } else {
                    return Err(e).context("Failed to fetch transaction history");
                }
            }
        }

        // Show options for further actions
        let options = vec![
            "Export to CSV",
            "Change network",
            "Change token",
            "Change limit",
            "Filter by status",
            "Toggle incoming/outgoing",
            "Toggle detailed view",           
            "Clear all filters",
            "Filter by date range",
            "Back to main menu",
        ];

        let selection = Select::new("\nSelect an option:", options.clone()).prompt()?;

        match selection {
            "Change network" => {
                let network = Select::new("Select network:", vec!["mainnet", "testnet"])
                    .with_starting_cursor(if command.network == "mainnet" { 0 } else { 1 })
                    .prompt()?;

                if network != command.network {
                    command.network = network.to_string();
                    // Reload tokens for the new network
                    match list_tokens(Some(&command.network)) {
                        Ok(tokens) => {
                            token_options = std::iter::once("RBTC (Native)".to_string())
                                .chain(tokens.into_iter().map(|(symbol, _info)| symbol))
                                .collect();
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to load tokens: {}. Using default token options.",
                                e
                            );
                            token_options = vec!["RBTC (Native)".to_string()];
                        }
                    }
                }
            }
            "Change token" => {
                let token = Select::new("Select token:", token_options.clone()).prompt()?;
                command.token = if token == "RBTC (Native)" {
                    None
                } else {
                    Some(token.split_whitespace().next().unwrap_or("").to_string())
                };
            }
            "Change limit" => {
                let limit = Text::new("Enter number of transactions to show (1-100):")
                    .with_default(&command.limit.to_string())
                    .with_validator(|input: &str| match input.parse::<u32>() {
                        Ok(n) if n > 0 && n <= 100 => Ok(Validation::Valid),
                        _ => Ok(Validation::Invalid(
                            "Please enter a number between 1 and 100".into(),
                        )),
                    })
                    .prompt()?;
                command.limit = limit.parse::<u32>().unwrap().clamp(1, 100);
            }
            "Filter by status" => {
                let status_options = vec!["Any", "Pending", "Success", "Failed"];
                let status = Select::new("Select status:", status_options).prompt()?;
                command.status = if status == "Any" {
                    None
                } else {
                    Some(status.to_lowercase())
                };
            }
            "Toggle incoming/outgoing" => {
                let options = vec!["Both", "Incoming only", "Outgoing only"];
                let selection = Select::new("Filter transactions:", options).prompt()?;
                match selection {
                    "Incoming only" => {
                        command.incoming = true;
                        command.outgoing = false;
                    }
                    "Outgoing only" => {
                        command.incoming = false;
                        command.outgoing = true;
                    }
                    _ => {
                        command.incoming = false;
                        command.outgoing = false;
                    }
                }
            }
            "Export to CSV" => {
                let filename = Text::new("Enter filename to save (e.g., transactions.csv):")
                    .with_default("transactions.csv")
                    .with_validator(|input: &str| {
                        if input.ends_with(".csv") {
                            Ok(Validation::Valid)
                        } else {
                            Ok(Validation::Invalid("Filename must end with .csv".into()))
                        }
                    })
                    .prompt()?;
                
                let mut export_cmd = command.clone();
                export_cmd.export_csv = Some(filename);
                
                match export_cmd.execute().await {
                    Ok(_) => {}
                    Err(e) => eprintln!("Error exporting to CSV: {}", e),
                }
                
                continue;
            }
            "Toggle detailed view" => {
                command.detailed = !command.detailed;
                println!(
                    "Detailed view: {}",
                    if command.detailed { "ON" } else { "OFF" }
                );
            }
            "Clear all filters" => {
                command.status = None;
                command.token = None;
                command.from = None;
                command.to = None;
                command.incoming = false;
                command.outgoing = false;
                command.limit = 10;
                println!("✓ All filters cleared");
            }
            "Filter by date range" => {
                let from = Text::new("Start date (YYYY-MM-DD, leave empty for no start date):")
                    .prompt_skippable()?;
                let to = Text::new("End date (YYYY-MM-DD, leave empty for today):")
                    .prompt_skippable()?;

                command.from = from.and_then(|s| if s.is_empty() { None } else { Some(s) });
                command.to = to.and_then(|s| if s.is_empty() { None } else { Some(s) });
            }
            "Back to main menu" => break,
            _ => {}
        }
    }

    Ok(())
}
