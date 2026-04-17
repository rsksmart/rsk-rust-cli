use alloy::network::EthereumWallet;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use anyhow::Result;
use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::{AffineRepr, CurveGroup, Group};
use ark_ff::{BigInteger, PrimeField};
use rand::RngCore;
use rootstock_wallet::zk::{types::VoteInputs, zk_generate_proof, CircuitType};
use sha2::{Digest, Sha256};
use std::env;
use std::ops::Mul;
use std::str::FromStr;
use zeroize::Zeroizing;

// Bind to the PrivateVoting contract ABI (full lifecycle + castVote).
sol! {
    #[sol(rpc)]
    contract PrivateVoting {
        function createProposal(bytes32 merkleRoot, uint256 deadline) external returns (uint256);
        function openProposal(uint256 proposalId) external;
        function castVote(
            uint256 proposalId,
            bytes calldata journal,
            bytes calldata seal,
            uint256 c1x,
            uint256 c1y,
            uint256 c2x,
            uint256 c2y
        ) external;
        function getProposalState(uint256 proposalId) external view returns (uint8);
    }
}

// ---------------------------------------------------------------------------
// Merkle helpers (must match the guest circuit algorithm exactly)
// ---------------------------------------------------------------------------

/// SHA-256("voter-leaf-v1" || voter_id_be32)
fn voter_leaf_hash(voter_id: u32) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"voter-leaf-v1");
    h.update(voter_id.to_be_bytes());
    h.finalize().into()
}

// ---------------------------------------------------------------------------
// ElGamal (host-side, BN254 native Rust)
// ---------------------------------------------------------------------------

/// Compute ElGamal encryption on BN254.
///   C1 = g^r
///   C2 = Y^r + g^v   (v=0 No, v=1 Yes; additively homomorphic)
///
/// Returns Err if the public key is not a valid BN254 G1 point.
fn host_elgamal(
    vote_choice: u8,
    randomness: &[u8; 32],
    pk_x: &[u8; 32],
    pk_y: &[u8; 32],
) -> anyhow::Result<(U256, U256, U256, U256)> {
    let r = Fr::from_be_bytes_mod_order(randomness);
    let g = G1Affine::generator();
    let c1 = g.mul(r).into_affine();

    let pk_x_fq = ark_bn254::Fq::from_be_bytes_mod_order(pk_x);
    let pk_y_fq = ark_bn254::Fq::from_be_bytes_mod_order(pk_y);
    let pk = G1Affine::new_unchecked(pk_x_fq, pk_y_fq);
    if !pk.is_on_curve() {
        return Err(anyhow::anyhow!("Election public key is not on the BN254 curve"));
    }
    if !pk.is_in_correct_subgroup_assuming_on_curve() {
        return Err(anyhow::anyhow!(
            "Election public key is not in the correct BN254 subgroup (small-subgroup attack)"
        ));
    }

    let yr = pk.into_group().mul(r);
    let gv = if vote_choice == 0 {
        G1Projective::default()
    } else {
        G1Projective::generator()
    };
    let c2 = (yr + gv).into_affine();

    Ok((
        U256::from_be_slice(&c1.x.into_bigint().to_bytes_be()),
        U256::from_be_slice(&c1.y.into_bigint().to_bytes_be()),
        U256::from_be_slice(&c2.x.into_bigint().to_bytes_be()),
        U256::from_be_slice(&c2.y.into_bigint().to_bytes_be()),
    ))
}

// ---------------------------------------------------------------------------
// Seal extraction (Groth16 compact vs STARK bincode)
// ---------------------------------------------------------------------------

