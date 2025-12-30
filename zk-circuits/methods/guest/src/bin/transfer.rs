use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct TransferInputs {
    pub sender_balance: u64,
    pub amount: u64,
    // In a real ZK rollup, we'd verify signatures or Merkle paths here.
    // For this MVP, we prove that the sender has enough balance.
    pub sender_id: u32,
    pub receiver_id: u32,
}

#[derive(Serialize, Deserialize)]
pub struct TransferPublicOutput {
    pub sender_id: u32,
    pub receiver_id: u32,
    pub amount: u64,
}

fn main() {
    // 1. Read input
    let input: TransferInputs = env::read();

    // 2. Logic: Ensure sufficient balance
    if input.sender_balance < input.amount {
        panic!("Insufficient balance");
    }

    // 3. Commit to public output (what the verifier sees)
    let output = TransferPublicOutput {
        sender_id: input.sender_id,
        receiver_id: input.receiver_id,
        amount: input.amount,
    };
    
    env::commit(&output);
}
