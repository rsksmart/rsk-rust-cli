use crate::types::wallet::WalletData;
use crate::utils::constants;
use crate::utils::helper::Config;
use anyhow::anyhow;
use ethers::types::{H256, U256};
use ethers::{
    contract::abigen, prelude::*, providers::Provider, signers::LocalWallet,
    types::transaction::eip2718::TypedTransaction,
};
use std::fs;
use std::sync::Arc;

abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function transfer(address recipient, uint256 amount) external returns (bool)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
    ]"#,
);

pub struct EthClient {
    provider: Arc<Provider<Http>>,
    wallet: Option<LocalWallet>,
}

impl EthClient {
    pub async fn new(config: &Config, cli_api_key: Option<String>) -> Result<Self, anyhow::Error> {
        // Load or update API key
        let wallet_file = constants::wallet_file_path();
        let mut wallet_data = if wallet_file.exists() {
            let data = fs::read_to_string(&wallet_file)?;
            serde_json::from_str::<WalletData>(&data)?
        } else {
            WalletData::new()
        };

        let _api_key = if let Some(key) = cli_api_key {
            wallet_data.api_key = Some(key.clone());
            fs::write(&wallet_file, serde_json::to_string_pretty(&wallet_data)?)?;
            Some(key)
        } else {
            wallet_data.api_key.clone()
        };

        let provider = Provider::<Http>::try_from(&config.network.rpc_url)
            .map_err(|e| anyhow!("Failed to connect to RPC: {}", e))?;
        let wallet = config
            .wallet
            .private_key
            .as_ref()
            .map(|key| {
                key.parse::<LocalWallet>()
                    .map_err(|e| anyhow!("Invalid private key: {}", e))
            })
            .transpose()?;
        Ok(Self {
            provider: Arc::new(provider),
            wallet,
        })
    }

    pub async fn get_balance(
        &self,
        address: &Address,
        token_address: &Option<Address>,
    ) -> Result<U256, anyhow::Error> {
        match token_address {
            Some(token_addr) => {
                let contract = IERC20::new(*token_addr, Arc::clone(&self.provider));
                contract
                    .balance_of(*address)
                    .call()
                    .await
                    .map_err(|e| anyhow!("Failed to get token balance: {}", e))
            }
            None => self
                .provider
                .get_balance(*address, None)
                .await
                .map_err(|e| anyhow!("Failed to get RBTC balance: {}", e)),
        }
    }

