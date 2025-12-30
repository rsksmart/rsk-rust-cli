use rootstock_wallet::zk::{
    CircuitType,
    types::{TransferInputs, VoteInputs},
    zk_generate_proof, zk_verify_proof,
};
use zk_circuits::{TRANSFER_ID, VOTE_ID};

#[test]
fn test_transfer_proof_generation_and_verification() {
    let inputs = TransferInputs {
        sender_balance: 100,
        amount: 50,
        sender_id: 1,
        receiver_id: 2,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();

    // Generate
    let (receipt, journal) =
        zk_generate_proof(CircuitType::Transfer, &input_bytes).expect("Proof generation failed");

    // Verify
    zk_verify_proof(&receipt, TRANSFER_ID).expect("Proof verification failed");

    // Check Journal
    use rootstock_wallet::zk::types::TransferPublicOutput;
    let output: TransferPublicOutput = risc0_zkvm::serde::from_slice(&journal).unwrap();
    assert_eq!(output.amount, 50);
}

#[test]
fn test_vote_proof_logic() {
    let inputs = VoteInputs {
        voter_id: 10,
        vote_choice: 1,
        secret: 55,
    };
    let input_bytes = serde_json::to_vec(&inputs).unwrap();

    let (receipt, journal) =
        zk_generate_proof(CircuitType::Vote, &input_bytes).expect("Proof gen failed");
    zk_verify_proof(&receipt, VOTE_ID).expect("Verify failed");
}
