// ZK proof integration tests require the `zk` feature flag.
// Run with: RISC0_DEV_MODE=1 cargo test --features zk
//
// RISC0_DEV_MODE=1 uses the mock prover: the guest executes and the journal is
// captured without generating a real cryptographic proof. This makes the full
// suite run in ~10 seconds. Without it, `default_prover()` runs the local STARK
// prover which takes minutes even for SHA-256-only circuits.
//
// For a real proof round-trip (e.g. before a production release):
//   cargo test --features zk -- -j 1   (single-threaded to avoid prover contention)
#![cfg(feature = "zk")]

use rootstock_wallet::zk::{
    types::{TransferInputs, VoteInputs},
    zk_generate_proof, zk_verify_proof, CircuitType, TRANSFER_ID, VOTE_ID,
};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Merkle helpers (must match the guest circuit algorithm exactly)
// ---------------------------------------------------------------------------

fn voter_leaf_hash(voter_id: u32) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"voter-leaf-v1");
    h.update(voter_id.to_be_bytes());
    h.finalize().into()
}

fn merkle_node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"voter-node-v1");
    h.update(left);
    h.update(right);
    h.finalize().into()
}

// ---------------------------------------------------------------------------
// Vote input helpers
// ---------------------------------------------------------------------------

/// Depth-0 Merkle tree: single eligible voter; root == leaf.
/// Most vote tests use this to stay simple and fast.
fn make_vote_inputs(voter_id: u32, vote_choice: u8, secret_seed: u8) -> VoteInputs {
    let mut secret = [0u8; 32];
    secret[0] = secret_seed;
    VoteInputs {
        voter_id,
        vote_choice,
        secret,
        merkle_root: voter_leaf_hash(voter_id),
        merkle_siblings: vec![],
        merkle_path_index: 0,
    }
}

/// Depth-1 Merkle tree with two voters: voter_id_left (index 0) and voter_id_right (index 1).
/// Returns the inputs for whichever voter is `is_right` (false = left child, true = right child).
fn make_vote_inputs_depth1(
    voter_id_left: u32,
    voter_id_right: u32,
    vote_for_right: bool,
    vote_choice: u8,
    secret_seed: u8,
) -> VoteInputs {
    let left_leaf = voter_leaf_hash(voter_id_left);
    let right_leaf = voter_leaf_hash(voter_id_right);
    let root = merkle_node_hash(&left_leaf, &right_leaf);

    let (voter_id, sibling, path_index) = if vote_for_right {
        (voter_id_right, left_leaf, 1u32) // voter is right child (bit 0 = 1)
    } else {
        (voter_id_left, right_leaf, 0u32) // voter is left child (bit 0 = 0)
    };

    let mut secret = [0u8; 32];
    secret[0] = secret_seed;
    VoteInputs {
        voter_id,
        vote_choice,
        secret,
        merkle_root: root,
        merkle_siblings: vec![sibling],
        merkle_path_index: path_index,
    }
}

// ---------------------------------------------------------------------------
// Transfer circuit — happy paths
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_proof_generation_and_verification() {
    let inputs = TransferInputs {
        sender_balance: 100,
        amount: 50,
        sender_id: 1,
        receiver_id: 2,
        nonce: 42,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Transfer, &input_bytes).expect("Proof generation failed");
    zk_verify_proof(&receipt, TRANSFER_ID).expect("Proof verification failed");

    assert_eq!(journal.len(), 64, "Transfer journal must be 64 bytes");
    let mut amount_bytes = [0u8; 8];
    amount_bytes.copy_from_slice(&journal[24..32]);
    assert_eq!(u64::from_be_bytes(amount_bytes), 50, "Journal amount mismatch");
}

#[test]
fn test_transfer_exact_balance_succeeds() {
    let inputs = TransferInputs {
        sender_balance: 77,
        amount: 77,
        sender_id: 10,
        receiver_id: 20,
        nonce: 1,
    };
    let (receipt, _) = zk_generate_proof(CircuitType::Transfer, &serde_json::to_vec(&inputs).unwrap())
        .expect("Exact balance should succeed");
    zk_verify_proof(&receipt, TRANSFER_ID).expect("Verification failed");
}

