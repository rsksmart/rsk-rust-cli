// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract PrivateVoting {
    // Merkle root of eligible voters
    bytes32 public merkleRoot;
    
    // Risc0 Image ID for the Vote Circuit
    bytes32 public imageId;
    
    // Nullifiers to prevent double voting
    mapping(uint256 => bool) public nullifiers;
    
    // Mapping of proposal ID to vote counts (0 = No, 1 = Yes)
    mapping(uint256 => mapping(uint256 => uint256)) public voteCounts;

    event VoteCast(uint256 indexed proposalId, uint256 voteChoice);

    constructor(bytes32 _merkleRoot, bytes32 _imageId) {
        merkleRoot = _merkleRoot;
        imageId = _imageId;
    }

    // Verify Risc0 proof
    // Note: In a real deployment, we would use Risc0's RiscZeroGroth16Verifier contract.
    // For this demo, we simulate verification or verify a mock proof.
    // We expect the proof to carry the Journal which contains (vote_hash, nullifier).
    function castVote(
        bytes calldata journal,
        bytes calldata seal // The proof
    ) public {
        // 1. Verify Proof (Mock for now, or call verifier)
        // require(verifier.verify(seal, imageId, journal), "Invalid Proof");
        
        // 2. Decode Journal
        // Journal Format: [vote_choice (4 bytes), nullifier (4 bytes)] (Simplified)
        (uint32 voteChoice, uint32 nullifier) = abi.decode(journal, (uint32, uint32));
        
        // 3. Check Nullifier
        require(!nullifiers[nullifier], "Double voting detected");
        nullifiers[nullifier] = true;
        
        // 4. Record Vote
        // We assume proposalId is 0 for this demo or included in journal
        uint256 proposalId = 0;
        voteCounts[proposalId][voteChoice]++;
        
        emit VoteCast(proposalId, voteChoice);
    }
    
    // Helper to decode journal manually if needed (Solidity abi.decode works for standard encoding)
}
