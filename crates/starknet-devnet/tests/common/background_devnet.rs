use std::collections::HashMap;
use std::fmt::LowerHex;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time;

use hyper::client::HttpConnector;
use hyper::http::request;
use hyper::{Body, Client, Response, StatusCode, Uri};
use lazy_static::lazy_static;
use serde_json::json;
use starknet_core::constants::ETH_ERC20_CONTRACT_ADDRESS;
use starknet_rs_core::types::{
    BlockId, BlockTag, BlockWithTxHashes, FieldElement, FunctionCall, MaybePendingBlockWithTxHashes,
};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_rs_signers::{LocalWallet, SigningKey};
use starknet_types::felt::Felt;
use starknet_types::num_bigint::BigUint;
use starknet_types::rpc::transaction_receipt::FeeUnit;
use tokio::sync::Mutex;
use url::Url;

use super::constants::{
    ACCOUNTS, CHAIN_ID_CLI_PARAM, HEALTHCHECK_PATH, HOST, MAX_PORT, MIN_PORT,
    PREDEPLOYED_ACCOUNT_INITIAL_BALANCE, RPC_PATH, SEED,
};
use super::errors::TestError;
use super::utils::ImpersonationAction;
use crate::common::utils::get_json_body;

lazy_static! {
    /// This is to prevent TOCTOU errors; i.e. one background devnet might find one
    /// port to be free, and while it's trying to start listening to it, another instance
    /// finds that it's free and tries occupying it
    /// Using the mutex in `get_free_port_listener` might be safer than using no mutex at all,
    /// but not sufficiently safe
    static ref BACKGROUND_DEVNET_MUTEX: Mutex<()> = Mutex::new(());
}

#[derive(Debug)]
pub struct BackgroundDevnet {
    pub http_client: Client<HttpConnector>,
    pub json_rpc_client: JsonRpcClient<HttpTransport>,
    pub process: Child,
    pub url: String,
    rpc_url: Url,
}

fn get_free_port() -> Result<u16, TestError> {
    for port in MIN_PORT..=MAX_PORT {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            return Ok(listener.local_addr().expect("No local addr").port());
        }
        // otherwise port is occupied
    }
    Err(TestError::NoFreePorts)
}

lazy_static! {
    static ref DEFAULT_CLI_MAP: HashMap<&'static str, String> = HashMap::from([
        ("--seed", SEED.to_string()),
        ("--accounts", ACCOUNTS.to_string()),
        ("--initial-balance", PREDEPLOYED_ACCOUNT_INITIAL_BALANCE.to_string()),
        ("--chain-id", CHAIN_ID_CLI_PARAM.to_string())
    ]);
}

