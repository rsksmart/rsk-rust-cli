use anyhow::Result;
use console::style;

use crate::config::{Config, ConfigManager};
use crate::types::network::Network;

pub fn run_doctor() -> Result<()> {
    println!("\n{}", style("🩺 Running diagnostics...").bold().cyan());
    println!("{}", "=".repeat(40));

    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;

    // Check config file
    println!("\n{}", style("🔍 Configuration:").bold());
    println!("  Config file: {}", config_manager.config_path().display());

    if !config_manager.config_path().exists() {
        println!("  ❌ Configuration file not found");
        println!("     Run `setup` to create a new configuration");
        return Ok(());
    }

    // Check network configuration
    println!("\n{}", style("🌐 Network Configuration:").bold());
    println!("  Default network: {}", config.default_network);

    // Check API keys
    println!("\n{}", style("🔑 API Keys:").bold());

    // Check mainnet API keys
    check_api_key(&config, Network::Mainnet);
    check_api_key(&config, Network::AlchemyMainnet);
    check_api_key(&config, Network::RootStockMainnet);

    // Check testnet API keys
    check_api_key(&config, Network::Testnet);
    check_api_key(&config, Network::AlchemyTestnet);
    check_api_key(&config, Network::RootStockTestnet);
    check_api_key(&config, Network::Regtest);

    // Check wallet configuration
    println!("\n{}", style("💼 Wallet Configuration:").bold());
    if let Some(wallet_name) = &config.default_wallet {
        println!("  Default wallet: {}", wallet_name);
        
        // [AUDIT-FIX] Added wallet existence check
        let wallet_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".local/share/rsk-rust-cli/wallets")
            .join(format!("{}.json", wallet_name));
            
        if wallet_path.exists() {
            println!("  {} Wallet file found", style("✓").green());
        } else {
            println!("  {} Wallet file NOT found at {}", style("✗").red(), wallet_path.display());
            println!("     Run `wallet list` to see available wallets");
        }
    } else {
        println!("  ℹ️ No default wallet set");
        println!("     Run `wallet create` to create a new wallet");
    }

    println!("\n{}", style("✅ Diagnostics complete").bold().green());
    Ok(())
}

fn check_api_key(config: &Config, network: Network) {
    let key = match network {
        Network::Mainnet | Network::AlchemyMainnet | Network::RootStockMainnet => {
            &config.alchemy_mainnet_key
        }
        Network::Testnet
        | Network::AlchemyTestnet
        | Network::RootStockTestnet
        | Network::Regtest => &config.alchemy_testnet_key,
    };

    let status = match key {
        Some(_) => style("✓ Configured").green(),
        None => style("✗ Missing").red(),
    };

    println!(
        "  {} API key: {}",
        match network {
            Network::Mainnet => "Mainnet",
            Network::Testnet => "Testnet",
            Network::Regtest => "Regtest",
            Network::AlchemyMainnet => "Alchemy Mainnet",
            Network::AlchemyTestnet => "Alchemy Testnet",
            Network::RootStockMainnet => "Rootstock Mainnet",
            Network::RootStockTestnet => "Rootstock Testnet",
        },
        status
    );
}
