use crate::zk::types::CircuitType;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[cfg(feature = "zk")]
use crate::zk::types::{TransferInputs, VoteInputs};
#[cfg(feature = "zk")]
use crate::zk::{zk_generate_proof, zk_verify_proof, TRANSFER_ID, VOTE_ID};

#[derive(Parser, Debug)]
pub struct ZkArgs {
    #[clap(subcommand)]
    pub command: ZkCommands,
}

#[derive(Subcommand, Debug)]
pub enum ZkCommands {
    /// Generate a ZK proof and save it to a file
    Generate {
        /// Circuit type (auto-completed by shell; invalid values are rejected with help text)
        #[clap(long, value_enum)]
        circuit: CircuitType,

        /// Path to JSON input file (must match the circuit's input schema)
        #[clap(long)]
        input: PathBuf,

        /// Path to save the output proof (.bin)
        #[clap(long)]
        output: PathBuf,
    },

    /// Verify a previously generated ZK proof
    Verify {
        /// Circuit type
        #[clap(long, value_enum)]
        circuit: CircuitType,

        /// Path to proof file (.bin)
        #[clap(long)]
        proof: PathBuf,

        /// Optional path to a JSON file containing expected public outputs.
        /// When provided, the decoded journal is compared against these values
        /// and an error is returned if they do not match.
        #[clap(long)]
        public_input: Option<PathBuf>,
    },

    /// Submit a ZK proof to an on-chain verifier contract (not yet implemented)
    ///
    /// This command will submit ZK proofs to the RSK PowPeg cross-chain bridge verifier,
    /// enabling trustless proof of peg-in/peg-out transactions. Full implementation requires
    /// the RiscZeroGroth16Verifier contract deployed on RSK and Groth16 proof generation.
    Bridge {
        /// Circuit type
        #[clap(long, value_enum)]
        circuit: CircuitType,

        /// Path to proof file (.bin)
        #[clap(long)]
        proof: PathBuf,

        /// On-chain verifier contract address (checksummed hex)
        #[clap(long)]
        contract: String,
    },
}

pub async fn handle_zk_command(args: ZkArgs) -> anyhow::Result<()> {
    #[cfg(not(feature = "zk"))]
    {
        eprintln!(
            "ZK features are disabled in this build.\n\
             Rebuild with the 'zk' feature flag enabled:\n\
             \n\
             cargo build --features zk\n\
             cargo run --features zk -- zk <subcommand>"
        );
        return Ok(());
    }

    #[cfg(feature = "zk")]
    handle_zk_command_inner(args).await
}

#[cfg(feature = "zk")]
async fn handle_zk_command_inner(args: ZkArgs) -> anyhow::Result<()> {
    match args.command {
        ZkCommands::Generate {
            circuit,
            input,
            output,
        } => {
            println!("Generating ZK proof for circuit: {:?}", circuit);

            let input_data = fs::read_to_string(&input)
                .map_err(|e| anyhow::anyhow!("Failed to read input file {:?}: {}", input, e))?;

            let input_bytes = match circuit {
                CircuitType::Transfer => {
                    let inputs: TransferInputs = serde_json::from_str(&input_data)
                        .map_err(|e| anyhow::anyhow!("Invalid transfer input JSON: {}", e))?;
                    serde_json::to_vec(&inputs)?
                }
                CircuitType::Vote => {
                    let inputs: VoteInputs = serde_json::from_str(&input_data)
                        .map_err(|e| anyhow::anyhow!("Invalid vote input JSON: {}", e))?;
                    serde_json::to_vec(&inputs)?
                }
            };

            // zk_generate_proof performs pre-flight validation before invoking the prover.
            let (receipt, journal) = zk_generate_proof(circuit, &input_bytes)?;

            let receipt_bytes = bincode::serialize(&receipt)?;
            fs::write(&output, &receipt_bytes)
                .map_err(|e| anyhow::anyhow!("Failed to write proof to {:?}: {}", output, e))?;

            println!("Proof generated and saved to {:?}", output);
            println!("Journal ({} bytes): {}", journal.len(), hex::encode(&journal));
        }

        ZkCommands::Verify {
            circuit,
            proof,
            public_input,
        } => {
            let image_id = match circuit {
                CircuitType::Transfer => TRANSFER_ID,
                CircuitType::Vote => VOTE_ID,
            };

            let proof_bytes = fs::read(&proof)
                .map_err(|e| anyhow::anyhow!("Failed to read proof file {:?}: {}", proof, e))?;
            let receipt: risc0_zkvm::Receipt = bincode::deserialize(&proof_bytes)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize proof: {}", e))?;

            zk_verify_proof(&receipt, image_id)?;
            println!("Proof cryptographic verification: SUCCESS");

            // Optional: compare journal against expected public outputs.
            if let Some(expected_path) = public_input {
                let journal = &receipt.journal.bytes;
                verify_public_inputs(circuit, journal, &expected_path)?;
                println!("Public input check: MATCH");
            }
        }

        ZkCommands::Bridge {
            circuit,
            proof: _,
            contract,
        } => {
            println!("zk-bridge: on-chain proof submission (not yet implemented)");
            println!();
            println!("Circuit : {:?}", circuit);
            println!("Contract: {}", contract);
            println!();
            println!(
                "This command will submit ZK proofs to the RSK PowPeg verifier contract,\n\
                 enabling trustless proof of cross-chain peg-in/peg-out transactions.\n\
                 \n\
                 Full implementation requires:\n\
                 1. Deploy PrivateVoting.sol (or a bridge-specific contract) on RSK.\n\
                 2. Use Groth16 proof wrapping (rzup install groth16) for compact on-chain proofs.\n\
                 3. Fund the signing wallet and set PRIVATE_KEY + CONTRACT_ADDRESS env vars.\n\
                 \n\
                 For the voting demo, use src/bin/demo_voting.rs instead."
            );
        }
    }
    Ok(())
}

