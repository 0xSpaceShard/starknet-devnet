use std::process::Command;
use std::str::FromStr;
use std::time;

use alloy::eips::BlockId;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Block;
use alloy::signers::Signer;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use reqwest::StatusCode;
use url::Url;

use super::background_server::get_acquired_port;
use super::constants::{DEFAULT_ANVIL_MNEMONIC_PHRASE, DEFAULT_ETH_ACCOUNT_PRIVATE_KEY, HOST};
use super::errors::TestError;
use super::safe_child::SafeChild;

pub struct BackgroundAnvil {
    pub process: SafeChild,
    pub url: Url,
    pub provider_signer: PrivateKeySigner,
}

sol! {
        #[derive(Debug)]
        #[sol(rpc)]
        L1L2Example,
        "../../contracts/l1-l2-artifacts/L1L2Example.json"
}

impl BackgroundAnvil {
    pub(crate) async fn spawn() -> Result<Self, TestError> {
        Self::spawn_with_additional_args(&[]).await
    }

    pub(crate) async fn spawn_with_additional_args_and_custom_signer(
        args: &[&str],
        mnemonic_phrase: &str,
        private_key: &str,
    ) -> Result<Self, TestError> {
        let process = Command::new("anvil")
            .arg("--port")
            .arg("0")
            .arg("--mnemonic")
            .arg(mnemonic_phrase)
            .arg("--silent")
            .args(args)
            .spawn()
            .expect("Could not start background Anvil");
        let mut safe_process = SafeChild { process };

        let sleep_time = time::Duration::from_millis(500);
        let max_retries = 30;
        let port = get_acquired_port(&mut safe_process, sleep_time, max_retries)
            .await
            .map_err(|e| TestError::AnvilNotStartable(format!("Cannot determine port: {e:?}")))?;

        let url: Url = format!("http://{HOST}:{port}").parse()?;
        let client = reqwest::Client::new();
        for _ in 0..max_retries {
            if let Ok(anvil_block_rsp) = send_dummy_request(&client, url.as_str()).await {
                assert_eq!(anvil_block_rsp.status(), StatusCode::OK);
                println!("Spawned background anvil at {url}");

                let chain_id =
                    ProviderBuilder::new().connect_http(url.clone()).get_chain_id().await.map_err(
                        |e| TestError::AlloyError(format!("Failed to get chain id: {e}")),
                    )?;

                let provider_signer = PrivateKeySigner::from_str(private_key)
                    .map_err(|e| TestError::AlloyError(format!("Invalid private key: {e}")))?
                    .with_chain_id(chain_id.into());
                return Ok(Self { process: safe_process, url, provider_signer });
            }

            tokio::time::sleep(sleep_time).await;
        }

        Err(TestError::AnvilNotStartable("Not responsive for too long".into()))
    }

    /// Spawns an instance at random port. Assumes CLI args in `args` don't contain `--port` or
    /// mnemonic parameters. Uses the mnemonic phrase defined in constants.
    pub(crate) async fn spawn_with_additional_args(args: &[&str]) -> Result<Self, TestError> {
        Self::spawn_with_additional_args_and_custom_signer(
            args,
            DEFAULT_ANVIL_MNEMONIC_PHRASE,
            DEFAULT_ETH_ACCOUNT_PRIVATE_KEY,
        )
        .await
    }

    pub async fn get_block(self, block: BlockId) -> Result<Block, TestError> {
        let provider = ProviderBuilder::new().connect_http(self.url.clone());

        provider
            .get_block(block)
            .await
            .map_err(|e| {
                TestError::AlloyError(format!(
                    "Error getting block from anvil at {}: {e}",
                    self.url
                ))
            })?
            .ok_or_else(|| TestError::AlloyError(format!("Block not found at {}", self.url)))
    }

    pub async fn deploy_l1l2_contract(
        &self,
        messaging_address: Address,
    ) -> Result<Address, TestError> {
        let provider = ProviderBuilder::new()
            .wallet(self.provider_signer.clone())
            .connect_http(self.url.clone());

        let contract = L1L2Example::deploy(provider, messaging_address).await.map_err(|e| {
            TestError::AlloyError(format!("Error deploying l1l2 contract on ethereum: {e}"))
        })?;

        Ok(*contract.address())
    }

    pub async fn get_balance_l1l2(&self, address: Address, user: U256) -> Result<U256, TestError> {
        let provider = ProviderBuilder::new().connect_http(self.url.clone());
        let contract = L1L2Example::new(address, provider);

        contract.get_balance(user).call().await.map_err(|e| {
            TestError::AlloyError(format!("Error calling l1l2 contract on ethereum: {e}"))
        })
    }

    pub async fn withdraw_l1l2(
        &self,
        address: Address,
        account_address: U256,
        user: U256,
        amount: U256,
    ) -> Result<(), TestError> {
        let provider = ProviderBuilder::new()
            .wallet(self.provider_signer.clone())
            .connect_http(self.url.clone());
        let contract = L1L2Example::new(address, provider);

        let _ = contract
            .withdraw(account_address, user, amount)
            .send()
            .await
            .map_err(|e| {
                TestError::AlloyError(format!(
                    "tx for withdrawing from l1-l2 contract on ethereum failed: {e}"
                ))
            })?
            .watch()
            .await
            .map_err(|e| {
                TestError::AlloyError(format!(
                    "Error confirming withdraw transaction from l1-l2 contract on ethereum: {e}"
                ))
            })?;

        Ok(())
    }

    pub async fn deposit_l1l2(
        &self,
        address: Address,
        contract_address: U256,
        user: U256,
        amount: U256,
    ) -> Result<(), TestError> {
        let provider = ProviderBuilder::new()
            .wallet(self.provider_signer.clone())
            .connect_http(self.url.clone());
        let contract = L1L2Example::new(address, provider);

        // The minimum value for messaging is 30k gwei,
        // we multiplied by 10 here.
        let value = U256::from(300000000000000_u128);

        let _ = contract
            .deposit(contract_address, user, amount)
            .value(value)
            .send()
            .await
            .map_err(|e| {
                TestError::AlloyError(format!(
                    "tx for deposit l1l2 contract on ethereum failed: {e}"
                ))
            })?
            .watch()
            .await
            .map_err(|e| {
                TestError::AlloyError(format!(
                    "Error confirming deposit l1l2 contract on ethereum: {e}"
                ))
            })?;
        Ok(())
    }
}

/// Even if the RPC method is dummy (doesn't exist),
/// the server is expected to respond properly if alive
async fn send_dummy_request(
    client: &reqwest::Client,
    rpc_url: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumberfuiorhgorueh",
            "params": [],
            "id": "1"
        }))
        .send()
        .await
}
