use std::process::Command;
use std::sync::Arc;
use std::time;

use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use ethers::types::Address;
use k256::ecdsa::SigningKey;
use rand::Rng;
use reqwest::StatusCode;

use super::constants::{DEFAULT_ETH_ACCOUNT_PRIVATE_KEY, HOST};
use super::errors::TestError;
use super::safe_child::SafeChild;

pub struct BackgroundAnvil {
    pub process: SafeChild,
    pub url: String,
    pub provider: Arc<Provider<Http>>,
    pub provider_signer: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
}

mod abigen {
    use ethers::prelude::abigen;
    abigen!(
        L1L2Example,
        "../../contracts/l1-l2-artifacts/L1L2Example.json",
        event_derives(serde::Serialize, serde::Deserialize)
    );
}

impl BackgroundAnvil {
    /// To avoid TOCTOU or binding issues, we try random ports and try to start
    /// Anvil on this port (as Anvil will actually open the socket right after binding).
    #[allow(dead_code)] // dead_code needed to pass clippy
    pub(crate) async fn spawn() -> Result<Self, TestError> {
        BackgroundAnvil::spawn_with_additional_args(&[]).await
    }

    pub(crate) async fn spawn_with_additional_args(args: &[&str]) -> Result<Self, TestError> {
        // Relies on `background_devnet::BackgroundDevnet` starting its check from smaller values
        // (1025). Relies on the probability of M simultaneously spawned Anvils occupying
        // different ports being fairly big (N*(N-1)*...*(N-M+1) / N**M; N=65_000-20_000+1)
        let port = rand::thread_rng().gen_range(20_000..=65_000);

        let process = Command::new("anvil")
            .arg("--port")
            .arg(port.to_string())
            .arg("--silent")
            .args(args)
            .spawn()
            .expect("Could not start background Anvil");
        let safe_process = SafeChild { process };

        let url = format!("http://{HOST}:{port}");
        let client = reqwest::Client::new();
        let max_retries = 30;
        for _ in 0..max_retries {
            if let Ok(anvil_block_rsp) = send_dummy_request(&client, &url).await {
                assert_eq!(anvil_block_rsp.status(), StatusCode::OK);
                println!("Spawned background anvil at {url}");

                let (provider, provider_signer) = setup_ethereum_provider(&url).await?;

                return Ok(Self { process: safe_process, url, provider, provider_signer });
            }

            tokio::time::sleep(time::Duration::from_millis(500)).await;
        }

        Err(TestError::AnvilNotStartable)
    }

    pub async fn deploy_l1l2_contract(
        &self,
        messaging_address: Address,
    ) -> Result<Address, TestError> {
        let contract = abigen::L1L2Example::deploy(
            self.provider_signer.clone(),
            messaging_address,
        )
            .map_err(|e| {
                TestError::EthersError(format!(
                    "Error formatting messaging contract deploy request: {e}"
                ))
            })?
        // Required by the new version of anvil, as default is no longer accepted.
        // We use here the default value from anvil and hardat multiplied by 2.
            .gas_price(2000000000)
            .send()
            .await
            .map_err(|e| {
                TestError::EthersError(format!(
                    "Error deploying messaging contract: {e}"
                ))
            })?;

        Ok(contract.address())
    }

    pub async fn get_balance_l1l2(&self, address: Address, user: U256) -> Result<U256, TestError> {
        let l1l2_contract = abigen::L1L2Example::new(address, self.provider.clone());

        l1l2_contract.get_balance(user).call().await.map_err(|e| {
            TestError::EthersError(format!("Error calling l1l2 contract on ethereum: {e}"))
        })
    }

    pub async fn withdraw_l1l2(
        &self,
        address: Address,
        account_address: U256,
        user: U256,
        amount: U256,
    ) -> Result<(), TestError> {
        let l1l2_contract = abigen::L1L2Example::new(address, self.provider_signer.clone());

        l1l2_contract
            .withdraw(account_address, user, amount)
            .send()
            .await
            .map_err(|e| {
                TestError::EthersError(format!(
                    "tx for withdrawing from l1-l2 contract on ethereum failed: {e}"
                ))
            })?
            .await
            .map_err(|e| {
                TestError::EthersError(format!(
                    "tx for withdrawing from l1-l2 contract on ethereum has no receipt: {e}"
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
        let l1l2_contract = abigen::L1L2Example::new(address, self.provider_signer.clone());

        // The minimum value for messaging is 30k gwei,
        // we multiplied by 10 here.
        let value: U256 = 300000000000000_u128.into();

        l1l2_contract
            .deposit(contract_address, user, amount)
            .value(value)
            .send()
            .await
            .map_err(|e| {
                TestError::EthersError(format!(
                    "tx for deposit l1l2 contract on ethereum failed: {e}"
                ))
            })?
            .await
            .map_err(|e| {
                TestError::EthersError(format!(
                    "tx for deposit l1l2 contract on ethereum has no receipt: {e}"
                ))
            })?;

        Ok(())
    }
}

async fn setup_ethereum_provider(
    rpc_url: &str,
) -> Result<
    (Arc<Provider<Http>>, Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>),
    TestError,
> {
    let provider = Provider::<Http>::try_from(rpc_url)
        .map_err(|e| TestError::EthersError(format!("Can't parse L1 node URL: {rpc_url} ({e})")))
        .map_err(|e| TestError::EthersError(e.to_string()))?;

    let chain_id =
        provider.get_chainid().await.map_err(|e| TestError::EthersError(e.to_string()))?;

    let wallet: LocalWallet = DEFAULT_ETH_ACCOUNT_PRIVATE_KEY
        .parse::<LocalWallet>()
        .map_err(|e| TestError::EthersError(e.to_string()))?
        .with_chain_id(chain_id.as_u32());

    let provider_signer = SignerMiddleware::new(provider.clone(), wallet);

    Ok((Arc::new(provider), Arc::new(provider_signer)))
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
