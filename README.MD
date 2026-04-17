[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/rsksmart/rsk-rust-cli/badge)](https://scorecard.dev/viewer/?uri=github.com/rsksmart/rsk-rust-cli)
[![CodeQL](https://github.com/rsksmart/rsk-rust-cli/workflows/CodeQL/badge.svg)](https://github.com/rsksmart/rsk-rust-cli/actions?query=workflow%3ACodeQL)

<img src="rootstock-logo.png" alt="RSK Logo" style="width:100%; height: auto;" />

# Rootstock Rust CLI

A command-line interface (CLI) tool written in Rust for interacting with the Rootstock blockchain.  
This tool provides comprehensive functionality to connect to an Ethereum-compatible RPC endpoint and manage wallets, query balances, send transactions, and interact with the Rootstock network.

## Features

### Wallet Management
- Create new wallets with custom names
- Import existing wallets using private keys
- Switch between multiple wallets
- Backup and restore wallet data
- List all available wallets
- Rename or delete wallets

### Zero-Knowledge Privacy (Risc0)
- **Private Voting**: Additively homomorphic encryption (ElGamal) to hide vote choices while allowing on-chain tallying.
- **Anonymous Transfers**: Proof of balance and ownership without revealing identities (Demonstration Only).
- **Proof Verification**: Verify ZK proofs locally or on the Rootstock network.

### Asset Management
- Check RBTC balance for any address
- View token balances (supports all ERC-20 tokens)
- Add custom tokens to your wallet
- Remove unwanted tokens from your token list

### Transactions
- Send RBTC to any address
- Transfer ERC-20 tokens
- View transaction history with filtering options
- Check transaction status
- Bulk transfer functionality

### Contact Management
- Save addresses as contacts with notes and tags
- Search, update, and manage your address book
- Import/export contact lists

### Network & Configuration
- Switch between Mainnet and Testnet
- Configure custom RPC endpoints
- Manage API keys (Alchemy/Rootstock)
- Built-in diagnostics (`config doctor`)

## 🛠️ Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Risc0 Toolchain (`rzup`)](https://risczero.com/install):
  ```bash
  curl -L https://risczero.com/install | bash
  rzup install
  ```

### Building from Source
1. Clone the repository:
   ```bash
   git clone https://github.com/rsksmart/rsk-rust-cli.git
   cd rsk-rust-cli
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run the application:
   ```bash
   ./target/release/rootstock-wallet
   ```

## 🔐 ZK-SNARK Privacy Extensions

This CLI uses [Risc0](https://dev.risczero.com/) to generate and verify zero-knowledge proofs inside a RISC-V zkVM. Two circuits are provided: a privacy-preserving **vote** circuit and a demonstration **transfer** circuit.

### Building with ZK Support

ZK features are behind an optional Cargo feature flag. You must also have the Risc0 RISC-V toolchain installed.

```bash
# Install Risc0 toolchain (one-time)
curl -L https://risczero.com/install | bash
rzup install

# Build with ZK enabled
cargo build --features zk

# Run all ZK integration tests (mock prover, ~10s)
RISC0_DEV_MODE=1 cargo test --features zk
```

---

### Circuit 1: Private Voting (Homomorphic ElGamal)

#### Architecture: host-side ElGamal + ZK nullifier

The system splits responsibilities between the **ZK circuit** (fast, RISC-V) and the **host** (native Rust):

| Component | Responsibility |
|-----------|---------------|
| **ZK circuit (guest)** | Prove `vote_choice ∈ {0,1}` and compute nullifier via SHA-256 |
| **Host (native Rust)** | Compute ElGamal ciphertext `(C1, C2)` on BN254 — milliseconds |
| **Smart contract** | Verify ZK proof, check nullifier, add `(C1, C2)` to homomorphic tally |

**Why the split?** Risc0 v1.x has no BN254 precompile. Software-emulated BN254 scalar multiplication inside the RISC-V zkVM takes 2–5 minutes per proof. SHA-256 has a hardware accelerator in the zkVM and completes in seconds. Risc0 v2.x adds BN254 precompile support; upgrading would allow moving ElGamal back into the circuit for full cryptographic binding between the proof and the ciphertext.

#### How it works

```
C1 = g^r          (randomised commitment; r = fresh CSPRNG scalar)
C2 = Y^r + g^v   (encrypted ballot: v=0 No, v=1 Yes; Y = election public key)
```

The smart contract accumulates votes by adding ciphertexts homomorphically via the `alt_bn128_add` precompile — it **never** decrypts individual votes. A nullifier (SHA-256 of voter ID + 32-byte secret) prevents double-voting.

#### Privacy guarantees

| What is hidden | What is public |
|----------------|----------------|
| Voter identity | Nullifier (unlinkable to voter without the secret) |
| Randomness `r` | Encrypted ciphertext `(C1, C2)` |
| — | Aggregate homomorphic tally (decryptable only by election authority) |

> **Note (Risc0 v1.x limitation):** The ZK proof proves `vote_choice ∈ {0,1}` but does not cryptographically bind the ciphertext `(C1, C2)` to the proof — ElGamal is computed by the host, not inside the circuit. This means a malicious prover could supply a mismatched ciphertext. For demo and testnet use this is acceptable; production deployments should upgrade to Risc0 v2.x.

#### ZK journal layout (64 bytes)

```
bytes  0..32  merkle_root — SHA-256 Merkle root of the eligible voter set
bytes 32..64  nullifier   — SHA-256("nullifier-v1" || voter_id_be32 || secret_be256)
```

`vote_choice` is validated privately inside the ZK circuit (proved to be 0 or 1) but is **not** committed to the journal, preserving ballot secrecy. The ElGamal ciphertext `(C1, C2)` — passed as separate `uint256` parameters — encodes the choice in encrypted form for the homomorphic tally.

On-chain Solidity — journal decoding:
```solidity
// journal.length must be 64
bytes32 journalMerkleRoot;
bytes32 nullifier;
assembly {
    journalMerkleRoot := calldataload(journal.offset)
    nullifier         := calldataload(add(journal.offset, 32))
}
require(journalMerkleRoot == proposals[proposalId].merkleRoot, "Voter not in eligible set");
require(!nullifiers[nullifier], "Double voting detected");
```

#### Merkle voter registration

Voters are registered via a SHA-256 Merkle tree committed on-chain with the proposal. The circuit proves the voter is in the eligible set without revealing which voter voted.

**Leaf hash**: `SHA-256("voter-leaf-v1" || voter_id_be32)`
**Node hash**: `SHA-256("voter-node-v1" || left_child_bytes32 || right_child_bytes32)`

- **Single-voter (depth 0)**: `merkle_root = leaf_hash(voter_id)`, `merkle_siblings = []`, `merkle_path_index = 0`
- **Multi-voter (depth N)**: build the tree off-chain; supply the N sibling hashes along the path from leaf to root; `merkle_path_index` is a bitmask where bit `i = 1` means the voter is the right child at level `i`

#### Input JSON schema

```json
{
  "voter_id": 12345,
  "vote_choice": 1,
  "secret": [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32],
  "merkle_root": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
  "merkle_siblings": [],
  "merkle_path_index": 0
}
```

- `vote_choice`: `0` = No, `1` = Yes (any other value is rejected before proving)
- `secret`: 32 random bytes — must be CSPRNG-generated; protects the nullifier from brute-force
- `merkle_root`: 32-byte SHA-256 Merkle root matching the on-chain proposal's registered root
- `merkle_siblings`: array of 32-byte sibling hashes along the Merkle path (empty for single-voter)
- `merkle_path_index`: bitmask; bit `i` indicates voter is right child at tree level `i`

#### Generate and verify a vote proof locally

```bash
# 1. Compute the leaf hash for voter_id=42 (matches the circuit's leaf_hash function):
#    SHA-256("voter-leaf-v1" || 0x0000002a)
#    For depth-0 (single voter): merkle_root == leaf_hash

# 2. Create an input file (depth-0: single eligible voter)
cat > vote_input.json << 'EOF'
{
  "voter_id": 42,
  "vote_choice": 1,
  "secret": [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32],
  "merkle_root": [65,196,124,222,241,25,60,2,194,133,43,150,154,86,255,251,251,152,138,194,199,32,137,193,21,43,138,40,163,2,185,90],
  "merkle_siblings": [],
  "merkle_path_index": 0
}
EOF

# 3. Generate the proof (produces a 64-byte journal: merkle_root + nullifier)
RISC0_DEV_MODE=1 cargo run --features zk -- zk generate \
  --circuit vote \
  --input vote_input.json \
  --output proof.bin

# 4. Verify the proof
RISC0_DEV_MODE=1 cargo run --features zk -- zk verify \
  --circuit vote \
  --proof proof.bin

# 5. (Optional) Assert specific public outputs
cat > expected.json << 'EOF'
{
  "merkle_root": "<64-char hex from step 3 output>",
  "nullifier": "<64-char hex from step 3 output>"
}
EOF
RISC0_DEV_MODE=1 cargo run --features zk -- zk verify \
  --circuit vote \
  --proof proof.bin \
  --public-input expected.json
```

#### End-to-end on-chain demo (RSK Testnet)

**Prerequisites**: RBTC on RSK Testnet, Foundry installed.

```bash
# 1. Set environment variables
export PRIVATE_KEY="your_private_key_hex_no_0x"

# 2. Deploy MockVerifier + PrivateVoting
#    MockVerifier always returns true (blocked on mainnet chain ID 30; testnet chain ID 31 is allowed)
forge create contracts/MockVerifier.sol:MockVerifier \
  --rpc-url https://public-node.testnet.rsk.co \
  --private-key $PRIVATE_KEY
# Note the deployed address as VERIFIER_ADDRESS

#    PrivateVoting: pass the VOTE_ID image ID (bytes32) + verifier address
#    VOTE_ID is the Risc0 Image ID, which can be found in the build artifacts
#    or by running `cargo run --features zk -- zk verify` with an existing proof.
forge create contracts/PrivateVoting.sol:PrivateVoting \
  --rpc-url https://public-node.testnet.rsk.co \
  --private-key $PRIVATE_KEY \
  --constructor-args <VOTE_IMAGE_ID_BYTES32> <VERIFIER_ADDRESS>
# Note the deployed address as CONTRACT_ADDRESS

# 3. Run the demo — creates a proposal, generates a proof, casts a vote on-chain
export CONTRACT_ADDRESS="<CONTRACT_ADDRESS from above>"
RISC0_DEV_MODE=1 cargo run --features zk --bin demo_voting

# 4. (Optional) Skip proposal creation and vote on an existing open proposal
export PROPOSAL_ID=0
RISC0_DEV_MODE=1 cargo run --features zk --bin demo_voting
```

The demo:
1. Builds a depth-0 Merkle tree for `voter_id=12345` (root = leaf hash); registers it via `createProposal()` + `openProposal()`
2. Generates a fresh 32-byte secret via CSPRNG
3. Proves SHA-256 Merkle membership + nullifier derivation inside the zkVM (completes in seconds)
4. Computes ElGamal ciphertext `(C1, C2)` in native Rust (milliseconds); validates the public key is on the BN254 curve and in the correct subgroup
5. Estimates gas dynamically (`eth_estimateGas` + 20% buffer); simulates before broadcasting
6. Submits the 64-byte journal + proof seal + ciphertext to `PrivateVoting.castVote()` on RSK Testnet
7. The contract: verifies the ZK proof, checks the journal merkle_root matches the proposal, marks the nullifier (double-vote prevention), and homomorphically adds `(C1, C2)` to the running tally

> **Proposal lifecycle**: Created → Open (voting active) → Closed → Tallied. Only the admin (deployer wallet) can advance through states. Anyone may close a proposal once its deadline has passed.

> **Note on verifiers**: `MockVerifier` always returns `true`. For production, deploy `RiscZeroGroth16Verifier` (from [risc0-ethereum](https://github.com/risc0/risc0-ethereum)) and pass its address as `_verifier`. Wrap proofs with Groth16 via `RISC0_PROVER=groth16` (requires `rzup install groth16`) to produce compact ~256-byte seals compatible with on-chain verification.

> **Risc0 v1.x limitation**: The ZK proof does not cryptographically bind the ElGamal ciphertext `(C1, C2)` to the proof — ElGamal is computed by the host, not inside the circuit. Risc0 v2.x adds a BN254 precompile that would allow full cryptographic binding.

---

### Circuit 2: Anonymous Transfer (Demonstration Only)

> [!WARNING]
> `sender_balance` is a **private input** supplied by the prover and is NOT cryptographically bound to any on-chain state (no Merkle root, no commitment scheme). This circuit is **demonstration only** — do not use it for production DeFi. A production implementation must bind the balance to an on-chain commitment that the verifier can check.

The transfer circuit proves that a sender knows a balance ≥ amount, without revealing the sender/receiver identities. A commitment anchors the specific transfer.

#### ZK journal layout (64 bytes)

```
bytes  0..32  amount     — u64 right-aligned in a bytes32 word (EVM convention)
bytes 32..64  commitment — SHA-256("transfer-commitment-v1" || sender_id_be || receiver_id_be || nonce_be)
```

#### Input JSON schema

```json
{
  "sender_balance": 1000,
  "amount": 100,
  "sender_id": 1,
  "receiver_id": 2,
  "nonce": 42
}
```

#### Generate and verify a transfer proof

```bash
cat > transfer_input.json << 'EOF'
{
  "sender_balance": 1000,
  "amount": 100,
  "sender_id": 1,
  "receiver_id": 2,
  "nonce": 42
}
EOF

cargo run --features zk -- zk generate \
  --circuit transfer \
  --input transfer_input.json \
  --output transfer_proof.bin

cargo run --features zk -- zk verify \
  --circuit transfer \
  --proof transfer_proof.bin
```

---

### On-chain Bridge (Not Yet Implemented)

```bash
cargo run --features zk -- zk bridge \
  --circuit vote \
  --proof proof.bin \
  --contract 0x<verifier-address>
# Returns: error: zk-bridge is not yet implemented
```

Full implementation requires deploying `RiscZeroGroth16Verifier` on RSK and wrapping proofs with Groth16 (`rzup install groth16`).

---

### Running ZK Tests

Both circuits use SHA-256 only in the guest. With `RISC0_DEV_MODE=1` (mock prover) all 19 tests complete in ~10 seconds.

```bash
# Fast: mock prover — executes guest, captures journal, skips STARK generation (~10s)
RISC0_DEV_MODE=1 cargo test --features zk

# Full STARK proofs — produces real receipts (use before production release)
# Run single-threaded (-j 1) to avoid resource contention between parallel provers
cargo test --features zk -- -j 1

# Build without ZK (verifies feature-gating compiles cleanly)
cargo build
```

---

## 🔍 Troubleshooting & Security

### Diagnostics
Run the diagnostic tool to check your configuration, API keys, and wallet status:
```bash
cargo run -- config doctor
```

### Security Measures
- **AES-256-GCM**: authenticated encryption for wallet files.
- **Memory Zeroization**: sensitive data (private keys) is wiped from memory using the `zeroize` crate.
- **Homomorphic Privacy**: vote choices are ElGamal-encrypted; the contract never sees individual choices.
- **32-byte nullifier secret**: brute-force infeasible (2²⁵⁶ keyspace); generated with CSPRNG.
- **Mock Guards**: `MockVerifier` reverts on RSK Mainnet (chain ID 30). Use `RiscZeroGroth16Verifier` for production.
- **CEI pattern**: `PrivateVoting.castVote()` marks the nullifier used before any external call.

## Resources
- [Rootstock Documentation](https://developers.rsk.co/)
- [Risc0 Documentation](https://dev.risczero.com/)
- [Issue Tracker](https://github.com/rsksmart/rsk-rust-cli/issues)

## Disclaimer
The software provided is offered "as is," without warranty of any kind. Use at your own risk. The ZK features are currently in a demonstration phase.
