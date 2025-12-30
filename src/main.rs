#![allow(warnings)]
use anyhow::{Result, anyhow};
use dotenv::dotenv;
use std::env;

mod api;
mod commands;
mod config;
mod interactive;
mod setup;
mod types;
mod utils;
mod zk;

use clap::Parser;
use commands::root::Commands;
use commands::zk::handle_zk_command;

#[derive(Parser)]
#[command(name = "rootstock-wallet")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Load environment variables from .env file if it exists
    dotenv().ok();

    // Ensure wallet is configured
    if let Err(e) = setup::ensure_configured().await {
        eprintln!("Failed to configure wallet: {}", e);
        std::process::exit(1);
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Zk(args)) => {
            handle_zk_command(args).await?;
        }
        Some(_cmd) => {
            eprintln!(
                "Other commands are currently only supported in interactive mode. Please run without arguments to start interactive mode."
            );
        }
        None => {
            // Start the interactive interface
            interactive::start().await?;
        }
    }

    Ok(())
}