/// Extract the on-chain seal bytes from a receipt.
///
/// - Groth16 receipt → compact ~256-byte seal, compatible with RiscZeroGroth16Verifier.
/// - STARK/other    → bincode-serialized full receipt (MB-sized); only MockVerifier
///                    will accept it. Prints a size warning.
fn extract_seal(receipt: &risc0_zkvm::Receipt) -> anyhow::Result<Vec<u8>> {
    use risc0_zkvm::InnerReceipt;
    match &receipt.inner {
        InnerReceipt::Groth16(g16) => {
            println!(
                "Groth16 seal: {} bytes — compatible with RiscZeroGroth16Verifier.",
                g16.seal.len()
            );
            Ok(g16.seal.clone())
        }
        _ => {
            let stark_bytes = bincode::serialize(receipt)?;
            eprintln!();
            eprintln!(
                "WARNING: STARK receipt ({} bytes). Only MockVerifier will accept this.",
                stark_bytes.len()
            );
            if stark_bytes.len() > 128 * 1024 {
                eprintln!(
                    "WARNING: Seal exceeds RSK's ~128 KB transaction limit ({} KB). \
                     The on-chain transaction will be rejected by RSK nodes.",
                    stark_bytes.len() / 1024
                );
            }
            eprintln!(
                "  For production: set RISC0_PROVER=groth16  \
                 (requires: rzup install groth16)"
            );
            eprintln!();
            Ok(stark_bytes)
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    // [AUDIT-FIX] Fixed crate name: dotenv -> dotenvy
    dotenvy::dotenv().ok();

    // 1. Wallet setup.
    // [AUDIT-FIX] Zeroize sensitive private key material.
    let pk_env = env::var("PRIVATE_KEY").map_err(|_| {
        anyhow::anyhow!("PRIVATE_KEY environment variable is required.")
    })?;
    let pk_trimmed = pk_env.trim().trim_start_matches("0x");
    let pk_str = Zeroizing::new(pk_trimmed.to_string());
    let signer = PrivateKeySigner::from_str(&pk_str)?;
    let wallet = EthereumWallet::from(signer.clone());

    // 2. Connect to RSK Testnet.
    let rpc_url = "https://mycrypto.testnet.rsk.co".parse()?;
    let provider = ProviderBuilder::new().wallet(wallet).on_http(rpc_url);
    println!("Active wallet address: {}", signer.address());

    // 3. Contract address.
    let contract_addr_str = env::var("CONTRACT_ADDRESS").map_err(|_| {
        anyhow::anyhow!(
            "CONTRACT_ADDRESS environment variable is required.\n\
             Deploy PrivateVoting.sol and set:\n\
             export CONTRACT_ADDRESS=<deployed-contract-address>"
        )
    })?;
    let contract_addr = Address::from_str(&contract_addr_str)?;
    let contract = PrivateVoting::new(contract_addr, provider.clone());

    // 4. Prepare voter.
    let voter_id: u32 = 12345;
    let vote_choice: u8 = 1; // Yes

    // Depth-0 Merkle tree: single eligible voter; root == leaf hash.
    // In production, use a pre-built multi-voter tree and supply the full path.
    //
    // DEMO WARNING: This creates a single-voter allowlist with voter_id=12345.
    // The admin (deployer wallet) registers this root on-chain via createProposal.
    let merkle_root = voter_leaf_hash(voter_id);
    println!(
        "Merkle root (single-voter, depth 0): {}",
        hex::encode(merkle_root)
    );

    // 5. Create and open the proposal (demo wallet is the contract admin).
    //    Set PROPOSAL_ID to skip this step and use an existing open proposal.
    let proposal_id: u64 = if let Ok(id_str) = env::var("PROPOSAL_ID") {
        let id: u64 = id_str
            .parse()
            .map_err(|_| anyhow::anyhow!("PROPOSAL_ID must be a non-negative integer"))?;
        println!("Using existing proposal ID: {}", id);
        id
    } else {
        println!("Creating proposal with voter Merkle root...");

        let gas_price = provider.get_gas_price().await?;
        let nonce = provider.get_transaction_count(signer.address()).await?;

        // Get the proposal ID from a dry-run before broadcasting.
        let new_id = contract
            .createProposal(merkle_root.into(), U256::ZERO)
            .from(signer.address())
            .call()
            .await?
            ._0
            .to::<u64>();

        contract
            .createProposal(merkle_root.into(), U256::ZERO)
            .from(signer.address())
            .nonce(nonce)
            .gas_price(gas_price)
            .gas(200_000)
            .send()
            .await?
            .watch()
            .await?;
        println!("Proposal {} created.", new_id);

        let nonce = provider.get_transaction_count(signer.address()).await?;
        contract
            .openProposal(U256::from(new_id))
            .from(signer.address())
            .nonce(nonce)
            .gas_price(gas_price)
            .gas(100_000)
            .send()
            .await?
            .watch()
            .await?;
        println!("Proposal {} opened.", new_id);

        new_id
    };

    // 6. ZK proof inputs.
    println!("\nPreparing ZK inputs for private vote...");

    let mut secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret);

    let mut randomness = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut randomness);

    let vote_input = VoteInputs {
        voter_id,
        vote_choice,
        secret,
        merkle_root,
        merkle_siblings: vec![], // depth 0: no siblings needed
        merkle_path_index: 0,
    };

    // 7. Generate ZK proof (SHA-256 + Merkle, completes in seconds with RISC0_DEV_MODE).
    println!("Generating ZK proof (vote validity + Merkle membership)...");
    let input_bytes = serde_json::to_vec(&vote_input)?;
    let (receipt, journal) = zk_generate_proof(CircuitType::Vote, &input_bytes)?;
    println!(
        "Proof generated. Journal (64 bytes): {}",
        hex::encode(&journal)
    );
    println!("  merkle_root : {}", hex::encode(&journal[0..32]));
    println!("  nullifier   : {}", hex::encode(&journal[32..64]));

    // 8. Compute ElGamal ciphertext on the HOST (native Rust — milliseconds).
    //
    // DEMO WARNING: public key = BN254 G1 generator (private key = 1, tally is
    // trivially decryptable). In production, use a key from a distributed key
    // generation ceremony.
    println!("\nComputing ElGamal encryption (host-side BN254)...");
    let mut pub_key_x = [0u8; 32];
    pub_key_x[31] = 1;
    let mut pub_key_y = [0u8; 32];
    pub_key_y[31] = 2;
    let (c1x, c1y, c2x, c2y) =
        host_elgamal(vote_choice, &randomness, &pub_key_x, &pub_key_y)?;
    println!("ElGamal ciphertext computed.");

    // 9. Extract seal (Groth16 compact if available, STARK bincode otherwise).
    let seal_bytes = extract_seal(&receipt)?;

    // 10. Build the castVote transaction and estimate gas dynamically.
    println!(
        "\nSending castVote to proposal {} on contract {}...",
        proposal_id, contract_addr_str
    );

    let gas_price = provider.get_gas_price().await?;
    let nonce = provider.get_transaction_count(signer.address()).await?;

    let call = contract.castVote(
        U256::from(proposal_id),
        journal.clone().into(),
        seal_bytes.clone().into(),
        c1x,
        c1y,
        c2x,
        c2y,
    );

    let gas_limit = match call.estimate_gas().await {
        Ok(estimate) => {
            // 20% buffer over the estimate.
            let gas = estimate * 12 / 10;
            println!("Gas estimate: {} (using {} with 20% buffer)", estimate, gas);
            gas
        }
        Err(e) => {
            eprintln!("Gas estimation failed: {}. Using fallback of 3_000_000.", e);
            3_000_000u64
        }
    };

    // Simulate before broadcasting.
    let tx_builder = contract
        .castVote(
            U256::from(proposal_id),
            journal.into(),
            seal_bytes.into(),
            c1x,
            c1y,
            c2x,
            c2y,
        )
        .nonce(nonce)
        .gas(gas_limit)
        .gas_price(gas_price);

    match tx_builder.call().await {
        Ok(_) => println!("Simulation successful, broadcasting transaction..."),
        Err(e) => return Err(anyhow::anyhow!("Transaction simulation failed: {}", e)),
    }

    let tx_hash = tx_builder.send().await?.watch().await?;
    println!("\nVote cast successfully! Homomorphic tally updated.");
    println!("Transaction hash: {:?}", tx_hash);

    Ok(())
}
