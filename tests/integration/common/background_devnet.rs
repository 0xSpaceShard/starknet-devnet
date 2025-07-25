use std::collections::HashMap;
use std::fmt::LowerHex;
use std::process::{Command, Output, Stdio};
use std::time;

use anyhow::anyhow;
use lazy_static::lazy_static;
use reqwest::{Client, StatusCode};
use serde_json::json;
use starknet_rs_core::types::{
    BlockId, BlockTag, BlockWithTxHashes, BlockWithTxs, Felt, FunctionCall,
    MaybePreConfirmedBlockWithTxHashes, MaybePreConfirmedBlockWithTxs,
    PreConfirmedBlockWithTxHashes, PreConfirmedBlockWithTxs,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_rs_signers::{LocalWallet, SigningKey};
use url::Url;

use super::constants::{
    ACCOUNTS, HEALTHCHECK_PATH, HOST, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE, RPC_PATH, SEED, WS_PATH,
};
use super::errors::{RpcError, TestError};
use super::reqwest_client::{PostReqwestSender, ReqwestClient};
use super::utils::{FeeUnit, ImpersonationAction, to_hex_felt};
use crate::common::background_server::get_acquired_port;
use crate::common::constants::{
    DEVNET_EXECUTABLE_BINARY_PATH, DEVNET_MANIFEST_PATH, STRK_ERC20_CONTRACT_ADDRESS,
};
use crate::common::safe_child::SafeChild;

#[derive(Debug)]
pub struct BackgroundDevnet {
    reqwest_client: ReqwestClient,
    pub json_rpc_client: JsonRpcClient<HttpTransport>,
    port: u16,
    pub process: SafeChild,
    pub url: String,
    rpc_url: Url,
}

lazy_static! {
    static ref DEFAULT_CLI_MAP: HashMap<&'static str, String> = HashMap::from([
        ("--seed", SEED.to_string()),
        ("--accounts", ACCOUNTS.to_string()),
        ("--initial-balance", PREDEPLOYED_ACCOUNT_INITIAL_BALANCE.to_string()),
        ("--port", 0.to_string()) // random port by default
    ]);
}

async fn wait_for_successful_response(
    client: &Client,
    healthcheck_url: &str,
    sleep_time: time::Duration,
    max_retries: usize,
) -> Result<(), anyhow::Error> {
    for _ in 0..max_retries {
        if let Ok(alive_resp) = client.get(healthcheck_url).send().await {
            let status = alive_resp.status();
            if status != StatusCode::OK {
                return Err(anyhow!("Server responded with: {status}"));
            }

            return Ok(());
        }

        tokio::time::sleep(sleep_time).await;
    }

    Err(anyhow!("Not responsive: {healthcheck_url}"))
}

impl BackgroundDevnet {
    pub(crate) async fn spawn() -> Result<Self, TestError> {
        BackgroundDevnet::spawn_with_additional_args(&[]).await
    }

    pub(crate) async fn spawn_forkable_devnet() -> Result<BackgroundDevnet, anyhow::Error> {
        let args = ["--state-archive-capacity", "full"];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await?;
        Ok(devnet)
    }

    pub fn reqwest_client(&self) -> &ReqwestClient {
        &self.reqwest_client
    }

