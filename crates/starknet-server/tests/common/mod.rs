use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::{thread, time};

use hyper::{Client, StatusCode, Uri};
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::JsonRpcClient;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("No free ports")]
    NoFreePorts,
}

const HOST: &str = "localhost";
const MIN_PORT: u16 = 1025;
const MAX_PORT: u16 = 65_535;
const SEED: usize = 42;
const ACCOUNTS: usize = 3;

const MAX_RETRIES: usize = 10;

// with seed 42
pub const PREDEPLOYED_ACCOUNT_ADDRESS: &str =
    "0x34ba56f92265f0868c57d3fe72ecab144fc96f97954bbbc4252cef8e8a979ba";

fn get_free_port_listener() -> Result<u16, TestError> {
    for port in MIN_PORT..=MAX_PORT {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            return Ok(listener.local_addr().expect("No local addr").port());
        }
        // otherwise port is occupied
    }
    Err(TestError::NoFreePorts)
}

pub(crate) struct BackgroundDevnet {
    pub(crate) json_rpc_client: JsonRpcClient<HttpTransport>,
    process: Child,
}

impl BackgroundDevnet {
    /// Ensures the background instance spawns at a free port, checks at most `MAX_RETRIES` times
    pub(crate) async fn spawn() -> Self {
        let free_port = get_free_port_listener().expect("No free ports");

        let devnet_url = format!("http://{HOST}:{free_port}");
        let devnet_rpc_url = Url::parse(format!("{}/rpc", devnet_url.as_str()).as_str()).unwrap();
        let json_rpc_client = JsonRpcClient::new(HttpTransport::new(devnet_rpc_url));

        let process = Command::new("cargo")
                .arg("run")
                .arg("--")
                .arg("--seed")
                .arg(SEED.to_string())
                .arg("--accounts")
                .arg(ACCOUNTS.to_string())
                .arg("--port")
                .arg(free_port.to_string())
                .stdout(Stdio::piped()) // comment this out for complete devnet stdout
                .spawn()
                .expect("Could not start background devnet");

        let healthcheck_uri =
            format!("{}/is_alive", devnet_url.as_str()).as_str().parse::<Uri>().unwrap();

        let mut retries = 0;
        let http_client = Client::new();
        while retries < MAX_RETRIES {
            if let Ok(alive_resp) = http_client.get(healthcheck_uri.clone()).await {
                assert_eq!(alive_resp.status(), StatusCode::OK);
                println!("Spawned background devnet at port {free_port}");
                return BackgroundDevnet { json_rpc_client, process };
            }

            // otherwise there is an error, probably a ConnectError if Devnet is not yet up
            // so we retry after some sleep
            retries += 1;
            thread::sleep(time::Duration::from_millis(500));
        }

        panic!("Could not start Background Devnet");
    }
}

/// By implementing Drop, we ensure there are no zombie background Devnet processes
/// in case of an early test failure
impl Drop for BackgroundDevnet {
    fn drop(&mut self) {
        self.process.kill().expect("Cannot kill process");
    }
}
