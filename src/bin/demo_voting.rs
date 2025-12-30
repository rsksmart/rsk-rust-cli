use alloy::network::EthereumWallet;
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::Signer;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use anyhow::Result;
use rootstock_wallet::zk::{CircuitType, types::VoteInputs, zk_generate_proof};
use std::env;
use std::str::FromStr;

// Define the contract interface
sol! {
    #[sol(rpc)]
    contract PrivateVoting {
        function castVote(bytes calldata journal, bytes calldata seal) external;
        function merkleRoot() external view returns (bytes32);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // 1. Setup Wallet & Provider
    // In a real app, use the wallet management from `rootstock-wallet` crate.
    // Here we just grab a private key from env for the demo.
    let pk_str = env::var("PRIVATE_KEY").unwrap_or_else(|_| {
        "0000000000000000000000000000000000000000000000000000000000000001".to_string()
    });
    let signer = PrivateKeySigner::from_str(&pk_str)?;
    let wallet = EthereumWallet::from(signer.clone());

    let rpc_url = "https://public-node.testnet.rsk.co".parse()?; // RSK Testnet
    let provider = ProviderBuilder::new()
        .wallet(wallet.clone()) // Clone wallet as we need address later
        .on_http(rpc_url);

    // 2. Contract Address (Mock or Env)
    let contract_addr_str = env::var("CONTRACT_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
    let contract_addr = Address::from_str(&contract_addr_str)?;
    let contract = PrivateVoting::new(contract_addr, provider.clone());

    // 3. Prepare ZK Vote
    println!("Generating ZK Proof for Vote...");
    let vote_input = VoteInputs {
        voter_id: 123,
        vote_choice: 1, // Yes
        secret: 9999,   // Top secret
    };

    // Serialize input
    let input_bytes = serde_json::to_vec(&vote_input)?;

    // Generate Proof
    let (receipt, journal) = zk_generate_proof(CircuitType::Vote, &input_bytes)?;

    // NOTE: Sending dummy seal bytes to keep transaction size under RSK limits for this demo.
    // Real deployments should use Groth16 wrapper for compact ZK-SNARK proofs.
    let seal_bytes = vec![0u8; 4];

    println!("Proof Generated! Sending Transaction...");
    println!("Journal ({} bytes): {:?}", journal.len(), journal);

    // 4. Send Transaction (Manual gas for RSK compatibility)
    // fetch gas price first
    let gas_price = provider.get_gas_price().await?;
    let nonce = provider.get_transaction_count(signer.address()).await?;

    let tx_builder = contract
        .castVote(journal.into(), seal_bytes.into())
        .nonce(nonce)
        .gas(1_000_000) // Conservative limit for demo
        .gas_price(gas_price);

    // Simulate first to check for reverts (e.g. Double Voting)
    match tx_builder.call().await {
        Ok(_) => println!("Simulation successful, sending transaction..."),
        Err(e) => {
            println!("Transaction Failed (Simulation): {}", e);
            // Optional: Try to parse specifically if it's a revert
            return Err(anyhow::anyhow!("Transaction would revert: {}", e));
        }
    }

    let tx_hash = tx_builder.send().await?.watch().await?;

    println!("Vote Cast! Tx Hash: {:?}", tx_hash);

    Ok(())
}
