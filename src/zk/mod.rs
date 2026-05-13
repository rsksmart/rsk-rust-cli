pub mod types;
pub use types::*;

#[cfg(feature = "zk")]
use risc0_zkvm::{ExecutorEnv, Receipt, default_prover};
#[cfg(feature = "zk")]
use zk_circuits::{TRANSFER_ELF, VOTE_ELF};
#[cfg(feature = "zk")]
pub use zk_circuits::{TRANSFER_ID, VOTE_ID};

/// Generate a ZK proof for the given circuit type.
///
/// `fc_input` must be a JSON-serialized instance of the appropriate `*Inputs` struct.
///
/// Pre-flight validation is performed before invoking the prover to avoid paying
/// the cost of proof generation for inputs that would be rejected by the circuit.
#[cfg(feature = "zk")]
pub fn zk_generate_proof(
    circuit_type: CircuitType,
    fc_input: &[u8],
) -> Result<(Receipt, Vec<u8>), anyhow::Error> {
    let env = match circuit_type {
        CircuitType::Transfer => {
            let input: TransferInputs = serde_json::from_slice(fc_input)
                .map_err(|e| anyhow::anyhow!("Invalid transfer input JSON: {}", e))?;
            // Pre-flight validation: fail early before paying proving cost.
            if input.amount == 0 {
                return Err(anyhow::anyhow!("Transfer amount must be greater than zero"));
            }
            if input.sender_balance < input.amount {
                return Err(anyhow::anyhow!(
                    "Insufficient balance: sender has {} but transfer requires {}",
                    input.sender_balance,
                    input.amount
                ));
            }
            ExecutorEnv::builder().write(&input)?.build()?
        }
        CircuitType::Vote => {
            let input: VoteInputs = serde_json::from_slice(fc_input)
                .map_err(|e| anyhow::anyhow!("Invalid vote input JSON: {}", e))?;
            if input.vote_choice > 1 {
                return Err(anyhow::anyhow!(
                    "Invalid vote choice: must be 0 (No) or 1 (Yes), got {}",
                    input.vote_choice
                ));
            }
            if input.merkle_siblings.len() > 32 {
                return Err(anyhow::anyhow!(
                    "Merkle depth cannot exceed 32 levels, got {}",
                    input.merkle_siblings.len()
                ));
            }
            ExecutorEnv::builder().write(&input)?.build()?
        }
    };

    let elf = match circuit_type {
        CircuitType::Transfer => TRANSFER_ELF,
        CircuitType::Vote => VOTE_ELF,
    };

    let prover = default_prover();
    let prove_info = prover.prove(env, elf)?;
    let receipt = prove_info.receipt;
    let journal = receipt.journal.bytes.clone();

    Ok((receipt, journal))
}

/// Verify that a receipt (proof) is valid for the given circuit image ID.
#[cfg(feature = "zk")]
pub fn zk_verify_proof(receipt: &Receipt, image_id: [u32; 8]) -> Result<(), anyhow::Error> {
    receipt.verify(image_id)?;
    Ok(())
}
