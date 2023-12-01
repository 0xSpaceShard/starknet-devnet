use std::process::{Child, Command};
use std::{thread, time};

use hyper::http::request;
use hyper::{Client, Response, StatusCode};
use rand::Rng;
use thiserror::Error;

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
    pub url: String,
}

impl BackgroundAnvil {
    /// To avoid TOCTOU or binding issues, we try random ports and try to start
    /// Anvil on this port (as Anvil will actually open the socket right after binding).
    #[allow(dead_code)] // dead_code needed to pass clippy
    pub(crate) async fn spawn() -> Result<Self, TestError> {
        let port = rand::thread_rng().gen_range(20_000..=65_000);

        let process = Command::new("anvil")
            .arg("--port")
            .arg(port.to_string())
            .arg("--silent")
            .spawn()
            .expect("Could not start background Anvil");

        let anvil_url = format!("http://127.0.0.1:{port}");

        let mut retries = 0;
        let max_retries = 10;

        while retries < max_retries {
            if let Ok(anvil_block_rsp) = get_block_number(&anvil_url).await {
                assert_eq!(anvil_block_rsp.status(), StatusCode::OK);
                println!("Spawned background anvil at port {port} (127.0.0.1)");
                return Ok(BackgroundAnvil { process, url: anvil_url });
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
        .body(
            r#"{
    "jsonrpc": "2.0",
    "method": "eth_blockNumberfuiorhgorueh",
    "params": []
    "id": "1"
}"#
            .into(),
        )
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
