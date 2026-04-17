// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// @title IRiscZeroVerifier
/// @notice Interface for a RISC Zero ZK proof verifier.
/// @dev Both PrivateVoting.sol and MockVerifier.sol import from here so the
///      interface has a single source of truth. Production implementations use
///      Risc0's official RiscZeroGroth16Verifier:
///      https://github.com/risc0/risc0-ethereum
interface IRiscZeroVerifier {
    /// @notice Verify a RISC Zero proof.
    /// @param seal          The proof seal (bincode-serialized Receipt for STARK,
    ///                      or compact ~256-byte seal for Groth16).
    /// @param imageId       The expected guest ELF image ID (computed at build time).
    /// @param journalDigest SHA-256 digest of the public journal bytes.
    /// @return              True if the proof is valid.
    function verify(
        bytes calldata seal,
        bytes32 imageId,
        bytes32 journalDigest
    ) external view returns (bool);
}
