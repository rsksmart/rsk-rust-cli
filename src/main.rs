#![allow(warnings)]
use anyhow::{Result, anyhow};
use dotenvy::dotenv;
use std::env;

mod api;
mod commands;
mod config;
mod interactive;
mod setup;
mod types;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // Check if any command line arguments were provided
    if env::args().count() > 1 {
        eprintln!("This program only runs in interactive mode. Please run without any arguments.");
        eprintln!("Usage: cargo run");
        std::process::exit(1);
    }

    // Initialize logging
    env_logger::init();

    // Load environment variables from .env file if it exists
    dotenv().ok();

    // Ensure wallet is configured
    if let Err(e) = setup::ensure_configured().await {
        eprintln!("Failed to configure wallet: {}", e);
        std::process::exit(1);
    }

    // Start the interactive interface
    interactive::start().await?;

    Ok(())
}
