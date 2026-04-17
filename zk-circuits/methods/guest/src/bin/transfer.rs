use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// SYNC: These types must match zk-circuits/src/types.rs exactly.
// The integration test `test_type_sync_transfer_inputs_round_trip` in tests/zk_tests.rs
// catches field-level mismatches via serialization round-trips.
//
// SECURITY WARNING (demonstration only): `sender_balance` is a private input
// supplied by the prover and is NOT cryptographically bound to any on-chain
// state (no Merkle root, no commitment scheme). Do not use this circuit for
// production DeFi. A production implementation must bind the balance to an
// on-chain commitment that the verifier can check.
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

    // Journal layout (64 bytes, ABI-compatible with Solidity):
    //
    //   bytes  0..32: amount — u64 right-aligned in a bytes32 word (EVM convention)
    //                          i.e. zero-padded at the front; amount sits at bytes[24..32]
    //   bytes 32..64: commitment — SHA-256 hash of transfer parties + nonce
    //
    // Solidity decoding:
    //   uint256 amountWord = uint256(bytes32(journal[0:32]));
    //   bytes32 commitment  = bytes32(journal[32:64]);
    let mut journal = [0u8; 64];
    journal[24..32].copy_from_slice(&input.amount.to_be_bytes());
    journal[32..64].copy_from_slice(&commitment);
    env::commit_slice(&journal);
}
