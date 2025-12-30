use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferInputs {
    pub sender_balance: u64,
    pub amount: u64,
    pub sender_id: u32,
    pub receiver_id: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransferPublicOutput {
    pub sender_id: u32,
    pub receiver_id: u32,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteInputs {
    pub voter_id: u32,
    pub vote_choice: u8,
    pub secret: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VotePublicOutput {
    pub vote_hash: [u8; 32],
    pub nullifier: [u8; 32],
}

pub enum CircuitType {
    Transfer,
    Vote,
}
