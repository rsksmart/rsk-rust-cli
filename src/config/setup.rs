use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Password, Select, theme::ColorfulTheme};

use crate::config::{Config, ConfigManager, DOCS_URL, RSK_RPC_DOCS_URL};
use crate::types::network::Network;
use crate::utils::secrets::SecretString;

pub fn run_setup_wizard() -> Result<()> {
    println!(
        "\n{}",
        style("🌟 Welcome to Rootstock Wallet CLI!").bold().cyan()
    );
    println!("{}", "=".repeat(40));
    println!("\nLet's get you set up with the basic configuration.\n");

    let config_manager = ConfigManager::new()?;
    let mut config = config_manager.load()?;

    // Network selection
    let networks = &[
        "Testnet (recommended for testing)",
        "Mainnet (for real funds)",
        "Regtest (local development)",
        "Alchemy Mainnet",
        "Alchemy Testnet",
        "Rootstock Mainnet",
        "Rootstock Testnet",
    ];

    let network_variants = &[
        Network::Testnet,
        Network::Mainnet,
        Network::Regtest,
        Network::AlchemyMainnet,
        Network::AlchemyTestnet,
        Network::RootStockMainnet,
        Network::RootStockTestnet,
    ];

    let network_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your default network")
        .default(0)
        .items(networks)
        .interact()?;

    let network = network_variants[network_idx];
    config.default_network = network;

    // API Key setup
    setup_api_keys(&mut config, network)?;

    // Save configuration
    config_manager.save(&config)?;

    println!("\n{}", style("✅ Setup complete!").bold().green());
    println!("\nYou can now use the wallet. For more information, visit:");
    println!("{}", style(DOCS_URL).blue().underlined());
    println!("\nRun `rsk-rust-cli --help` to see available commands.");

    Ok(())
}

fn setup_api_keys(config: &mut Config, network: Network) -> Result<()> {
    println!("\n{}", style("🔑 API Key Setup (Optional)").bold().cyan());
    println!("{}", "=".repeat(40));

    let key_type = match network {
        Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => "mainnet",
        Network::Testnet
        | Network::AlchemyTestnet
        | Network::RootStockTestnet
        | Network::Regtest => "testnet",
    };

    println!(
        "\n{}",
        style("The wallet works with public RSK nodes by default.").green()
    );
    println!("You can optionally configure API keys for enhanced performance and features:\n");

    println!(
        "• {}: Better rate limits and performance",
        style("RSK RPC API").bold()
    );
    println!(
        "• {}: Transaction history and advanced queries",
        style("Alchemy API").bold()
    );

    // Optional RSK RPC API key setup
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Would you like to set up RSK RPC API key for {} (recommended)?",
            key_type
        ))
        .default(false)
        .interact()?
    {
        println!("\nGet your RSK RPC API key from:");
        println!("{}", style(RSK_RPC_DOCS_URL).blue().underlined());

        let rsk_key: String = Password::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Enter your RSK RPC {} API key", key_type))
            .interact()?;

        // Add RSK RPC API key to config
        use crate::api::{ApiKey, ApiProvider};
        let rsk_api_key = ApiKey {
            key: SecretString::new(rsk_key),
            network: key_type.to_string(),
            provider: ApiProvider::RskRpc,
            name: Some("RSK RPC".to_string()),
        };
        config.api.keys.push(rsk_api_key);
    }

    // Optional Alchemy API key setup
    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Would you like to set up Alchemy API key for {} (for transaction history)?",
            key_type
        ))
        .default(false)
        .interact()?
    {
        println!("\nAlchemy provides transaction history and advanced query features.");
        println!("Get your Alchemy API key from:");
        let alchemy_url = match network {
            Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => {
                "https://dashboard.alchemy.com/apps/create?referrer=/apps"
            }
            _ => "https://dashboard.alchemy.com/apps/create?referrer=/apps&chain=rsk-testnet",
        };
        println!("{}", style(alchemy_url).blue().underlined());

        let alchemy_key: String = Password::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Enter your Alchemy {} API key", key_type))
            .interact()?;

        // Add Alchemy API key to config
        use crate::api::{ApiKey, ApiProvider};
        let alchemy_api_key = ApiKey {
            key: SecretString::new(alchemy_key.clone()),
            network: key_type.to_string(),
            provider: ApiProvider::Alchemy,
            name: Some("Alchemy".to_string()),
        };
        config.api.keys.push(alchemy_api_key);

        // Also set legacy fields for backward compatibility
        match network {
            Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => {
                config.alchemy_mainnet_key = Some(alchemy_key)
            }
            _ => config.alchemy_testnet_key = Some(alchemy_key),
        }
    }

    // Ask if they want to set up the other network type too
    let other_network = match network {
        Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => {
            println!("\nWould you like to set up testnet API keys as well?");
            Network::Testnet
        }
        Network::Testnet
        | Network::AlchemyTestnet
        | Network::RootStockTestnet
        | Network::Regtest => {
            println!("\nWould you like to set up mainnet API keys as well?");
            Network::Mainnet
        }
    };

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(format!(
            "Set up {} API keys now?",
            match other_network {
                Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => "mainnet",
                _ => "testnet",
            }
        ))
        .default(false)
        .interact()?
    {
        setup_api_keys(config, other_network)?;
    }

    Ok(())
}