impl BackgroundDevnet {
    /// Ensures the background instance spawns at a free port, checks at most `MAX_RETRIES`
    /// times
    #[allow(dead_code)] // dead_code needed to pass clippy
    pub(crate) async fn spawn() -> Result<Self, TestError> {
        BackgroundDevnet::spawn_with_additional_args(&[]).await
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

    pub(crate) async fn spawn_with_additional_args(args: &[&str]) -> Result<Self, TestError> {
        // we keep the reference, otherwise the mutex unlocks immediately
        let _mutex_guard = BACKGROUND_DEVNET_MUTEX.lock().await;

        let free_port = get_free_port().expect("No free ports");

        let devnet_url = format!("http://{HOST}:{free_port}");
        let devnet_rpc_url = Url::parse(format!("{}{RPC_PATH}", devnet_url.as_str()).as_str())?;
        let json_rpc_client = JsonRpcClient::new(HttpTransport::new(devnet_rpc_url.clone()));

        let process = Command::new("cargo")
                .arg("run")
                .arg("--release")
                .arg("--")
                .arg("--port")
                .arg(free_port.to_string())
                .args(Self::add_default_args(args))
                .stdout(Stdio::piped()) // comment this out for complete devnet stdout
                .spawn()
                .expect("Could not start background devnet");

        let healthcheck_uri =
            format!("{}{HEALTHCHECK_PATH}", devnet_url.as_str()).as_str().parse::<Uri>()?;

        let mut retries = 0;
        let max_retries = 30; // limit the number of times we check if devnet is spawned
        let http_client = Client::new();
        while retries < max_retries {
            if let Ok(alive_resp) = http_client.get(healthcheck_uri.clone()).await {
                assert_eq!(alive_resp.status(), StatusCode::OK);
                println!("Spawned background devnet at port {free_port}");
                return Ok(BackgroundDevnet {
                    http_client,
                    json_rpc_client,
                    process,
                    url: devnet_url,
                    rpc_url: devnet_rpc_url,
                });
            }

            // otherwise there is an error, probably a ConnectError if Devnet is not yet up
            // so we retry after some sleep
            retries += 1;
            tokio::time::sleep(time::Duration::from_millis(500)).await;
        }

        Err(TestError::DevnetNotStartable)
    }

    pub async fn post_json(
        &self,
        path: String,
        body: hyper::Body,
    ) -> Result<Response<hyper::Body>, hyper::Error> {
        let req = request::Request::builder()
            .method("POST")
            .uri(format!("{}{}", self.url.as_str(), path))
            .header("content-type", "application/json")
            .body(body)
            .unwrap();
        self.http_client.request(req).await
    }

    pub async fn send_custom_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let body_json = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": method,
            "params": params
        });

        let body = hyper::Body::from(body_json.to_string());
        let resp = self.post_json(RPC_PATH.into(), body).await.unwrap();
        get_json_body(resp).await
    }

    pub fn clone_provider(&self) -> JsonRpcClient<HttpTransport> {
        JsonRpcClient::new(HttpTransport::new(self.rpc_url.clone()))
    }

    pub async fn mint(&self, address: impl LowerHex, mint_amount: u128) -> FieldElement {
        let req_body = Body::from(
            json!({
                "address": format!("{address:#x}"),
                "amount": mint_amount
            })
            .to_string(),
        );

        let resp = self.post_json("/mint".into(), req_body).await.unwrap();
        let resp_status = resp.status();
        let resp_body = get_json_body(resp).await;
        assert_eq!(resp_status, StatusCode::OK, "Checking status of {resp_body:?}");

        FieldElement::from_hex_be(resp_body["tx_hash"].as_str().unwrap()).unwrap()
    }

    /// Get ETH balance at contract_address, as written in ERC20
    pub async fn get_balance_at_block(
        &self,
        address: &FieldElement,
        block_id: BlockId,
    ) -> Result<FieldElement, anyhow::Error> {
        let call = FunctionCall {
            contract_address: FieldElement::from_hex_be(ETH_ERC20_CONTRACT_ADDRESS).unwrap(),
            entry_point_selector: get_selector_from_name("balanceOf").unwrap(),
            calldata: vec![*address],
        };
        let balance_raw = self.json_rpc_client.call(call, block_id).await?;
        assert_eq!(balance_raw.len(), 2);
        let balance_low: BigUint = (Felt::from(*balance_raw.get(0).unwrap())).into();
        let balance_high: BigUint = (Felt::from(*balance_raw.get(1).unwrap())).into();
        let balance: BigUint = (balance_high << 128) + balance_low;
        Ok(FieldElement::from_byte_slice_be(&balance.to_bytes_be())?)
    }

    /// Get balance at contract_address, as written in the ERC20 contract corresponding to `unit`
    pub async fn get_balance(
        &self,
        address: &FieldElement,
        unit: FeeUnit,
    ) -> Result<FieldElement, anyhow::Error> {
        let params = format!("address={:#x}&unit={}", address, unit);

        let resp = self.get("/account_balance", Some(params)).await?;
        let json_resp = get_json_body(resp).await;

        let amount: BigUint = serde_json::from_value(json_resp["amount"].clone()).unwrap();
        let received_unit: FeeUnit = serde_json::from_value(json_resp["unit"].clone()).unwrap();
        assert_eq!(received_unit, unit);

        let felt_amount = FieldElement::from_hex_be(&amount.to_str_radix(16))?;
        Ok(felt_amount)
    }

    /// Performs GET request on devnet; path should have a leading slash
    pub async fn get(
        &self,
        path: &str,
        query: Option<String>,
    ) -> Result<Response<Body>, hyper::Error> {
        let uri = format!("{}{}?{}", self.url, path, query.unwrap_or("".into()));
        let response = self.http_client.get(uri.as_str().parse::<Uri>().unwrap()).await.unwrap();
        Ok(response)
    }

    /// This method returns the private key and the address of the first predeployed account
    pub async fn get_first_predeployed_account(&self) -> (LocalWallet, FieldElement) {
        let predeployed_accounts_response = self.get("/predeployed_accounts", None).await.unwrap();

        let predeployed_accounts_json = get_json_body(predeployed_accounts_response).await;
        let first_account = predeployed_accounts_json.as_array().unwrap().get(0).unwrap();

        let account_address =
            FieldElement::from_hex_be(first_account["address"].as_str().unwrap()).unwrap();
        let private_key =
            FieldElement::from_hex_be(first_account["private_key"].as_str().unwrap()).unwrap();

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key));

        (signer, account_address)
    }

    pub async fn restart(&self) -> Result<Response<Body>, hyper::Error> {
        self.post_json("/restart".into(), Body::empty()).await
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
    pub async fn create_block(&self) -> Result<FieldElement, anyhow::Error> {
        let block_creation_resp =
            self.post_json("/create_block".into(), Body::from(json!({}).to_string())).await?;

        let block_creation_resp_body = get_json_body(block_creation_resp).await;
        let block_hash_str = block_creation_resp_body["block_hash"].as_str().unwrap();
        Ok(FieldElement::from_hex_be(block_hash_str)?)
    }

    pub async fn get_latest_block_with_tx_hashes(
        &self,
    ) -> Result<BlockWithTxHashes, anyhow::Error> {
        match self.json_rpc_client.get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest)).await {
            Ok(MaybePendingBlockWithTxHashes::Block(b)) => Ok(b),
            other => Err(anyhow::format_err!("Got unexpected block: {other:?}")),
        }
    }

    pub async fn execute_impersonation_action(
        &self,
        action: &ImpersonationAction,
    ) -> Result<(), anyhow::Error> {
        let (method_name, params) = match action {
            ImpersonationAction::ImpersonateAccount(account) => (
                "devnet_impersonateAccount",
                json!({
                    "account_address": format!("0x{:x}", account)
                }),
            ),
            ImpersonationAction::StopImpersonatingAccount(account) => (
                "devnet_stopImpersonateAccount",
                json!({
                    "account_address": format!("0x{:x}", account)
                }),
            ),
            ImpersonationAction::AutoImpersonate => ("devnet_autoImpersonate", json!({})),
            ImpersonationAction::StopAutoImpersonate => ("devnet_stopAutoImpersonate", json!({})),
        };

        let result = self.send_custom_rpc(method_name, params).await;

        result
            .get("error")
            .and_then(|error_json| error_json.get("message"))
            .and_then(|message_json| message_json.as_str())
            .map_or(Ok(()), |message| Err(anyhow::Error::msg(message.to_string())))
    }
}

/// By implementing Drop, we ensure there are no zombie background Devnet processes
/// in case of an early test failure
impl Drop for BackgroundDevnet {
    fn drop(&mut self) {
        self.process.kill().expect("Cannot kill process");
    }
}
