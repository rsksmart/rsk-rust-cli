// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import "./interfaces/IRiscZeroVerifier.sol";

/// @title PrivateVoting
/// @notice On-chain private voting with:
///   - Admin-controlled proposal lifecycle (Created → Open → Closed → Tallied)
///   - In-circuit SHA-256 Merkle membership proof (only registered voters may vote)
///   - Additively homomorphic ElGamal tally (votes stay encrypted on-chain)
///   - ZK proof verification via a pluggable IRiscZeroVerifier
///
/// ZK journal layout (64 bytes per vote):
///   bytes  0..32: merkle_root — must match proposals[proposalId].merkleRoot
///   bytes 32..64: nullifier   — SHA-256("nullifier-v1" || voter_id_be || secret)
///
/// vote_choice is validated privately in-circuit and NOT committed to the journal,
/// preserving ballot secrecy. The ElGamal ciphertext (C1, C2) encodes it
/// in encrypted form for the homomorphic tally.
///
/// Risc0 v1.x limitation: ElGamal is computed host-side (no BN254 precompile).
/// Upgrade to Risc0 v2.x to bind the ciphertext cryptographically to the proof.
contract PrivateVoting {
    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    enum ProposalState { Created, Open, Closed, Tallied }

    struct EncryptedTally {
        uint256 c1x;
        uint256 c1y;
        uint256 c2x;
        uint256 c2y;
        bool initialized;
    }

    struct Proposal {
        /// SHA-256 Merkle root of the eligible voter set for this proposal.
        bytes32 merkleRoot;
        ProposalState state;
        /// Unix timestamp voting deadline; 0 = no automatic deadline.
        uint256 deadline;
        EncryptedTally tally;
        uint256 voteCount;
    }

    // -----------------------------------------------------------------------
    // State
    // -----------------------------------------------------------------------

    address public immutable admin;
    bytes32 public immutable imageId;
    IRiscZeroVerifier public immutable verifier;

    mapping(uint256 => Proposal) public proposals;
    uint256 public proposalCount;

    /// Nullifiers are global (across all proposals) to prevent a nullifier
    /// from being reused even if a voter somehow votes on two proposals.
    mapping(bytes32 => bool) public nullifiers;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    event ProposalCreated(uint256 indexed proposalId, bytes32 merkleRoot, uint256 deadline);
    event ProposalOpened(uint256 indexed proposalId);
    event ProposalClosed(uint256 indexed proposalId);
    event VoteCast(uint256 indexed proposalId, bytes32 nullifier, uint256 voteCount);
    event TallySubmitted(uint256 indexed proposalId);

    // -----------------------------------------------------------------------
    // Modifiers
    // -----------------------------------------------------------------------

    modifier onlyAdmin() {
        require(msg.sender == admin, "Only admin");
        _;
    }

    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    constructor(bytes32 _imageId, address _verifier) {
        admin = msg.sender;
        imageId = _imageId;
        verifier = IRiscZeroVerifier(_verifier);
    }

    // -----------------------------------------------------------------------
    // Proposal lifecycle (admin)
    // -----------------------------------------------------------------------

    /// @notice Create a new proposal. Only the admin may call this.
    /// @param merkleRoot  SHA-256 Merkle root of eligible voters. Must be non-zero.
    /// @param deadline    Voting deadline as Unix timestamp; 0 = admin closes manually.
    /// @return proposalId The ID of the newly created proposal.
    function createProposal(bytes32 merkleRoot, uint256 deadline)
        external
        onlyAdmin
        returns (uint256 proposalId)
    {
        require(merkleRoot != bytes32(0), "Merkle root cannot be zero");
        require(deadline == 0 || deadline > block.timestamp, "Deadline must be in the future");

        proposalId = proposalCount++;
        proposals[proposalId].merkleRoot = merkleRoot;
        proposals[proposalId].state = ProposalState.Created;
        proposals[proposalId].deadline = deadline;

        emit ProposalCreated(proposalId, merkleRoot, deadline);
    }

    /// @notice Open a proposal for voting. Only the admin may call this.
    function openProposal(uint256 proposalId) external onlyAdmin {
        require(proposals[proposalId].state == ProposalState.Created, "Must be in Created state");
        proposals[proposalId].state = ProposalState.Open;
        emit ProposalOpened(proposalId);
    }

    /// @notice Close a proposal. Admin may close at any time; anyone may close once
    ///         the deadline has passed.
    function closeProposal(uint256 proposalId) external {
        Proposal storage p = proposals[proposalId];
        require(p.state == ProposalState.Open, "Must be Open");
        bool deadlinePassed = p.deadline > 0 && block.timestamp >= p.deadline;
        require(msg.sender == admin || deadlinePassed, "Only admin, or wait for deadline");
        p.state = ProposalState.Closed;
        emit ProposalClosed(proposalId);
    }

    /// @notice Record that the encrypted tally has been decrypted and the result
    ///         published off-chain. Moves the proposal to the Tallied state.
    ///
    /// In production, this should accept a ZK proof of correct ElGamal decryption
    /// so the result is verifiable on-chain without trusting the admin.
    function submitTally(uint256 proposalId) external onlyAdmin {
        require(proposals[proposalId].state == ProposalState.Closed, "Must be Closed");
        proposals[proposalId].state = ProposalState.Tallied;
        emit TallySubmitted(proposalId);
    }

    // -----------------------------------------------------------------------
    // Voting
    // -----------------------------------------------------------------------

    /// @notice Cast a private vote with a homomorphic tally update.
    ///
    /// @param proposalId  Target proposal (must be Open and within deadline).
    /// @param journal     64-byte ZK journal: [merkle_root:32][nullifier:32].
    /// @param seal        ZK proof seal (Groth16 compact bytes or STARK bincode).
    /// @param c1x         ElGamal C1 x-coordinate (host-computed BN254 G1 point).
    /// @param c1y         ElGamal C1 y-coordinate.
    /// @param c2x         ElGamal C2 x-coordinate.
    /// @param c2y         ElGamal C2 y-coordinate.
    function castVote(
        uint256 proposalId,
        bytes calldata journal,
        bytes calldata seal,
        uint256 c1x,
        uint256 c1y,
        uint256 c2x,
        uint256 c2y
    ) external {
        Proposal storage p = proposals[proposalId];
        require(p.state == ProposalState.Open, "Proposal is not open for voting");
        require(p.deadline == 0 || block.timestamp < p.deadline, "Voting deadline has passed");
        require(journal.length == 64, "Invalid journal length");

        // 1. Verify Merkle root — proves this voter was in the eligible set
        //    for this specific proposal.
        bytes32 journalMerkleRoot;
        assembly { journalMerkleRoot := calldataload(journal.offset) }
        require(journalMerkleRoot == p.merkleRoot, "Voter not in eligible set for this proposal");

        // 2. Extract nullifier; mark used BEFORE external call (CEI pattern).
        bytes32 nullifier;
        assembly { nullifier := calldataload(add(journal.offset, 32)) }
        require(!nullifiers[nullifier], "Double voting detected");
        nullifiers[nullifier] = true;

        // 3. Verify the ZK proof. Proves: vote_choice ∈ {0,1}, correct Merkle
        //    membership, and correct nullifier derivation.
        bytes32 journalDigest = sha256(journal);
        require(verifier.verify(seal, imageId, journalDigest), "Invalid ZK proof");

        // 4. Homomorphic tally update (alt_bn128_add precompile, EIP-196).
        //    Note: C1/C2 are supplied by the caller and not bound to the proof
        //    in Risc0 v1.x. Upgrade to v2.x for full cryptographic binding.
        EncryptedTally storage tally = p.tally;
        if (!tally.initialized) {
            tally.c1x = c1x;
            tally.c1y = c1y;
            tally.c2x = c2x;
            tally.c2y = c2y;
            tally.initialized = true;
        } else {
            (tally.c1x, tally.c1y) = _ecAdd(tally.c1x, tally.c1y, c1x, c1y);
            (tally.c2x, tally.c2y) = _ecAdd(tally.c2x, tally.c2y, c2x, c2y);
        }
        p.voteCount++;

        emit VoteCast(proposalId, nullifier, p.voteCount);
    }

    // -----------------------------------------------------------------------
    // View helpers
    // -----------------------------------------------------------------------

    function getProposalState(uint256 proposalId) external view returns (ProposalState) {
        return proposals[proposalId].state;
    }

    function getEncryptedTally(uint256 proposalId)
        external
        view
        returns (uint256 c1x, uint256 c1y, uint256 c2x, uint256 c2y, uint256 voteCount)
    {
        EncryptedTally storage t = proposals[proposalId].tally;
        return (t.c1x, t.c1y, t.c2x, t.c2y, proposals[proposalId].voteCount);
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    /// @dev alt_bn128 (BN254) point addition via EIP-196 precompile.
    function _ecAdd(
        uint256 x1, uint256 y1,
        uint256 x2, uint256 y2
    ) internal view returns (uint256 x3, uint256 y3) {
        uint256[4] memory input = [x1, y1, x2, y2];
        bool success;
        assembly {
            // 0x06 = alt_bn128_add; 0x80 = 128 bytes in; 0x40 = 64 bytes out
            success := staticcall(gas(), 0x06, input, 0x80, input, 0x40)
            x3 := mload(input)
            y3 := mload(add(input, 0x20))
        }
        require(success, "EC_ADD failed");
    }
}
