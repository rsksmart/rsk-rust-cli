// ZK proof integration tests require the `zk` feature flag.
// Run with: cargo test --features zk
#![cfg(feature = "zk")]

/// These tests run the full prove-then-verify pipeline using the RISC Zero mock prover,
/// which executes the guest circuits locally without generating real cryptographic proofs.
/// Guest circuit panics (insufficient balance, invalid vote) and journal layout are
/// verified without the hours-long overhead of real Groth16 proof generation.
use rootstock_wallet::zk::{
    types::{TransferInputs, VoteInputs},
    zk_generate_proof, zk_verify_proof, CircuitType, TRANSFER_ID, VOTE_ID,
};

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

    // Journal layout: [amount right-aligned in bytes 24..32][commitment in bytes 32..64]
    assert_eq!(journal.len(), 64, "Transfer journal must be 64 bytes");
    let mut amount_bytes = [0u8; 8];
    amount_bytes.copy_from_slice(&journal[24..32]);
    let decoded_amount = u64::from_be_bytes(amount_bytes);
    assert_eq!(decoded_amount, 50, "Journal must contain the correct amount");
}

#[test]
fn test_transfer_exact_balance_succeeds() {
    // Exact balance (no surplus) should succeed.
    let inputs = TransferInputs {
        sender_balance: 77,
        amount: 77,
        sender_id: 10,
        receiver_id: 20,
        nonce: 1,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let (receipt, _) =
        zk_generate_proof(CircuitType::Transfer, &input_bytes).expect("Exact balance should succeed");
    zk_verify_proof(&receipt, TRANSFER_ID).expect("Verification failed");
}

// ---------------------------------------------------------------------------
// Transfer circuit — error paths (pre-flight validation in host, not guest)
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
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let result = zk_generate_proof(CircuitType::Transfer, &input_bytes);
    assert!(result.is_err(), "Insufficient balance must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Insufficient balance"),
        "Error should mention balance: {msg}"
    );
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
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let result = zk_generate_proof(CircuitType::Transfer, &input_bytes);
    assert!(result.is_err(), "Zero amount must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("amount must be greater than zero") || msg.contains("zero"),
        "Error should mention zero amount: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Transfer circuit — privacy property
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
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let (_, journal) =
        zk_generate_proof(CircuitType::Transfer, &input_bytes).expect("Proof generation failed");

    // sender_id raw big-endian bytes must NOT appear verbatim in the journal.
    // The journal should only contain amount and a SHA-256 commitment.
    let sender_bytes = sender_id.to_be_bytes();
    let receiver_bytes: [u8; 4] = 0xCAFEBABEu32.to_be_bytes();

    // Check as a 4-byte subsequence anywhere in the journal
    let contains_sender = journal.windows(4).any(|w| w == sender_bytes);
    let contains_receiver = journal.windows(4).any(|w| w == receiver_bytes);

    // Note: there's a tiny chance of collision if the SHA-256 output happens to contain
    // the same 4 bytes, but with these specific test values it should not.
    assert!(
        !contains_sender,
        "Journal must not contain raw sender_id bytes (privacy violation)"
    );
    assert!(
        !contains_receiver,
        "Journal must not contain raw receiver_id bytes (privacy violation)"
    );
}

// ---------------------------------------------------------------------------
// Vote circuit — happy paths
// ---------------------------------------------------------------------------

#[test]
fn test_vote_yes_proof() {
    let inputs = VoteInputs {
        voter_id: 10,
        vote_choice: 1,
        secret: 55,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &input_bytes).expect("Proof gen failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verify failed");
    assert_eq!(journal.len(), 64, "Vote journal must be 64 bytes");
    assert_eq!(journal[31], 1, "vote_choice must be 1 in journal[31]");
}

#[test]
fn test_vote_no_proof() {
    let inputs = VoteInputs {
        voter_id: 99,
        vote_choice: 0, // No
        secret: 12345,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &input_bytes).expect("Vote No proof failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verification of No vote failed");
    assert_eq!(journal[31], 0, "vote_choice must be 0 (No) in journal[31]");
}

// ---------------------------------------------------------------------------
// Vote circuit — error paths
// ---------------------------------------------------------------------------

