use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// SYNC: These types must match zk-circuits/src/types.rs exactly.
//
// Why no BN254 here: ark-ec scalar multiplication inside the RISC-V emulator
// (Risc0 v1.x, no BN254 precompile) takes 2-5 minutes per proof. ElGamal
// encryption is performed by the HOST in demo_voting.rs using native Rust.
// The ZK proof guarantees:
//   1. vote_choice ∈ {0, 1}
//   2. voter_id is a leaf in the Merkle tree with root `merkle_root`
//   3. The nullifier is derived correctly (prevents double-voting)
// Production note: Risc0 v2.x adds BN254 precompiles for full in-circuit encryption.
#[derive(Serialize, Deserialize)]
pub struct VoteInputs {
    pub voter_id: u32,
    pub vote_choice: u8,
    pub secret: [u8; 32],
    pub merkle_root: [u8; 32],
    pub merkle_siblings: Vec<[u8; 32]>,
    pub merkle_path_index: u32,
}

/// SHA-256("voter-leaf-v1" || voter_id_be32)
fn leaf_hash(voter_id: u32) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"voter-leaf-v1");
    h.update(voter_id.to_be_bytes());
    h.finalize().into()
}

/// Walk the Merkle path from the voter's leaf to the root.
/// Returns the computed root; the circuit panics if it differs from the expected root.
fn compute_merkle_root(voter_id: u32, siblings: &[[u8; 32]], path_index: u32) -> [u8; 32] {
    let mut current = leaf_hash(voter_id);
    for (i, sibling) in siblings.iter().enumerate() {
        let voter_is_right = (path_index >> i) & 1 == 1;
        let mut h = Sha256::new();
        h.update(b"voter-node-v1");
        if voter_is_right {
            // voter is the right child; sibling is on the left
            h.update(sibling);
            h.update(&current);
        } else {
            // voter is the left child; sibling is on the right
            h.update(&current);
            h.update(sibling);
        }
        current = h.finalize().into();
    }
    current
}

fn main() {
    let input: VoteInputs = env::read();

    // Validate vote choice.
    if input.vote_choice > 1 {
        panic!("Invalid vote choice: must be 0 (No) or 1 (Yes)");
    }

    // Validate Merkle depth.
    if input.merkle_siblings.len() > 32 {
        panic!("Merkle depth cannot exceed 32 levels");
    }

    // Verify voter eligibility: recompute the Merkle root and compare.
    let computed_root = compute_merkle_root(
        input.voter_id,
        &input.merkle_siblings,
        input.merkle_path_index,
    );
    if computed_root != input.merkle_root {
        panic!("Voter is not in the eligible voter set (Merkle proof invalid)");
    }

    // Nullifier = SHA-256("nullifier-v1" || voter_id_be || secret).
    // Deterministic per (voter_id, secret) — prevents double-voting on-chain.
    let mut h = Sha256::new();
    h.update(b"nullifier-v1");
    h.update(input.voter_id.to_be_bytes());
    h.update(input.secret);
    let nullifier: [u8; 32] = h.finalize().into();

    // Journal layout (64 bytes):
    //   bytes  0..32: merkle_root — the eligible-voter Merkle root (public)
    //   bytes 32..64: nullifier   — SHA-256 commitment (public)
    //
    // The smart contract verifies journal[0:32] == proposals[proposalId].merkleRoot,
    // ensuring this proof is bound to the correct eligible-voter set.
    //
    // vote_choice is validated privately inside the circuit and intentionally
    // NOT committed to the journal to preserve ballot secrecy.
    let mut journal = [0u8; 64];
    journal[0..32].copy_from_slice(&input.merkle_root);
    journal[32..64].copy_from_slice(&nullifier);
    env::commit_slice(&journal);
}