    /// Takes specified args and adds default values for args that are missing
    fn add_default_args<'a>(specified_args: &[&'a str]) -> Vec<&'a str> {
        let mut specified_args_vec: Vec<&str> = specified_args.to_vec();
        let mut final_args: Vec<&str> = vec![];

        // Iterate through default args, and remove from specified args when found
        // That way in the end we can just append the non-removed args
        for (arg_name, default_value) in DEFAULT_CLI_MAP.iter() {
            let value =
                match specified_args_vec.iter().position(|arg_candidate| arg_candidate == arg_name)
                {
                    Some(pos) => {
                        // arg value comes after name
                        specified_args_vec.remove(pos);
                        specified_args_vec.remove(pos)
                    }
                    None => default_value,
                };
            final_args.push(arg_name);
            final_args.push(value);
        }

        // simply append those args that don't have an entry in DEFAULT_CLI_MAP
        final_args.append(&mut specified_args_vec);
        final_args
    }

    fn start_safe_process(args: &[&str]) -> Result<SafeChild, TestError> {
        // If not on CircleCI, first build the workspace with cargo. Then rely on the built binary.
        if std::env::var("CIRCLECI").is_err() {
            let Output { status, stderr, .. } = Command::new("cargo")
                .args(["build", "--release", "--manifest-path", DEVNET_MANIFEST_PATH])
                .stdout(Stdio::null())
                .output()
                .map_err(|err| {
                    TestError::DevnetNotStartable(format!("Error spawning build process {err:?}"))
                })?;
            if !status.success() {
                let stderr_str = String::from_utf8_lossy(&stderr);
                return Err(TestError::DevnetNotStartable(format!(
                    "Error during build process {stderr_str}"
                )));
            }
        }

        let process = Command::new(DEVNET_EXECUTABLE_BINARY_PATH)
            .args(Self::add_default_args(args))
            .stdout(Stdio::piped()) // comment this out for complete devnet stdout
            .spawn()
            .map_err(|e| TestError::DevnetNotStartable(format!("Spawning error: {e:?}")))?;

        Ok(SafeChild { process })
    }

    pub(crate) async fn spawn_with_additional_args(args: &[&str]) -> Result<Self, TestError> {
        let mut safe_process = Self::start_safe_process(args)?;

        let sleep_time = time::Duration::from_millis(500);
        let max_retries = 60;
        let port = get_acquired_port(&mut safe_process, sleep_time, max_retries)
            .await
            .map_err(|e| TestError::DevnetNotStartable(format!("Cannot determine port: {e:?}")))?;

        // now we know the port; check if it can be used to poll Devnet's endpoint
        let client = Client::new();
        let devnet_url = format!("http://{HOST}:{port}");
        let healthcheck_url = format!("{devnet_url}{HEALTHCHECK_PATH}").to_string();
        wait_for_successful_response(&client, &healthcheck_url, sleep_time, max_retries)
            .await
            .map_err(|e| TestError::DevnetNotStartable(format!("Server unresponsive: {e:?}")))?;
        println!("Spawned background devnet at {devnet_url}");

        let devnet_rpc_url = Url::parse(format!("{devnet_url}{RPC_PATH}").as_str())?;
        Ok(Self {
            reqwest_client: ReqwestClient::new(devnet_url.clone(), client),
            json_rpc_client: JsonRpcClient::new(HttpTransport::new(devnet_rpc_url.clone())),
            port,
            process: safe_process,
            url: devnet_url,
            rpc_url: devnet_rpc_url,
        })
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{HOST}:{}{WS_PATH}", self.port)
    }

    pub async fn send_custom_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RpcError> {
        let mut body_json = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": method,
        });

        if !params.is_null() {
            body_json["params"] = params;
        }

        // Convert HTTP error to RPC error; panic if not possible.
        let json_rpc_result: serde_json::Value =
            self.reqwest_client().post_json_async(RPC_PATH, body_json).await.map_err(|err| {
                let err_msg = err.error_message();

                if let Ok(rpc_error) = serde_json::from_str::<RpcError>(&err_msg) {
                    return rpc_error;
                };

                if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(&err_msg) {
                    if let Some(err_prop) = err_val.get("error").cloned() {
                        if let Ok(rpc_error) = serde_json::from_value::<RpcError>(err_prop) {
                            return rpc_error;
                        }
                    }
                }

                panic!("Cannot extract RPC error from: {err_msg}")
            })?;

        if let Some(result) = json_rpc_result.get("result") {
            Ok(result.clone())
        } else if let Some(error) = json_rpc_result.get("error") {
            Err(serde_json::from_value(error.clone()).unwrap())
        } else {
            Err(RpcError {
                code: -1,
                message: "Server responded with malformed response".into(),
                data: None,
            })
        }
    }

    pub fn clone_provider(&self) -> JsonRpcClient<HttpTransport> {
        JsonRpcClient::new(HttpTransport::new(self.rpc_url.clone()))
    }

    /// Mint some FRI at `address` and return the resulting transaction hash.
    pub async fn mint(&self, address: impl LowerHex, mint_amount: u128) -> Felt {
        self.mint_unit(address, mint_amount, FeeUnit::Fri).await
    }

    pub async fn mint_unit(
        &self,
        address: impl LowerHex,
        mint_amount: u128,
        unit: FeeUnit,
    ) -> Felt {
        let resp_body: serde_json::Value = self
            .send_custom_rpc(
                "devnet_mint",
                json!({
                    "address": format!("{address:#x}"),
                    "amount": mint_amount,
                    "unit": unit,
                }),
            )
            .await
            .unwrap();

        Felt::from_hex_unchecked(resp_body["tx_hash"].as_str().unwrap())
    }

    /// Get ETH balance at contract_address, as written in ERC20
    pub async fn get_balance_at_block(
        &self,
        address: &Felt,
        block_id: BlockId,
    ) -> Result<Felt, anyhow::Error> {
        let call = FunctionCall {
            contract_address: STRK_ERC20_CONTRACT_ADDRESS,
            entry_point_selector: get_selector_from_name("balanceOf").unwrap(),
            calldata: vec![*address],
        };
        let balance_raw = self.json_rpc_client.call(call, block_id).await?;
        assert_eq!(balance_raw.len(), 2);
        let balance_low = balance_raw.first().unwrap().to_biguint();
        let balance_high = balance_raw.last().unwrap().to_biguint();
        Ok(Felt::from((balance_high << 128) + balance_low))
    }

    /// Get balance at contract_address, as written in the ERC20 contract corresponding to `unit`
    /// from latest state
    pub async fn get_balance_latest(
        &self,
        address: &Felt,
        unit: FeeUnit,
    ) -> Result<Felt, anyhow::Error> {
        Self::get_balance_by_tag(self, address, unit, BlockTag::Latest).await
    }

    /// Get balance at contract_address, as written in the ERC20 contract corresponding to `unit`
    /// from pre-confirmed state or latest state
    pub async fn get_balance_by_tag(
        &self,
        address: &Felt,
        unit: FeeUnit,
        tag: BlockTag,
    ) -> Result<Felt, anyhow::Error> {
        let json_resp = self
            .send_custom_rpc(
                "devnet_getAccountBalance",
                json!({
                    "address": address,
                    "unit": unit,
                    "block_id": Self::tag_to_str(tag),
                }),
            )
            .await
            .unwrap();

        // response validity asserted in test_balance.rs::assert_balance_endpoint_response
        let amount_raw = json_resp["amount"].as_str().unwrap();
        Ok(Felt::from_dec_str(amount_raw)?)
    }

    fn tag_to_str(tag: BlockTag) -> &'static str {
        match tag {
            BlockTag::Latest => "latest",
            BlockTag::PreConfirmed => "pre_confirmed",
            BlockTag::L1Accepted => "l1_accepted",
        }
    }

    /// This method returns the private key and the address of the first predeployed account
    pub async fn get_first_predeployed_account(&self) -> (LocalWallet, Felt) {
        let predeployed_accounts_json =
            self.send_custom_rpc("devnet_getPredeployedAccounts", json!({})).await.unwrap();

        let first_account = predeployed_accounts_json.as_array().unwrap().first().unwrap();

        let account_address = Felt::from_hex_unchecked(first_account["address"].as_str().unwrap());
        let private_key = Felt::from_hex_unchecked(first_account["private_key"].as_str().unwrap());

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        (signer, account_address)
    }

    pub async fn restart(&self) {
        self.send_custom_rpc("devnet_restart", json!({})).await.unwrap();
    }

    pub async fn fork(&self) -> Result<Self, TestError> {
        let args = ["--fork-network", self.url.as_str(), "--accounts", "0"];
        BackgroundDevnet::spawn_with_additional_args(&args).await
    }

    pub async fn fork_with_full_state_archive(&self) -> Result<Self, TestError> {
        let args = [
            "--fork-network",
            self.url.as_str(),
            "--accounts",
            "0",
            "--state-archive-capacity",
            "full",
        ];
        BackgroundDevnet::spawn_with_additional_args(&args).await
    }

    /// Mines a new block and returns its hash
    pub async fn create_block(&self) -> Result<Felt, anyhow::Error> {
        let block_creation_resp_body: serde_json::Value =
            self.send_custom_rpc("devnet_createBlock", json!({})).await.unwrap();

        let block_hash_str = block_creation_resp_body["block_hash"].as_str().unwrap();
        Ok(Felt::from_hex(block_hash_str)?)
    }

    pub async fn get_latest_block_with_tx_hashes(
        &self,
    ) -> Result<BlockWithTxHashes, anyhow::Error> {
        match self.json_rpc_client.get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest)).await {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(b)) => Ok(b),
            other => Err(anyhow::format_err!("Got unexpected block: {other:?}")),
        }
    }

    pub async fn get_pre_confirmed_block_with_tx_hashes(
        &self,
    ) -> Result<PreConfirmedBlockWithTxHashes, anyhow::Error> {
        match self
            .json_rpc_client
            .get_block_with_tx_hashes(BlockId::Tag(BlockTag::PreConfirmed))
            .await
        {
            Ok(MaybePreConfirmedBlockWithTxHashes::PreConfirmedBlock(b)) => Ok(b),
            other => Err(anyhow::format_err!("Got unexpected block: {other:?}")),
        }
    }

    pub async fn get_latest_block_with_txs(&self) -> Result<BlockWithTxs, anyhow::Error> {
        match self.json_rpc_client.get_block_with_txs(BlockId::Tag(BlockTag::Latest)).await {
            Ok(MaybePreConfirmedBlockWithTxs::Block(b)) => Ok(b),
            other => Err(anyhow::format_err!("Got unexpected block: {other:?}")),
        }
    }

    pub async fn get_confirmed_block_with_tx_hashes(
        &self,
        block_id: &BlockId,
    ) -> Result<BlockWithTxHashes, anyhow::Error> {
        match self.json_rpc_client.get_block_with_tx_hashes(block_id).await {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(block)) => Ok(block),
            other => Err(anyhow::format_err!("Got unexpected block response: {other:?}")),
        }
    }

    pub async fn get_pre_confirmed_block_with_txs(
        &self,
    ) -> Result<PreConfirmedBlockWithTxs, anyhow::Error> {
        match self.json_rpc_client.get_block_with_txs(BlockId::Tag(BlockTag::PreConfirmed)).await {
            Ok(MaybePreConfirmedBlockWithTxs::PreConfirmedBlock(b)) => Ok(b),
            other => Err(anyhow::format_err!("Got unexpected block: {other:?}")),
        }
    }

    pub async fn get_l1_accepted_block_with_tx_hashes(
        &self,
    ) -> Result<BlockWithTxHashes, anyhow::Error> {
        match self
            .json_rpc_client
            .get_block_with_tx_hashes(BlockId::Tag(BlockTag::L1Accepted))
            .await
        {
            Ok(MaybePreConfirmedBlockWithTxHashes::Block(b)) => Ok(b),
            Err(e) => Err(anyhow::Error::new(e)),
            Ok(other) => Err(anyhow::format_err!("Got unexpected block: {other:?}")),
        }
    }

    pub async fn abort_blocks(
        &self,
        starting_block_id: &BlockId,
    ) -> Result<Vec<Felt>, anyhow::Error> {
        let mut aborted_blocks = self
            .send_custom_rpc(
                "devnet_abortBlocks",
                json!({ "starting_block_id" : starting_block_id }),
            )
            .await
            .map_err(|e| anyhow::Error::msg(e.to_string()))?;

        let aborted_blocks = aborted_blocks["aborted"]
            .take()
            .as_array()
            .ok_or(anyhow::Error::msg("Invalid abort response"))?
            .clone();

        Ok(aborted_blocks
            .into_iter()
            .map(|block_hash| serde_json::from_value(block_hash).unwrap())
            .collect())
    }

    pub async fn accept_on_l1(&self, starting_block_id: &BlockId) -> Result<Vec<Felt>, RpcError> {
        let accepted_block_hashes_raw = self
            .send_custom_rpc(
                "devnet_acceptOnL1",
                json!({ "starting_block_id" : starting_block_id }),
            )
            .await?;

        let accepted_block_hashes =
            serde_json::from_value(accepted_block_hashes_raw["accepted"].clone()).unwrap();
        Ok(accepted_block_hashes)
    }

    pub async fn get_config(&self) -> serde_json::Value {
        self.send_custom_rpc("devnet_getConfig", json!({})).await.unwrap()
    }

    pub async fn execute_impersonation_action(
        &self,
        action: &ImpersonationAction,
    ) -> Result<(), anyhow::Error> {
        let (method_name, params) = match action {
            ImpersonationAction::ImpersonateAccount(account) => {
                ("devnet_impersonateAccount", json!({ "account_address": to_hex_felt(account) }))
            }
            ImpersonationAction::StopImpersonateAccount(account) => {
                ("devnet_stopImpersonateAccount", json!({"account_address": to_hex_felt(account)}))
            }
            ImpersonationAction::AutoImpersonate => ("devnet_autoImpersonate", json!({})),
            ImpersonationAction::StopAutoImpersonate => ("devnet_stopAutoImpersonate", json!({})),
        };

        let result = self.send_custom_rpc(method_name, params).await;

        match result {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::Error::msg(err.message.to_string())),
        }
    }
}
