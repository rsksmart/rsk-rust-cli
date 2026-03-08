//! ZK circuit types — re-exported from the shared `zk-circuits` crate.
//!
//! Types are defined in `zk-circuits/src/types.rs` (single source of truth).
//! Do not define types here; add them there instead.

#[cfg(feature = "zk")]
pub use zk_circuits::types::{TransferInputs, TransferPublicOutput, VoteInputs, VotePublicOutput};

/// Circuit type selector for CLI and host-side dispatch.
///
/// `clap::ValueEnum` is a CLI concern only, so `CircuitType` lives here rather
/// than in the shared crate (which has no clap dependency).
#[derive(clap::ValueEnum, Clone, Debug, Copy, PartialEq, Eq)]
pub enum CircuitType {
    Transfer,
    Vote,
}
