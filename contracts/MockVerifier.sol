// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import "./interfaces/IRiscZeroVerifier.sol";

/// @title MockVerifier
/// @notice A test-only IRiscZeroVerifier that always returns true.
///
/// WARNING: This verifier performs NO cryptographic checking. It is intended
/// exclusively for local development and integration testing. NEVER deploy
/// MockVerifier to mainnet or any environment where real security is required.
///
/// Production deployments must use Risc0's official RiscZeroGroth16Verifier:
///   https://github.com/risc0/risc0-ethereum
contract MockVerifier is IRiscZeroVerifier {
    /// @inheritdoc IRiscZeroVerifier
    function verify(
        bytes calldata, /* seal */
        bytes32,        /* imageId */
        bytes32         /* journalDigest */
    ) external view returns (bool) {
        // [AUDIT-FIX] Prevent accidental mainnet deployment. Chain 30 = RSK Mainnet.
        // Chain 31 (RSK Testnet) is intentionally allowed for integration testing.
        require(block.chainid != 30, "MockVerifier: mainnet deployment prohibited");
        return true;
    }
}
