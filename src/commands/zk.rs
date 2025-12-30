use clap::{Subcommand, Parser};
use crate::zk::{zk_generate_proof, zk_verify_proof};
use crate::zk::types::{TransferInputs, VoteInputs, CircuitType as ZkCircuitType};
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
pub struct ZkArgs {
    #[clap(subcommand)]
    pub command: ZkCommands,
}

#[derive(Subcommand, Debug)]
pub enum ZkCommands {
    /// Generate a ZK proof
    Generate {
        /// Circuit type: 'transfer' or 'vote'
        #[clap(long)]
        circuit: String,

        /// Path to JSON input file
        #[clap(long)]
        input: PathBuf,

        /// Path to save the output proof
        #[clap(long)]
        output: PathBuf,
    },
    /// Verify a ZK proof
    Verify {
        /// Circuit type: 'transfer' or 'vote'
        #[clap(long)]
        circuit: String,

        /// Path to proof file
        #[clap(long)]
        proof: PathBuf,

        /// Path to public inputs JSON (optional, often part of proof/journal)
        #[clap(long)]
        public_input: Option<PathBuf>,
    },
}

pub async fn handle_zk_command(args: ZkArgs) -> anyhow::Result<()> {
    match args.command {
        ZkCommands::Generate { circuit, input, output } => {
            println!("Generating proof for circuit: {}", circuit);
            let input_data = fs::read_to_string(&input)?;
            
            let (target_circuit, input_bytes) = match circuit.as_str() {
                "transfer" => {
                    let inputs: TransferInputs = serde_json::from_str(&input_data)?;
                    (ZkCircuitType::Transfer, serde_json::to_vec(&inputs)?)
                },
                "vote" => {
                    let inputs: VoteInputs = serde_json::from_str(&input_data)?;
                    (ZkCircuitType::Vote, serde_json::to_vec(&inputs)?)
                },
                _ => return Err(anyhow::anyhow!("Unknown circuit type")),
            };

            let (receipt, public_output) = zk_generate_proof(target_circuit, &input_bytes)?;
            
            // Save receipt (proof)
            let receipt_bytes = bincode::serialize(&receipt)?;
            fs::write(&output, receipt_bytes)?;
            
            println!("Proof generated and saved to {:?}", output);
            println!("Public Output: {:?}", public_output); // TODO: deserialize for pretty print
        },
        ZkCommands::Verify { circuit, proof, public_input: _ } => {
             // For verification, we need the ImageID for the circuit
             // We can hardcode/lookup based on string.
             
             use zk_circuits::{TRANSFER_ID, VOTE_ID};
             
             let image_id = match circuit.as_str() {
                 "transfer" => TRANSFER_ID,
                 "vote" => VOTE_ID,
                 _ => return Err(anyhow::anyhow!("Unknown circuit type")),
             };
             
             let proof_bytes = fs::read(&proof)?;
             let receipt: risc0_zkvm::Receipt = bincode::deserialize(&proof_bytes)?;
             
             zk_verify_proof(&receipt, image_id)?;
             println!("Proof verification SUCCESS!");
        }
    }
    Ok(())
}
