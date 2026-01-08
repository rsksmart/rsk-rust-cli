[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/rsksmart/rsk-rust-cli/badge)](https://scorecard.dev/viewer/?uri=github.com/rsksmart/rsk-rust-cli)
[![CodeQL](https://github.com/rsksmart/rsk-rust-cli/workflows/CodeQL/badge.svg)](https://github.com/rsksmart/rsk-rust-cli/actions?query=workflow%3ACodeQL)

<img src="rootstock-logo.png" alt="RSK Logo" style="width:100%; height: auto;" />

# Rootstock Rust CLI

A command-line interface (CLI) tool written in Rust for interacting with the Rootstock (Rootstock) blockchain.  
This tool provides comprehensive functionality to connect to an Ethereum-compatible RPC endpoint and manage wallets, query balances, send transactions, and interact with the Rootstock network.

## Features

### Wallet Management

- Create new wallets with custom names
- Import existing wallets using private keys
- Switch between multiple wallets
- Backup and restore wallet data
- List all available wallets
- Rename or delete wallets

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

- Save frequently used addresses as contacts
- Add notes and tags to contacts
- Search through your contacts
- Update or remove existing contacts
- Import/export contact lists

### Network & Configuration

- Switch between Mainnet and Testnet
- Configure custom RPC endpoints
- Manage API keys with validation for Rootstock networks
- View network status and connection details
- Offline functionality for wallet and contact management

## 🛠️ Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Git](https://git-scm.com/)
- [Cargo](https://doc.rust-lang.org/cargo/) (comes with Rust)

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
   # For development
   cargo run
   
   # For production
   ./target/release/rsk-rust-cli
   ```

## ⚙️ Configuration

The wallet automatically handles configuration through an interactive setup wizard when you first run it. You can also manage your configuration at any time using the interactive menu:

## 🎮 Usage

### Interactive Mode

The wallet features an intuitive interactive interface. Simply run:

```bash
cargo run
```

### 📋 Individual Feature Demos

For detailed walkthroughs of specific features:

### First Time Setup

[![First Time Setup](https://asciinema.org/a/lbHMriiOt5ZNfpyP3LnjCRJ6e.svg)]( https://asciinema.org/a/lbHMriiOt5ZNfpyP3LnjCRJ6e)

### Wallet Features

Create Wallet, Importing Existing Wallet, List Wallets, Switch Wallets, Delete Wallet & Rename Wallet
[![Importing Existing Wallet](https://asciinema.org/a/BoPr2TFlggYem2b1zhhEzS52X.svg)](https://asciinema.org/a/BoPr2TFlggYem2b1zhhEzS52X)

### Check Balance of RBTC and ERC-20 Tokens

Check your RBTC and ERC-20 token balances (e.g., RIF).

[![Check Balance](https://asciinema.org/a/ChSOgxrI2VXuA5Yq6Q0Jgc2nI.svg)](https://asciinema.org/a/ChSOgxrI2VXuA5Yq6Q0Jgc2nI
)

### Send Funds

Transfer RBTC from one wallet to another (e.g., from personal to lock wallet).
[![Send Funds](https://asciinema.org/a/dZTXi9OFBD2npbMpjfA68INX3.svg)](https://asciinema.org/a/dZTXi9OFBD2npbMpjfA68INX3
)
[View example transaction on explorer](https://explorer.testnet.rootstock.io/tx/c77c5bf53d1492bcf88104a746ca7a21f65add08f92dc0f9cad35db0a97886b0
)

### Set API Key

Configure and validate your API key for accessing transaction history and status checking.
[![Set API Key](https://asciinema.org/a/LPdZYLNWcCK0feJyvcXDFZSXn.svg)](https://asciinema.org/a/LPdZYLNWcCK0feJyvcXDFZSXn)

### Transaction History(make sure alchemy api key is set)

[![Set API Key](https://asciinema.org/a/Sh7qW67bHkDz0KGSjiqBLc8JC.svg)](https://asciinema.org/a/Sh7qW67bHkDz0KGSjiqBLc8JC)

### Check Transaction Status

[![Check Transaction Status](https://asciinema.org/a/jwQkdPRDSaUx8WC61ZNb8RnJ5.svg)](https://asciinema.org/a/jwQkdPRDSaUx8WC61ZNb8RnJ5)

You can also view it at [View Transaction](https://explorer.testnet.rsk.co/tx/c77c5bf53d1492bcf88104a746ca7a21f65add08f92dc0f9cad35db0a97886b0)

### Bulk Transfer

Send multiple transactions at once.
[![Bulk Transfer](https://asciinema.org/a/AC39AUnt9U0ih5MDUWltPdJWV.svg)](https://asciinema.org/a/AC39AUnt9U0ih5MDUWltPdJWV)

 [View Transaction 1](https://explorer.testnet.rsk.co/tx/0xfeeb73fb060f59352ff0aa99373fb502edd9702d999e8043158db18eb969ffa3) , [View Transaction 2](https://explorer.testnet.rsk.co/tx/0xbe16c77a67ee99edd492c14ca5668402d2a6a21baa39bdd2bd582ad301086951)

### Token Management

Add, list, and delete tokens from your wallet.
[![Add Token](https://asciinema.org/a/H4vFOowEHUDBi9txo8PGntedp.svg)](https://asciinema.org/a/H4vFOowEHUDBi9txo8PGntedp)

### Contact

Add, list, and delete contacts from your address book.
[![Add Contact](https://asciinema.org/a/q7aRPz3nUxuwopSSUFTZlNmxj.svg)](https://asciinema.org/a/q7aRPz3nUxuwopSSUFTZlNmxj)

## 🔍 Troubleshooting

### Common Issues

#### Transaction Failures

- Ensure you have enough RBTC for gas fees
- Verify recipient address format (0x...)
- Check network compatibility (mainnet/testnet)

#### Connection Issues

- Verify your internet connection
- App automatically switches to offline mode when network unavailable
- Limited functionality available offline (wallet management, contacts, tokens)

#### API Key Issues

- Ensure API key is valid for the selected network (mainnet/testnet)
- Keys are validated against Rootstock RPC endpoints
- Invalid keys will show clear error messages

#### Wallet Issues

- Confirm wallet file exists at `~/.local/share/rsk-rust-cli/`
- Check file permissions if access is denied
- Ensure you're using the correct network (mainnet/testnet)


## Resources

- [Rootstock Documentation](https://developers.rsk.co/)
- [RSK Smart Contracts](https://developers.rsk.co/develop/)
- [RSK Developer Portal](https://developers.rsk.co/)
- [Issue Tracker](https://github.com/rsksmart/rsk-rust-cli/issues)

## Security

This application handles private keys and financial transactions. Security measures include:

- **Encryption**: AES-256-GCM encryption for private keys with authenticated encryption
- **File Security**: Secure file permissions (0o600) for wallet files
- **Password Security**: Strong password validation with complexity requirements
- **Memory Security**: Memory zeroization for sensitive data to prevent leaks
- **Input Validation**: Comprehensive input validation and sanitization
- **API Security**: API key validation against Rootstock networks
- **Offline Security**: Core wallet functions available without network exposure

### ⚠️ Command-Line Security Notice

When using commands that require passwords or private keys as arguments, be aware that:

- **Shell History**: These values may be stored in your shell history (`.bash_history`, `.zsh_history`, etc.)
- **Process Listings**: They may be visible in process listings (`ps aux`, `top`, etc.)
- **System Logs**: Some systems log command-line arguments

**Recommended Security Practices:**

1. **Use Interactive Mode**: The interactive mode prompts for sensitive data securely without exposing it in command history
2. **Clear Shell History**: After entering sensitive commands, clear your shell history:
   ```bash
   history -c  # Clear current session
   history -w  # Write empty history to file
   ```
3. **Use Environment Variables**: For automation, use environment variables instead of command-line arguments
4. **Avoid Production Use**: For production environments, use secure secret management systems

**Example - Secure vs Insecure:**

```bash
# ❌ INSECURE - Password visible in history
rsk-rust-cli wallet create myWallet myPassword123

# ✅ SECURE - Use interactive mode
cargo run
# Then select "Wallet Management" and follow prompts
```

**Dependency Status:** All security vulnerabilities resolved. Minor unmaintained dependency warnings (derivative, paste) are transitive dependencies via Alloy and pose no security risk.

## Contributing

We welcome contributions from the community. Please fork the repository and submit pull requests with your changes. Ensure your code adheres to the project's main objective.

## Support

For any questions or support, please open an issue on the repository or reach out to the maintainers.

# Disclaimer

The software provided in this GitHub repository is offered "as is," without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, and non-infringement.

- **Testing:** The software has not undergone testing of any kind, and its functionality, accuracy, reliability, and suitability for any purpose are not guaranteed.
- **Use at Your Own Risk:** The user assumes all risks associated with the use of this software. The author(s) of this software shall not be held liable for any damages, including but not limited to direct, indirect, incidental, special, consequential, or punitive damages arising out of the use of or inability to use this software, even if advised of the possibility of such damages.
- **No Liability:** The author(s) of this software are not liable for any loss or damage, including without limitation, any loss of profits, business interruption, loss of information or data, or other pecuniary loss arising out of the use of or inability to use this software.
- **Sole Responsibility:** The user acknowledges that they are solely responsible for the outcome of the use of this software, including any decisions made or actions taken based on the software's output or functionality.
- **No Endorsement:** Mention of any specific product, service, or organization does not constitute or imply endorsement by the author(s) of this software.
- **Modification and Distribution:** This software may be modified and distributed under the terms of the license provided with the software. By modifying or distributing this software, you agree to be bound by the terms of the license.
- **Assumption of Risk:** By using this software, the user acknowledges and agrees that they have read, understood, and accepted the terms of this disclaimer and assumes all risks associated with the use of this software.
