use risc0_zkvm::{ExecutorEnv, Receipt, default_prover};
use zk_circuits::{TRANSFER_ELF, VOTE_ELF};
pub use zk_circuits::{TRANSFER_ID, VOTE_ID};

pub mod types;
pub use types::*;

pub fn zk_generate_proof(
    circuit_type: CircuitType,
    fc_input: &[u8], // Input serialized as bytes or we can change signature to accept specific structs
) -> Result<(Receipt, Vec<u8>), anyhow::Error> {
    let env = match circuit_type {
        CircuitType::Transfer => {
            let input: TransferInputs = serde_json::from_slice(fc_input)?;
            ExecutorEnv::builder().write(&input)?.build()?
        }
        CircuitType::Vote => {
            let input: VoteInputs = serde_json::from_slice(fc_input)?;
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
    let journal = receipt.journal.bytes.clone(); // The committed public output

    Ok((receipt, journal))
}

pub fn zk_verify_proof(receipt: &Receipt, image_id: [u32; 8]) -> Result<(), anyhow::Error> {
    receipt.verify(image_id)?;
    Ok(())
}
