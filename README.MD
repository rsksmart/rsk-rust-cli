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
- Manage API keys for services
- View network status and connection details

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

## Demo

### First Time Setup

[![First Time Setup](https://asciinema.org/a/V8Qf1AboSZu08l12KJFxTfyeV.svg)](https://asciinema.org/a/V8Qf1AboSZu08l12KJFxTfyeV)

### Importing Existing Wallet

[![Importing Existing Wallet](https://asciinema.org/a/cVrvEOP4LvJQLvTitxcayRBFt.svg)](https://asciinema.org/a/cVrvEOP4LvJQLvTitxcayRBFt)

### Wallet Creation

[![Wallet Creation](https://asciinema.org/a/Lj7YVm8idPbAEpHYPfjFuTfns.svg)](https://asciinema.org/a/Lj7YVm8idPbAEpHYPfjFuTfns)

### List Wallets & Switch Wallets

[![List Wallets](https://asciinema.org/a/HUpONPUC4OgQKFyStNm5KwaqT.svg)](https://asciinema.org/a/HUpONPUC4OgQKFyStNm5KwaqT)

### Delete Wallet & Rename Wallet

[![Delete Wallet](https://asciinema.org/a/cqjxdJjfaY1wqvO4upgRUMKDG.svg)](https://asciinema.org/a/cqjxdJjfaY1wqvO4upgRUMKDG)

### Check Balance of RBTC and ERC-20 Tokens

Check your RBTC and ERC-20 token balances (e.g., RIF).

[View example transaction on explorer](https://explorer.testnet.rootstock.io/tx/0x0293c59578303f3dc88daeda6c8564fd39b612dd85d7a1e025a37e611dc5b900)
[![Check Balance](https://asciinema.org/a/rqlxYIWZ2Gh1Pn5sEFZZFK5xz.svg)](https://asciinema.org/a/rqlxYIWZ2Gh1Pn5sEFZZFK5xz
)

### Send Funds

Transfer RBTC from one wallet to another (e.g., from personal to lock wallet).
[![Send Funds](https://asciinema.org/a/tEt4dSOgmEEguP6pgQ7setuPV.svg)](https://asciinema.org/a/tEt4dSOgmEEguP6pgQ7setuPV
)

### Set API Key

Configure your API key for accessing transaction history and status checking.
[![Set API Key](https://asciinema.org/a/6ZdWMvEMMZVsSCFkNHjq3MbPq.svg)](https://asciinema.org/a/6ZdWMvEMMZVsSCFkNHjq3MbPq)

### Transaction History

[![Set API Key](https://asciinema.org/a/Sh7qW67bHkDz0KGSjiqBLc8JC.svg)](https://asciinema.org/a/Sh7qW67bHkDz0KGSjiqBLc8JC)

### Check Transaction Status

[![Check Transaction Status](https://asciinema.org/a/CYYjrSV58KRGMgOmbdknm85Am.svg)](https://asciinema.org/a/CYYjrSV58KRGMgOmbdknm85Am)

You can also view it at [View Transaction](https://explorer.testnet.rsk.co/tx/0293c59578303f3dc88daeda6c8564fd39b612dd85d7a1e025a37e611dc5b900)

### Bulk Transfer

Send multiple transactions at once.
[![Bulk Transfer](https://asciinema.org/a/CX1fT6B9prX6Jjg652YWWrLrR.svg)](https://asciinema.org/a/CX1fT6B9prX6Jjg652YWWrLrR)

 [View Transaction 1](https://explorer.testnet.rsk.co/tx/87f26638a688477230855acc548595c6eb6baaf3fdb5ffba4d2b1cf788d2aaec) , [View Transaction 2](https://explorer.testnet.rsk.co/tx/de62ba82e458e52ae36f47750a86c73a9710ced3f24264a8e65780d42d77b72e)

### Token Management

Add, list, and delete tokens from your wallet.
[![Add Token](https://asciinema.org/a/dY7GpSsmk6uxsZdB9n6C2Fzgs.svg)](https://asciinema.org/a/dY7GpSsmk6uxsZdB9n6C2Fzgs)

### Contact

Add, list, and delete contacts from your address book.
[![Add Contact](https://asciinema.org/a/QaN1SZIXud4dxJ8woqDqnb1v5.svg)](https://asciinema.org/a/QaN1SZIXud4dxJ8woqDqnb1v5)

## 🔍 Troubleshooting

### Common Issues

#### Transaction Failures

- Ensure you have enough RBTC for gas fees
- Verify recipient address format (0x...)
- Check network compatibility (mainnet/testnet)

#### Connection Issues

- Verify your internet connection

#### Wallet Issues

- Confirm wallet file exists at `~/.local/share/rootstock-wallet/`
- Check file permissions if access is denied
- Ensure you're using the correct network (mainnet/testnet)


## Resources

- [Rootstock Documentation](https://developers.rsk.co/)
- [RSK Smart Contracts](https://developers.rsk.co/develop/)
- [RSK Developer Portal](https://developers.rsk.co/)
- [Issue Tracker](https://github.com/rsksmart/rsk-rust-cli/issues)

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
