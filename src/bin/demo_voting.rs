use alloy::network::EthereumWallet;
use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use anyhow::Result;
use rootstock_wallet::zk::{types::VoteInputs, zk_generate_proof, CircuitType};
use std::env;
use std::str::FromStr;

// Bind to the deployed PrivateVoting contract ABI.
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

    // 1. Setup wallet from environment — no insecure fallback.
    //    PRIVATE_KEY must be a 32-byte hex string (without 0x prefix).
    let pk_str = env::var("PRIVATE_KEY").map_err(|_| {
        anyhow::anyhow!(
            "PRIVATE_KEY environment variable is required.\n\
             Set it to your wallet private key (hex, no 0x prefix):\n\
             \n\
             export PRIVATE_KEY=<your-private-key-hex>"
        )
    })?;
    let signer = PrivateKeySigner::from_str(&pk_str)?;
    let wallet = EthereumWallet::from(signer.clone());

    // 2. Connect to RSK Testnet.
    let rpc_url = "https://public-node.testnet.rsk.co".parse()?;
    let provider = ProviderBuilder::new().wallet(wallet).on_http(rpc_url);

    // 3. Contract address — must be deployed with MockVerifier or a real verifier.
    //    Deploy PrivateVoting.sol + MockVerifier.sol first using Foundry or Hardhat.
    let contract_addr_str = env::var("CONTRACT_ADDRESS").map_err(|_| {
        anyhow::anyhow!(
            "CONTRACT_ADDRESS environment variable is required.\n\
             Deploy PrivateVoting.sol (with MockVerifier) and set:\n\
             \n\
             export CONTRACT_ADDRESS=<deployed-contract-address>"
        )
    })?;
    let contract_addr = Address::from_str(&contract_addr_str)?;
    let contract = PrivateVoting::new(contract_addr, provider.clone());

    // 4. Generate ZK proof for a private vote.
    println!("Generating ZK proof for vote...");
    let vote_input = VoteInputs {
        voter_id: 123,
        vote_choice: 1, // 1 = Yes
        secret: 9999,   // Private; determines nullifier — change per voter
    };
    let input_bytes = serde_json::to_vec(&vote_input)?;
    let (receipt, journal) = zk_generate_proof(CircuitType::Vote, &input_bytes)?;

    println!("Proof generated.");
    println!("Journal ({} bytes): {}", journal.len(), hex::encode(&journal));

    // Serialize the full receipt as the seal.
    //
    // NOTE: For production on-chain deployment, use Risc0's Groth16 prover to generate
    // a compact, verifiable seal that the RiscZeroGroth16Verifier contract can check:
    //   rzup install groth16
    //   RISC0_PROVER=bonsai cargo run --features zk --bin demo_voting
    //
    // For this demo, the seal is the full bincode-serialized receipt, accepted by
    // MockVerifier (which skips actual cryptographic verification).
    let seal_bytes = bincode::serialize(&receipt)?;

    println!("Sending transaction to contract {}...", contract_addr_str);

    let gas_price = provider.get_gas_price().await?;
    let nonce = provider.get_transaction_count(signer.address()).await?;

    let tx_builder = contract
        .castVote(journal.into(), seal_bytes.into())
        .nonce(nonce)
        .gas(1_000_000)
        .gas_price(gas_price);

    // Simulate before broadcasting to catch double-vote reverts and other errors.
    match tx_builder.call().await {
        Ok(_) => println!("Simulation successful, broadcasting transaction..."),
        Err(e) => {
            return Err(anyhow::anyhow!("Transaction simulation failed: {}", e));
        }
    }

    let tx_hash = tx_builder.send().await?.watch().await?;
    println!("Vote cast! Transaction hash: {:?}", tx_hash);

    Ok(())
}