// ---------------------------------------------------------------------------
// Transfer circuit — error paths
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_insufficient_balance_rejected_before_proving() {
    let inputs = TransferInputs {
        sender_balance: 10,
        amount: 100,
        sender_id: 1,
        receiver_id: 2,
        nonce: 0,
    };
    let result = zk_generate_proof(CircuitType::Transfer, &serde_json::to_vec(&inputs).unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Insufficient balance"));
}

#[test]
fn test_transfer_zero_amount_rejected_before_proving() {
    let inputs = TransferInputs {
        sender_balance: 100,
        amount: 0,
        sender_id: 1,
        receiver_id: 2,
        nonce: 0,
    };
    let result = zk_generate_proof(CircuitType::Transfer, &serde_json::to_vec(&inputs).unwrap());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("amount must be greater than zero") || msg.contains("zero"), "{msg}");
}

// ---------------------------------------------------------------------------
// Transfer circuit — privacy
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_journal_does_not_contain_raw_sender_id() {
    let sender_id: u32 = 0xDEADBEEF;
    let inputs = TransferInputs {
        sender_balance: 200,
        amount: 100,
        sender_id,
        receiver_id: 0xCAFEBABE,
        nonce: 1,
    };
    let (_, journal) =
        zk_generate_proof(CircuitType::Transfer, &serde_json::to_vec(&inputs).unwrap())
            .expect("Proof generation failed");

    assert!(!journal.windows(4).any(|w| w == sender_id.to_be_bytes()));
    assert!(!journal.windows(4).any(|w| w == 0xCAFEBABEu32.to_be_bytes()));
}

// ---------------------------------------------------------------------------
// Vote circuit — happy paths (depth 0: single-voter tree)
// ---------------------------------------------------------------------------

#[test]
fn test_vote_yes_proof() {
    let inputs = make_vote_inputs(10, 1, 55);
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap())
            .expect("Proof gen failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verify failed");

    // Journal: [merkle_root:32][nullifier:32] = 64 bytes
    assert_eq!(journal.len(), 64, "Vote journal must be 64 bytes");
    // merkle_root in journal must match the input
    assert_eq!(&journal[0..32], &inputs.merkle_root, "merkle_root mismatch in journal");
    // Nullifier must be non-zero
    assert_ne!(&journal[32..64], &[0u8; 32], "Nullifier must not be zero");
}

#[test]
fn test_vote_no_proof() {
    let inputs = make_vote_inputs(99, 0, 42);
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap())
            .expect("Vote No proof failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verification failed");

    assert_eq!(journal.len(), 64);
    assert_eq!(&journal[0..32], &inputs.merkle_root);
    assert_ne!(&journal[32..64], &[0u8; 32]);
}

// ---------------------------------------------------------------------------
// Vote circuit — depth-1 Merkle tree (two voters)
// ---------------------------------------------------------------------------

#[test]
fn test_vote_depth1_left_voter() {
    let inputs = make_vote_inputs_depth1(1, 2, false, 1, 10);
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap())
            .expect("Depth-1 left voter proof failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verify failed");

    assert_eq!(journal.len(), 64);
    assert_eq!(&journal[0..32], &inputs.merkle_root, "merkle_root mismatch");
}

#[test]
fn test_vote_depth1_right_voter() {
    let inputs = make_vote_inputs_depth1(1, 2, true, 0, 20);
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap())
            .expect("Depth-1 right voter proof failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verify failed");

    assert_eq!(journal.len(), 64);
    assert_eq!(&journal[0..32], &inputs.merkle_root, "merkle_root mismatch");
}

// ---------------------------------------------------------------------------
// Vote circuit — error paths
// ---------------------------------------------------------------------------

#[test]
fn test_vote_invalid_choice_rejected_before_proving() {
    let inputs = make_vote_inputs(1, 2, 100); // vote_choice=2 invalid
    let result = zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("vote choice") || msg.contains("0 or 1"), "{msg}");
}

#[test]
fn test_vote_invalid_voter_rejected_by_circuit() {
    // voter_id=999 is NOT in a depth-0 tree whose root was built for voter_id=42.
    let wrong_root = voter_leaf_hash(42);
    let inputs = VoteInputs {
        voter_id: 999, // not in tree
        vote_choice: 1,
        secret: [7u8; 32],
        merkle_root: wrong_root,
        merkle_siblings: vec![],
        merkle_path_index: 0,
    };
    let result = zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap());
    assert!(result.is_err(), "Voter not in tree must be rejected by the circuit");
}

#[test]
fn test_vote_merkle_depth_exceeds_32_rejected() {
    let mut inputs = make_vote_inputs(1, 1, 1);
    inputs.merkle_siblings = vec![[0u8; 32]; 33]; // depth 33 > max 32
    let result = zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("32"));
}

// ---------------------------------------------------------------------------
// Vote circuit — nullifier correctness
// ---------------------------------------------------------------------------

