use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// SYNC: These types must match zk-circuits/src/types.rs exactly.
// The integration test `test_type_sync_vote_round_trip` in tests/zk_tests.rs
// catches field-level mismatches via serialization round-trips.
#[derive(Serialize, Deserialize)]
pub struct VoteInputs {
    pub voter_id: u32,
    /// Must be 0 (No) or 1 (Yes).
    pub vote_choice: u8,
    /// Private secret used to derive the nullifier. Never revealed publicly.
    pub secret: u32,
}

fn main() {
    let input: VoteInputs = env::read();

    if input.vote_choice > 1 {
        panic!("Invalid vote choice: must be 0 (No) or 1 (Yes)");
    }

    // Compute vote_hash: 32-byte big-endian word with vote_choice in the last byte.
    // The upper 31 bytes are zeroed; only the choice (0 or 1) is committed.
    let mut vote_hash = [0u8; 32];
    vote_hash[31] = input.vote_choice;

    // Compute nullifier = SHA-256("nullifier-v1" || voter_id_be || secret_be).
    //
    // Security properties:
    //   - Deterministic: same voter + same secret → same nullifier (prevents double-voting).
    //   - One-way: nullifier cannot be used to recover voter_id or secret.
    //   - Collision-resistant: distinct (voter_id, secret) pairs produce distinct nullifiers
    //     with overwhelming probability (unlike the previous `wrapping_add` approach which
    //     had trivial collisions, e.g. voter_id=1,secret=99 == voter_id=50,secret=50).
    //   - Domain-separated: "nullifier-v1" prefix prevents cross-context hash collisions.
    let mut hasher = Sha256::new();
    hasher.update(b"nullifier-v1");
    hasher.update(input.voter_id.to_be_bytes());
    hasher.update(input.secret.to_be_bytes());
    let nullifier: [u8; 32] = hasher.finalize().into();

    // Construct a 64-byte journal compatible with Solidity's:
    //   (bytes32 voteHash, bytes32 nullifier) = abi.decode(journal, (bytes32, bytes32))
    //
    // Using env::commit_slice bypasses risc0's serde layer, ensuring the raw byte
    // layout exactly matches what the Solidity verifier expects.
    //   Bytes  0..32: vote_hash  (vote_choice in last byte, rest zeroed)
    //   Bytes 32..64: nullifier  (SHA-256 hash)
    let mut journal = [0u8; 64];
    journal[0..32].copy_from_slice(&vote_hash);
    journal[32..64].copy_from_slice(&nullifier);
    env::commit_slice(&journal);
}
