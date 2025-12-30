use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VoteInputs {
    pub voter_id: u32,
    pub vote_choice: u8, // 0 = No, 1 = Yes
    pub secret: u32,     // Private key or secret to nullify
}

#[derive(Serialize, Deserialize)]
pub struct VotePublicOutput {
    pub vote_hash: [u8; 32], // ABI-encoded (BE) 32 bytes
    pub nullifier: [u8; 32], // ABI-encoded (BE) 32 bytes
}

fn main() {
    let input: VoteInputs = env::read();

    // Logic: Verify vote is valid (e.g. 0 or 1)
    if input.vote_choice > 1 {
        panic!("Invalid vote choice");
    }

    // Prevent double voting (Mock derivation)
    let nullifier_val = input.voter_id.wrapping_add(input.secret);

    // Prepare ABI-encoded 32-byte words (Big Endian for EVM compatibility)
    let mut vote_hash_bytes = [0u8; 32];
    vote_hash_bytes[31] = input.vote_choice;

    let mut nullifier_bytes = [0u8; 32];
    let n_bytes = nullifier_val.to_be_bytes();
    nullifier_bytes[28..32].copy_from_slice(&n_bytes);

    let output = VotePublicOutput {
        vote_hash: vote_hash_bytes,
        nullifier: nullifier_bytes,
    };

    env::commit(&output);
}
