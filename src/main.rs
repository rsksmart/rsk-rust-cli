// Removed blanket allow(warnings) to improve code quality and catch diagnostics.
use anyhow::Result;
use dotenvy::dotenv;

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
    env_logger::init();
    dotenv().ok();

    let cli = Cli::parse();

    // ZK proof commands are purely computational and do not require a configured wallet.
    // Only run the setup wizard for non-ZK commands.
    let is_zk_command = matches!(cli.command, Some(Commands::Zk(_)));
    if !is_zk_command {
        if let Err(e) = setup::ensure_configured().await {
            eprintln!("Failed to configure wallet: {}", e);
            std::process::exit(1);
        }
    }

    match cli.command {
        Some(Commands::Zk(args)) => {
            handle_zk_command(args).await?;
        }
        Some(Commands::Config(cmd)) => {
            cmd.execute().await?;
        }
        Some(_cmd) => {
            eprintln!(
                "Other commands are currently only supported in interactive mode. \
                 Please run without arguments to start interactive mode."
            );
        }
        None => {
            interactive::start().await?;
        }
    }

    Ok(())
}