/// Decode the journal and compare it against user-supplied expected values.
#[cfg(feature = "zk")]
fn verify_public_inputs(
    circuit: CircuitType,
    journal: &[u8],
    expected_path: &PathBuf,
) -> anyhow::Result<()> {
    let expected_json = fs::read_to_string(expected_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read expected public input file {:?}: {}",
            expected_path,
            e
        )
    })?;

    match circuit {
        CircuitType::Vote => {
            if journal.len() != 64 {
                return Err(anyhow::anyhow!(
                    "Vote journal must be 64 bytes, got {}",
                    journal.len()
                ));
            }
            // Journal layout: [vote_hash: bytes32][nullifier: bytes32]
            let vote_choice = journal[31]; // last byte of voteHash word
            let nullifier_hex = hex::encode(&journal[32..64]);

            let expected: serde_json::Value = serde_json::from_str(&expected_json)
                .map_err(|e| anyhow::anyhow!("Invalid expected JSON: {}", e))?;

            if let Some(expected_choice) = expected.get("vote_choice").and_then(|v| v.as_u64()) {
                if expected_choice as u8 != vote_choice {
                    return Err(anyhow::anyhow!(
                        "vote_choice mismatch: expected {}, journal has {}",
                        expected_choice,
                        vote_choice
                    ));
                }
            }
            if let Some(expected_nullifier) = expected.get("nullifier").and_then(|v| v.as_str()) {
                let normalized = expected_nullifier.trim_start_matches("0x");
                if normalized != nullifier_hex {
                    return Err(anyhow::anyhow!(
                        "nullifier mismatch: expected {}, journal has {}",
                        expected_nullifier,
                        nullifier_hex
                    ));
                }
            }
        }
        CircuitType::Transfer => {
            if journal.len() != 64 {
                return Err(anyhow::anyhow!(
                    "Transfer journal must be 64 bytes, got {}",
                    journal.len()
                ));
            }
            // Journal layout: [amount: u64 right-aligned in bytes32][commitment: bytes32]
            let mut amount_bytes = [0u8; 8];
            amount_bytes.copy_from_slice(&journal[24..32]);
            let amount = u64::from_be_bytes(amount_bytes);
            let commitment_hex = hex::encode(&journal[32..64]);

            let expected: serde_json::Value = serde_json::from_str(&expected_json)
                .map_err(|e| anyhow::anyhow!("Invalid expected JSON: {}", e))?;

            if let Some(expected_amount) = expected.get("amount").and_then(|v| v.as_u64()) {
                if expected_amount != amount {
                    return Err(anyhow::anyhow!(
                        "amount mismatch: expected {}, journal has {}",
                        expected_amount,
                        amount
                    ));
                }
            }
            if let Some(expected_commitment) =
                expected.get("commitment").and_then(|v| v.as_str())
            {
                let normalized = expected_commitment.trim_start_matches("0x");
                if normalized != commitment_hex {
                    return Err(anyhow::anyhow!(
                        "commitment mismatch: expected {}, journal has {}",
                        expected_commitment,
                        commitment_hex
                    ));
                }
            }
        }
    }
    Ok(())
}