#[test]
fn test_vote_nullifier_is_deterministic() {
    let inputs = make_vote_inputs(42, 1, 77);
    let bytes = serde_json::to_vec(&inputs).unwrap();
    let (_, j1) = zk_generate_proof(CircuitType::Vote, &bytes).unwrap();
    let (_, j2) = zk_generate_proof(CircuitType::Vote, &bytes).unwrap();
    assert_eq!(&j1[32..64], &j2[32..64], "Nullifier must be deterministic");
}

#[test]
fn test_vote_different_secrets_produce_different_nullifiers() {
    let a = make_vote_inputs(1, 0, 1);
    let b = make_vote_inputs(1, 0, 2); // same voter, different secret
    let (_, ja) = zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&a).unwrap()).unwrap();
    let (_, jb) = zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&b).unwrap()).unwrap();
    assert_ne!(&ja[32..64], &jb[32..64], "Different secrets must produce different nullifiers");
}

#[test]
fn test_vote_nullifier_is_32_bytes_and_nonzero() {
    let inputs = make_vote_inputs(7, 1, 42);
    let (_, journal) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs).unwrap()).unwrap();
    assert_eq!(journal.len(), 64);
    assert_ne!(&journal[32..64], &[0u8; 32]);
}

// ---------------------------------------------------------------------------
// Vote circuit — journal layout and privacy
// ---------------------------------------------------------------------------

/// Journal layout contract:
///   - [0..32]  = merkle_root (public, matches proposal's registered root)
///   - [32..64] = nullifier (public, prevents double-voting)
///   - vote_choice is private — same voter casting Yes vs No produces identical journals
#[test]
fn test_vote_journal_layout() {
    let inputs_yes = make_vote_inputs(1, 1, 10);
    let inputs_no = make_vote_inputs(1, 0, 10); // same voter/secret, different vote

    let (_, j_yes) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs_yes).unwrap()).unwrap();
    let (_, j_no) =
        zk_generate_proof(CircuitType::Vote, &serde_json::to_vec(&inputs_no).unwrap()).unwrap();

    assert_eq!(j_yes.len(), 64);
    assert_eq!(j_no.len(), 64);

    // Same voter + secret → same merkle_root and nullifier regardless of vote_choice.
    assert_eq!(&j_yes[0..32], &j_no[0..32], "merkle_root must be identical for same voter/tree");
    assert_eq!(
        &j_yes[32..64],
        &j_no[32..64],
        "nullifier must be identical (same voter/secret regardless of vote_choice)"
    );

    // merkle_root in journal must equal the tree root used to construct the inputs.
    assert_eq!(&j_yes[0..32], &inputs_yes.merkle_root);
}

// ---------------------------------------------------------------------------
// Circuit type enum
// ---------------------------------------------------------------------------

#[test]
fn test_circuit_type_equality() {
    assert_eq!(CircuitType::Transfer, CircuitType::Transfer);
    assert_eq!(CircuitType::Vote, CircuitType::Vote);
    assert_ne!(CircuitType::Transfer, CircuitType::Vote);
}

// ---------------------------------------------------------------------------
// Type sync: host types must round-trip through serde_json without data loss
// ---------------------------------------------------------------------------

#[test]
fn test_type_sync_transfer_inputs_round_trip() {
    let original = TransferInputs {
        sender_balance: 1000,
        amount: 250,
        sender_id: 111,
        receiver_id: 222,
        nonce: 99,
    };
    let decoded: TransferInputs =
        serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(decoded.sender_balance, original.sender_balance);
    assert_eq!(decoded.amount, original.amount);
    assert_eq!(decoded.sender_id, original.sender_id);
    assert_eq!(decoded.receiver_id, original.receiver_id);
    assert_eq!(decoded.nonce, original.nonce);
}

#[test]
fn test_type_sync_vote_inputs_round_trip() {
    let inputs = make_vote_inputs(999, 1, 123);
    let decoded: VoteInputs =
        serde_json::from_str(&serde_json::to_string(&inputs).unwrap()).unwrap();
    assert_eq!(decoded.voter_id, inputs.voter_id);
    assert_eq!(decoded.vote_choice, inputs.vote_choice);
    assert_eq!(decoded.secret, inputs.secret);
    assert_eq!(decoded.merkle_root, inputs.merkle_root);
    assert_eq!(decoded.merkle_siblings, inputs.merkle_siblings);
    assert_eq!(decoded.merkle_path_index, inputs.merkle_path_index);
}
