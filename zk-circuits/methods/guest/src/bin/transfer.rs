use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// SYNC: These types must match zk-circuits/src/types.rs exactly.
// The integration test `test_type_sync_transfer_round_trip` in tests/zk_tests.rs
// catches field-level mismatches via serialization round-trips.
#[derive(Serialize, Deserialize)]
pub struct TransferInputs {
    pub sender_balance: u64,
    pub amount: u64,
    pub sender_id: u32,
    pub receiver_id: u32,
    /// Nonce prevents commitment replay across multiple transfers between the same parties.
    pub nonce: u64,
}

fn main() {
    let input: TransferInputs = env::read();

    // Early validation — fail fast before any computation.
    if input.amount == 0 {
        panic!("Transfer amount must be greater than zero");
    }
    if input.sender_balance < input.amount {
        panic!("Insufficient balance");
    }

    // Compute transfer commitment = SHA-256("transfer-commitment-v1" || sender_id || receiver_id || nonce).
    //
    // Privacy: sender_id and receiver_id are private inputs. Only their commitment is public.
    // The verifier confirms a specific transfer took place without learning who the parties are.
    // The nonce ensures that two transfers for the same amount between the same parties
    // produce different commitments, preventing replay attacks.
    let mut hasher = Sha256::new();
    hasher.update(b"transfer-commitment-v1");
    hasher.update(input.sender_id.to_be_bytes());
    hasher.update(input.receiver_id.to_be_bytes());
    hasher.update(input.nonce.to_be_bytes());
    let commitment: [u8; 32] = hasher.finalize().into();

    // Construct a 40-byte journal compatible with Solidity's:
    //   (uint64 amount, bytes32 commitment) = abi.decode(journal, (uint64, bytes32))
    //
    //   Bytes 0.. 8: amount     (u64 big-endian, ABI-padded to 32 bytes by Solidity)
    //   Bytes 8..40: commitment (SHA-256 hash of transfer parties + nonce)
    //
    // NOTE: Solidity's abi.decode for (uint64, bytes32) expects 64 bytes total
    // (each slot is 32 bytes). Use manual assembly or abi.decode with uint256 cast.
    // Alternatively decode as: uint256 amountWord = uint256(bytes32(journal[0:32]))
    // For simpler on-chain decoding, we pad amount to 32 bytes:
    let mut journal = [0u8; 64];
    // Amount: right-aligned in first 32-byte word (big-endian, EVM convention)
    journal[24..32].copy_from_slice(&input.amount.to_be_bytes());
    // Commitment: second 32-byte word
    journal[32..64].copy_from_slice(&commitment);
    env::commit_slice(&journal);
}
