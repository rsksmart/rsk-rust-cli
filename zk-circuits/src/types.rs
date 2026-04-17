//! Shared ZK circuit types — single source of truth for host-side code.
//!
//! Guest binaries (zk-circuits/methods/guest/src/bin/) maintain local copies
//! of the input types (marked `// SYNC`) because they compile to RISC-V and
//! cannot import from this crate directly. The integration tests in
//! `tests/zk_tests.rs` verify round-trip serialization to catch any divergence.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Vote circuit types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteInputs {
    pub voter_id: u32,
    /// Must be 0 (No) or 1 (Yes).
    pub vote_choice: u8,
    /// 32-byte secret for nullifier derivation. Must be CSPRNG-generated.
    pub secret: [u8; 32],

    // --- Merkle eligibility proof ---
    //
    // The circuit verifies in-circuit that `voter_id` is a leaf of the
    // Merkle tree whose root is `merkle_root`. The root is committed to
    // the journal so the smart contract can verify the voter was in the
    // authorised voter set for the specific proposal.
    //
    // Leaf hash  : SHA-256("voter-leaf-v1" || voter_id_be32)
    // Node hash  : SHA-256("voter-node-v1" || left_child || right_child)
    // Tree depth : merkle_siblings.len()  (max 32, supports up to 2^32 voters)

    /// Expected Merkle root — committed to the journal (public output).
    pub merkle_root: [u8; 32],
    /// Sibling hashes along the path from the voter's leaf to the root.
    /// Length equals the tree depth (0 = single-voter tree, root == leaf).
    pub merkle_siblings: Vec<[u8; 32]>,
    /// Path direction bitmask: bit i = 0 → voter is the LEFT child at level i.
    pub merkle_path_index: u32,
}

// ---------------------------------------------------------------------------
// Transfer circuit types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferInputs {
    pub sender_balance: u64,
    pub amount: u64,
    pub sender_id: u32,
    pub receiver_id: u32,
    /// Nonce prevents commitment replay across multiple transfers.
    pub nonce: u64,
}
