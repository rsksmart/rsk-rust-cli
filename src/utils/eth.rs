use crate::types::wallet::WalletData;
use crate::utils::constants;
use crate::utils::helper::Config;
use anyhow::anyhow;
use alloy::primitives::{Address, B256, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::signers::local::PrivateKeySigner;
use alloy::transports::http::{Client, Http};
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::sol;
use std::fs;
use std::sync::Arc;

// Define ERC20 interface using alloy's sol! macro
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract IERC20 {
        function balanceOf(address account) external view returns (uint256);
        function transfer(address recipient, uint256 amount) external returns (bool);
        function decimals() external view returns (uint8);
        function symbol() external view returns (string);
    }
}

pub struct EthClient {
    provider: Arc<RootProvider<Http<Client>>>,
    wallet: Option<PrivateKeySigner>,
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
            wallet_data.api_key = Some(crate::utils::secrets::SecretString::new(key.clone()));
            crate::utils::secure_fs::write_secure(&wallet_file, &serde_json::to_string_pretty(&wallet_data)?)?;
            Some(key)
        } else {
            wallet_data.api_key.as_ref().map(|k| k.expose().clone())
        };

        // Use the RPC URL from config (which defaults to public nodes)
        let provider = ProviderBuilder::new()
            .on_http(config.network.rpc_url.parse()?);
        let wallet = config
            .wallet
            .private_key
            .as_ref()
            .map(|key| {
                key.as_str().parse::<PrivateKeySigner>()
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
                let contract = IERC20::new(*token_addr, &self.provider);
                let balance = contract
                    .balanceOf(*address)
                    .call()
                    .await
                    .map_err(|e| anyhow!("Failed to get token balance: {}", e))?;
                Ok(balance._0)
            }
            None => self
                .provider
                .get_balance(*address)
                .await
                .map_err(|e| anyhow!("Failed to get RBTC balance: {}", e)),
        }
    }

    pub async fn send_transaction(
        &self,
        to: Address,
        amount: U256,
        token_address: Option<Address>,
    ) -> Result<B256, anyhow::Error> {
        let wallet = self
            .wallet
            .as_ref()
            .ok_or_else(|| anyhow!("No wallet configured"))?;
        let nonce = self
            .provider
            .get_transaction_count(wallet.address())
            .await
            .map_err(|e| anyhow!("Failed to get nonce: {}", e))?;
        let gas_price = self
            .provider
            .get_gas_price()
            .await
            .map_err(|e| anyhow!("Failed to get gas price: {}", e))?;
        let rbtc_balance = self
            .provider
            .get_balance(wallet.address())
            .await
            .map_err(|e| anyhow!("Failed to get RBTC balance: {}", e))?;
        let estimated_gas_cost = U256::from(gas_price) * U256::from(100_000);
        if rbtc_balance < estimated_gas_cost {
            let balance_rbtc = rbtc_balance.to::<u128>() as f64 / 1e18;
            let gas_cost_rbtc = estimated_gas_cost.to::<u128>() as f64 / 1e18;
            return Err(anyhow!(
                "Insufficient RBTC for gas fees. Balance: {:.6} RBTC, Required: {:.6} RBTC", 
                balance_rbtc, gas_cost_rbtc
            ));
        }
        let chain_id = self.provider.get_chain_id().await?;

        match token_address {
            Some(token_addr) => {
                let contract = IERC20::new(token_addr, &self.provider);
                let token_balance = contract
                    .balanceOf(wallet.address())
                    .call()
                    .await
                    .map_err(|e| anyhow!("Failed to get token balance: {}", e))?;
                if token_balance._0 < amount {
                    let balance_f64 = token_balance._0.to::<u128>() as f64 / 1e18;
                    let amount_f64 = amount.to::<u128>() as f64 / 1e18;
                    return Err(anyhow!(
                        "Insufficient token balance. Balance: {:.6}, Required: {:.6}", 
                        balance_f64, amount_f64
                    ));
                }
                
                use alloy::rpc::types::TransactionRequest;
                let call_data = contract.transfer(to, amount).calldata().clone();
                let tx = TransactionRequest::default()
                    .with_to(token_addr)
                    .with_from(wallet.address())
                    .with_nonce(nonce)
                    .with_gas_price(gas_price)
                    .with_value(U256::ZERO)
                    .with_input(call_data)
                    .with_chain_id(chain_id);
                
                let gas_estimate = self
                    .provider
                    .estimate_gas(&tx)
                    .await
                    .map_err(|e| anyhow!("Failed to estimate gas for token transfer: {}", e))?;
                
                let tx = tx.with_gas_limit(gas_estimate);
                
                // Build transaction for signing
                let tx_envelope = tx.build(&EthereumWallet::from(wallet.clone())).await?;
                
                let pending_tx = self
                    .provider
                    .send_tx_envelope(tx_envelope)
                    .await
                    .map_err(|e| anyhow!("Failed to send token transaction: {}", e))?;
                Ok(*pending_tx.tx_hash())
            }
            None => {
                if rbtc_balance < amount + estimated_gas_cost {
                    return Err(anyhow!("Insufficient RBTC for transfer and gas"));
                }
                
                use alloy::rpc::types::TransactionRequest;
                let tx = TransactionRequest::default()
                    .with_to(to)
                    .with_value(amount)
                    .with_from(wallet.address())
                    .with_nonce(nonce)
                    .with_gas_price(gas_price)
                    .with_chain_id(chain_id);
                
                let gas_estimate = self
                    .provider
                    .estimate_gas(&tx)
                    .await
                    .map_err(|e| anyhow!("Failed to estimate gas for RBTC transfer: {}", e))?;
                
                let tx = tx.with_gas_limit(gas_estimate);
                
                // Build transaction for signing
                let tx_envelope = tx.build(&EthereumWallet::from(wallet.clone())).await?;
                
                let pending_tx = self
                    .provider
                    .send_tx_envelope(tx_envelope)
                    .await
                    .map_err(|e| anyhow!("Failed to send RBTC transaction: {}", e))?;
                Ok(*pending_tx.tx_hash())
            }
        }
    }

    /// Get transaction receipt by hash
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: B256,
    ) -> Result<alloy::rpc::types::TransactionReceipt, anyhow::Error> {
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
        let contract = IERC20::new(token_address, &self.provider);
        let decimals = contract.decimals().call().await?._0;
        let symbol = contract.symbol().call().await?._0;
        Ok((decimals, symbol))
    }

    /// Get a reference to the underlying provider
    pub fn provider(&self) -> &RootProvider<Http<Client>> {
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
                let contract = IERC20::new(token_addr, &self.provider);
                let call = contract.transfer(to, amount);
                call.estimate_gas()
                    .await
                    .map(|gas| U256::from(gas))
                    .map_err(|e| anyhow!("Failed to estimate gas for token transfer: {}", e))
            }
            None => {
                use alloy::rpc::types::TransactionRequest;
                let tx = TransactionRequest::default()
                    .with_to(to)
                    .with_value(amount);
                self.provider
                    .estimate_gas(&tx)
                    .await
                    .map(U256::from)
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
