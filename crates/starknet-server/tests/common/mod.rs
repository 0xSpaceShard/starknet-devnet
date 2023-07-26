pub mod constants {
    use starknet_core::constants::DEVNET_DEFAULT_INITIAL_BALANCE;
    use starknet_rs_core::types::FieldElement;

    pub const HOST: &str = "localhost";
    pub const MIN_PORT: u16 = 1025;
    pub const MAX_PORT: u16 = 65_535;
    pub const SEED: usize = 42;
    pub const ACCOUNTS: usize = 3;
    pub const CHAIN_ID: FieldElement = starknet_rs_core::chain_id::TESTNET;
    pub const CHAIN_ID_CLI_PARAM: &str = "TESTNET";

    // predeployed account info with seed=42
    pub const PREDEPLOYED_ACCOUNT_ADDRESS: &str =
        "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba";
    pub const PREDEPLOYED_ACCOUNT_PRIVATE_KEY: &str = "0xb137668388dbe9acdfa3bc734cc2c469";
    // half the default value - sanity check
    pub const PREDEPLOYED_ACCOUNT_INITIAL_BALANCE: u128 = DEVNET_DEFAULT_INITIAL_BALANCE / 2;
}

pub mod util {
    use std::fmt::LowerHex;
    use std::net::TcpListener;
    use std::process::{Child, Command, Stdio};
    use std::{thread, time};

    use hyper::client::HttpConnector;
    use hyper::http::request;
    use hyper::{Body, Client, Response, StatusCode, Uri};
    use lazy_static::lazy_static;
    use serde_json::json;
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::JsonRpcClient;
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use thiserror::Error;
    use tokio::sync::Mutex;
    use url::Url;

    use super::constants::{
        ACCOUNTS, CHAIN_ID_CLI_PARAM, HOST, MAX_PORT, MIN_PORT,
        PREDEPLOYED_ACCOUNT_ADDRESS, PREDEPLOYED_ACCOUNT_INITIAL_BALANCE,
        PREDEPLOYED_ACCOUNT_PRIVATE_KEY, SEED,
    };

    #[derive(Error, Debug)]
    pub enum TestError {
        #[error("No free ports")]
        NoFreePorts,

        #[error("Could not parse URL")]
        UrlParseError(#[from] url::ParseError),

        #[error("Invalid URI")]
        InvalidUri(#[from] hyper::http::uri::InvalidUri),

        #[error("Could not start Devnet")]
        DevnetNotStartable,
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

    pub async fn get_json_body(resp: Response<Body>) -> serde_json::Value {
        let resp_body = resp.into_body();
        let resp_body_bytes = hyper::body::to_bytes(resp_body).await.unwrap();
        serde_json::from_slice(&resp_body_bytes).unwrap()
    }

    /// Assumes Devnet has been run with the usual account seed and returns
    /// the signer and address of the 0th account
    pub fn get_predeployed_account_props() -> (LocalWallet, FieldElement) {
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_PRIVATE_KEY).unwrap(),
        ));
        let address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        (signer, address)
    }

    lazy_static! {
        /// This is to prevent TOCTOU errors; i.e. one background devnet might find one
        /// port to be free, and while it's trying to start listening to it, another instance
        /// finds that it's free and tries occupying it
        /// Using the mutex in `get_free_port_listener` might be safer than using no mutex at all,
        /// but not sufficiently safe
        static ref BACKGROUND_DEVNET_MUTEX: Mutex<()> = Mutex::new(());
    }

    pub struct BackgroundDevnet {
        pub http_client: Client<HttpConnector>,
        pub json_rpc_client: JsonRpcClient<HttpTransport>,
        process: Child,
        url: String,
        rpc_url: Url,
    }

    impl BackgroundDevnet {
        /// Ensures the background instance spawns at a free port, checks at most `MAX_RETRIES`
        /// times
        pub(crate) async fn spawn() -> Result<Self, TestError> {
            // we keep the reference, otherwise the mutex unlocks immediately
            let _mutex_guard = BACKGROUND_DEVNET_MUTEX.lock().await;

            let free_port = get_free_port().expect("No free ports");

            let devnet_url = format!("http://{HOST}:{free_port}");
            let devnet_rpc_url = Url::parse(format!("{}/rpc", devnet_url.as_str()).as_str())?;
            let json_rpc_client = JsonRpcClient::new(HttpTransport::new(devnet_rpc_url.clone()));

            let process = Command::new("cargo")
                .arg("run")
                .arg("--release")
                .arg("--")
                .arg("--seed")
                .arg(SEED.to_string())
                .arg("--accounts")
                .arg(ACCOUNTS.to_string())
                .arg("--port")
                .arg(free_port.to_string())
                .arg("--initial-balance")
                .arg(PREDEPLOYED_ACCOUNT_INITIAL_BALANCE.to_string())
                .arg("--chain-id")
                .arg(CHAIN_ID_CLI_PARAM)
                // .stdout(Stdio::piped()) // comment this out for complete devnet stdout
                .spawn()
                .expect("Could not start background devnet");

            let healthcheck_uri =
                format!("{}/is_alive", devnet_url.as_str()).as_str().parse::<Uri>()?;

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
                thread::sleep(time::Duration::from_millis(500));
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

        pub fn clone_provider(&self) -> JsonRpcClient<HttpTransport> {
            JsonRpcClient::new(HttpTransport::new(self.rpc_url.clone()))
        }

        pub async fn mint(&self, address: impl LowerHex, mint_amount: u128) {
            let req_body = Body::from(
                json!({
                    "address": format!("{address:#x}"),
                    "amount": mint_amount
                })
                .to_string(),
            );

            let resp = self.post_json("/mint".into(), req_body).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "Checking status of {resp:?}");
        }
    }

    /// By implementing Drop, we ensure there are no zombie background Devnet processes
    /// in case of an early test failure
    impl Drop for BackgroundDevnet {
        fn drop(&mut self) {
            self.process.kill().expect("Cannot kill process");
        }
    }
}
