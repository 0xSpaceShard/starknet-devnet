use std::collections::HashMap;
use std::fmt::LowerHex;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::{thread, time};

use hyper::client::HttpConnector;
use hyper::http::request;
use hyper::{Body, Client, Response, StatusCode, Uri};
use lazy_static::lazy_static;
use rand::Rng;
use serde_json::json;
use starknet_core::constants::ERC20_CONTRACT_ADDRESS;
use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, FunctionCall};
use starknet_rs_core::utils::get_selector_from_name;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use starknet_rs_signers::{LocalWallet, SigningKey};
use starknet_types::felt::Felt;
use starknet_types::num_bigint::BigUint;
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("No free ports")]
    NoFreePorts,
    #[error("Could not parse URL")]
    UrlParseError(#[from] url::ParseError),
    #[error("Invalid URI")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),
    #[error("Could not start Anvil")]
    AnvilNotStartable,
}

pub struct BackgroundAnvil {
    pub process: Child,
    url: String,
}

impl BackgroundAnvil {
    /// To avoid TOCTOU or binding issues, we try random ports and try to start
    /// Anvil on this port (as Anvil will actually open the socket right after binding).
    pub(crate) async fn spawn() -> Result<Self, TestError> {
        use std::io::Write;

        let port = rand::thread_rng().gen_range(20_000..=65_000);

        let output = Command::new("pwd")
            .output()
            .expect("Could not echo");
        println!("--- {:?}", std::io::stdout().write_all(&output.stdout).unwrap());


        let process = Command::new("anvil")
            .arg("--port")
            .arg(port.to_string())
            .arg("--silent")
            .spawn()
            .expect("Could not start background Anvil");

        let anvil_url = format!("http://127.0.0.1:{port}");

        let mut retries = 0;
        let max_retries = 10;
        let http_client = Client::new();
        while retries < max_retries {
            if let Ok(anvil_block_rsp) = get_block_number(&anvil_url).await {
                assert_eq!(anvil_block_rsp.status(), StatusCode::OK);
                println!("{:?}", anvil_block_rsp);
                println!("Spawned background anvil at port {port} (127.0.0.1)");
                return Ok(BackgroundAnvil {
                    process,
                    url: anvil_url,
                });
            }

            retries += 1;
            thread::sleep(time::Duration::from_millis(500));
        }

        Err(TestError::AnvilNotStartable)
    }
}

pub async fn get_block_number(rpc_url: &str) -> Result<Response<hyper::Body>, hyper::Error> {
    let req = request::Request::builder()
        .method("POST")
        .uri(rpc_url)
        .header("content-type", "application/json")
        .body(r#"{
    "jsonrpc": "2.0",
    "method": "eth_blockNumberfuiorhgorueh",
    "params": []
    "id": "1"
}"#.into())
        .unwrap();

    let http_client = Client::new();
    http_client.request(req).await
}

/// By implementing Drop, we ensure there are no zombie background Anvil processes
/// in case of an early test failure
impl Drop for BackgroundAnvil {
    fn drop(&mut self) {
        self.process.kill().expect("Cannot kill process");
    }
}