#[test]
fn test_vote_invalid_choice_rejected_before_proving() {
    let inputs = VoteInputs {
        voter_id: 1,
        vote_choice: 2, // Invalid
        secret: 100,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();
    let result = zk_generate_proof(CircuitType::Vote, &input_bytes);
    assert!(result.is_err(), "vote_choice=2 must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("vote choice") || msg.contains("0 or 1"),
        "Error should mention valid vote range: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Nullifier correctness and security
// ---------------------------------------------------------------------------

#[test]
fn test_vote_nullifier_is_deterministic() {
    // Same (voter_id, secret) → same nullifier in the journal.
    let inputs = VoteInputs {
        voter_id: 42,
        vote_choice: 1,
        secret: 777,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();

    let (_, journal1) = zk_generate_proof(CircuitType::Vote, &input_bytes).unwrap();
    let (_, journal2) = zk_generate_proof(CircuitType::Vote, &input_bytes).unwrap();

    assert_eq!(
        &journal1[32..64],
        &journal2[32..64],
        "Nullifier must be deterministic for the same inputs"
    );
}

#[test]
fn test_vote_different_secrets_produce_different_nullifiers() {
    // Different secrets → different nullifiers (collision resistance).
    let make_inputs = |secret: u32| VoteInputs {
        voter_id: 1,
        vote_choice: 0,
        secret,
    };

    let (_, journal_a) = zk_generate_proof(
        CircuitType::Vote,
        &serde_json::to_vec(&make_inputs(1)).unwrap(),
    )
    .unwrap();
    let (_, journal_b) = zk_generate_proof(
        CircuitType::Vote,
        &serde_json::to_vec(&make_inputs(2)).unwrap(),
    )
    .unwrap();

    assert_ne!(
        &journal_a[32..64],
        &journal_b[32..64],
        "Different secrets must produce different nullifiers"
    );
}

#[test]
fn test_vote_same_voter_id_different_secrets_no_collision() {
    // This test directly verifies that the old `wrapping_add` bug is gone.
    // voter_id=1, secret=99 and voter_id=50, secret=50 both sum to 100 under wrapping_add,
    // but must produce different SHA-256 nullifiers.
    let inputs_a = VoteInputs {
        voter_id: 1,
        vote_choice: 0,
        secret: 99,
    };
    let inputs_b = VoteInputs {
        voter_id: 50,
        vote_choice: 0,
        secret: 50,
    };

    let (_, journal_a) = zk_generate_proof(
        CircuitType::Vote,
        &serde_json::to_vec(&inputs_a).unwrap(),
    )
    .unwrap();
    let (_, journal_b) = zk_generate_proof(
        CircuitType::Vote,
        &serde_json::to_vec(&inputs_b).unwrap(),
    )
    .unwrap();

    assert_ne!(
        &journal_a[32..64],
        &journal_b[32..64],
        "Nullifiers for (voter_id=1,secret=99) and (voter_id=50,secret=50) must differ \
         (regression for wrapping_add collision bug)"
    );
}

#[test]
fn test_vote_nullifier_is_32_bytes() {
    let inputs = VoteInputs {
        voter_id: 7,
        vote_choice: 1,
        secret: 42,
    };
    let (_, journal) = zk_generate_proof(
        CircuitType::Vote,
        &serde_json::to_vec(&inputs).unwrap(),
    )
    .unwrap();
    assert_eq!(journal.len(), 64);
    // Nullifier occupies bytes 32..64.
    let nullifier = &journal[32..64];
    assert_eq!(nullifier.len(), 32, "Nullifier must be exactly 32 bytes");
    // Must not be all-zeros (extremely unlikely for a SHA-256 output).
    assert_ne!(nullifier, &[0u8; 32], "Nullifier must not be zero");
}

// ---------------------------------------------------------------------------
// Circuit type enum validation
// ---------------------------------------------------------------------------

#[test]
fn test_circuit_type_equality() {
    assert_eq!(CircuitType::Transfer, CircuitType::Transfer);
    assert_eq!(CircuitType::Vote, CircuitType::Vote);
    assert_ne!(CircuitType::Transfer, CircuitType::Vote);
}

// ---------------------------------------------------------------------------
// Type sync: ensure host types round-trip correctly
// ---------------------------------------------------------------------------

#[test]
fn test_type_sync_transfer_inputs_round_trip() {
    // Verifies that serde_json can round-trip TransferInputs without data loss.
    // This catches field additions/removals that would cause silent mismatches
    // between host code and the guest circuit.
    let original = TransferInputs {
        sender_balance: 1000,
        amount: 250,
        sender_id: 111,
        receiver_id: 222,
        nonce: 99,
    };
    let json = serde_json::to_string(&original).unwrap();
    let decoded: TransferInputs = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.sender_balance, original.sender_balance);
    assert_eq!(decoded.amount, original.amount);
    assert_eq!(decoded.sender_id, original.sender_id);
    assert_eq!(decoded.receiver_id, original.receiver_id);
    assert_eq!(decoded.nonce, original.nonce);
}

#[test]
fn test_type_sync_vote_inputs_round_trip() {
    let original = VoteInputs {
        voter_id: 999,
        vote_choice: 1,
        secret: 12345,
    };
    let json = serde_json::to_string(&original).unwrap();
    let decoded: VoteInputs = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.voter_id, original.voter_id);
    assert_eq!(decoded.vote_choice, original.vote_choice);
    assert_eq!(decoded.secret, original.secret);
}
