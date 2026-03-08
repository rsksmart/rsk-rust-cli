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
    /// Private secret used to derive the nullifier. Never revealed publicly.
    pub secret: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VotePublicOutput {
    /// 32-byte big-endian word: last byte = vote_choice, rest zeroed.
    pub vote_hash: [u8; 32],
    /// SHA-256("nullifier-v1" || voter_id_be || secret_be).
    /// Prevents double-voting without revealing voter identity.
    pub nullifier: [u8; 32],
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TransferPublicOutput {
    /// Proven transfer amount (public).
    pub amount: u64,
    /// SHA-256("transfer-commitment-v1" || sender_id_be || receiver_id_be || nonce_be).
    /// Proves the transfer involved specific parties without revealing their identities.
    pub commitment: [u8; 32],
}