    pub async fn send_transaction(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
    ) -> Result<H256, anyhow::Error> {
        let wallet = self
            .wallet
            .as_ref()
            .ok_or_else(|| anyhow!("No wallet configured"))?;
        let nonce = self
            .provider
            .get_transaction_count(wallet.address(), None)
            .await
            .map_err(|e| anyhow!("Failed to get nonce: {}", e))?;
        let gas_price = self
            .provider
            .get_gas_price()
            .await
            .map_err(|e| anyhow!("Failed to get gas price: {}", e))?;
        let rbtc_balance = self
            .provider
            .get_balance(wallet.address(), None)
            .await
            .map_err(|e| anyhow!("Failed to get RBTC balance: {}", e))?;
        let estimated_gas_cost = gas_price * U256::from(100_000);
        if rbtc_balance < estimated_gas_cost {
            return Err(anyhow!("Insufficient RBTC for gas fees"));
        }
        let chain_id = self.provider.get_chainid().await?.as_u64();

        match token_address {
            Some(token_addr) => {
                let contract = IERC20::new(token_addr, Arc::clone(&self.provider));
                let token_balance = contract
                    .balance_of(wallet.address())
                    .call()
                    .await
                    .map_err(|e| anyhow!("Failed to get token balance: {}", e))?;
                if token_balance < amount {
                    return Err(anyhow!("Insufficient token balance"));
                }
                let data = contract
                    .transfer(to, amount)
                    .calldata()
                    .ok_or_else(|| anyhow!("Failed to encode transfer calldata"))?;
                let mut tx = TypedTransaction::Legacy(TransactionRequest {
                    to: Some(token_addr.into()),
                    from: Some(wallet.address()),
                    nonce: Some(nonce),
                    gas_price: Some(gas_price),
                    gas: None,
                    value: Some(U256::zero()),
                    data: Some(data),
                    chain_id: Some(chain_id.into()),
                    ..Default::default()
                });
                let gas_estimate = self
                    .provider
                    .estimate_gas(&tx, None)
                    .await
                    .map_err(|e| anyhow!("Failed to estimate gas for token transfer: {}", e))?;
                tx.set_gas(gas_estimate);
                let signature = wallet
                    .sign_transaction(&tx)
                    .await
                    .map_err(|e| anyhow!("Failed to sign transaction: {}", e))?;
                let raw_tx = tx.rlp_signed(&signature);
                let pending_tx = self
                    .provider
                    .send_raw_transaction(raw_tx)
                    .await
                    .map_err(|e| anyhow!("Failed to send token transaction: {}", e))?;
                Ok(pending_tx.tx_hash())
            }
            None => {
                if rbtc_balance < amount + estimated_gas_cost {
                    return Err(anyhow!("Insufficient RBTC for transfer and gas"));
                }
                let tx = TransactionRequest::new()
                    .to(to)
                    .value(amount)
                    .from(wallet.address())
                    .nonce(nonce)
                    .gas_price(gas_price)
                    .chain_id(chain_id);
                let gas_estimate = self
                    .provider
                    .estimate_gas(&tx.clone().into(), None)
                    .await
                    .map_err(|e| anyhow!("Failed to estimate gas for RBTC transfer: {}", e))?;
                let typed_tx: TypedTransaction = tx.gas(gas_estimate).into();
                let signature = wallet
                    .sign_transaction(&typed_tx)
                    .await
                    .map_err(|e| anyhow!("Failed to sign transaction: {}", e))?;
                let raw_tx = typed_tx.rlp_signed(&signature);
                let pending_tx = self
                    .provider
                    .send_raw_transaction(raw_tx)
                    .await
                    .map_err(|e| anyhow!("Failed to send RBTC transaction: {}", e))?;
                Ok(pending_tx.tx_hash())
            }
        }
    }

    /// Get transaction receipt by hash
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<TransactionReceipt, anyhow::Error> {
        self.provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| anyhow!("Failed to get transaction receipt: {}", e))
            .and_then(|receipt| receipt.ok_or_else(|| anyhow!("Transaction receipt not found")))
    }

    pub async fn get_token_info(
        &self,
        token_address: Address,
    ) -> Result<(u8, String), anyhow::Error> {
        let contract = IERC20::new(token_address, Arc::clone(&self.provider));
        let decimals = contract.decimals().call().await?;
        let symbol = contract.symbol().call().await?;
        Ok((decimals, symbol))
    }

    /// Get a reference to the underlying provider
    pub fn provider(&self) -> &Provider<Http> {
        &self.provider
    }

    pub async fn estimate_gas(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
    ) -> Result<U256, anyhow::Error> {
        match token_address {
            Some(token_addr) => {
                let contract = IERC20::new(token_addr, Arc::clone(&self.provider));
                let tx = contract.transfer(to, amount);
                tx.estimate_gas()
                    .await
                    .map_err(|e| anyhow!("Failed to estimate gas for token transfer: {}", e))
            }
            None => {
                let tx = TransactionRequest::new().to(to).value(amount);
                self.provider
                    .estimate_gas(&tx.into(), None)
                    .await
                    .map_err(|e| anyhow!("Failed to estimate gas for RBTC transfer: {}", e))
            }
        }
    }
}

/// Generate an explorer URL for a transaction hash
pub fn get_explorer_url(tx_hash: &str, is_testnet: bool) -> String {
    if is_testnet {
        format!("https://explorer.testnet.rsk.co/tx/{}", tx_hash)
    } else {
        format!("https://explorer.rsk.co/tx/{}", tx_hash)
    }
}
