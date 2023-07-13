use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::{thread, time};

use hyper::client::HttpConnector;
use hyper::{Client, StatusCode, Uri};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TestError {
    #[error("Cannot start Devnet")]
    DevnetNotStartable,
    #[error("No free ports")]
    NoFreePorts,
}

const HOST: &str = "localhost";
const MIN_PORT: u16 = 1025;
const MAX_PORT: u16 = 65_535;
const SEED: usize = 42;
const ACCOUNTS: usize = 3;

const MAX_RETRIES: usize = 10;

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
    client: Client<HttpConnector>,
    process: Option<Child>,
}

impl BackgroundDevnet {
    pub(crate) fn new() -> Self {
        BackgroundDevnet { client: Client::new(), process: None }
    }

    /// Ensures the background instance spawns at a free port, checks at most `MAX_RETRIES` times
    pub(crate) async fn spawn(&mut self) -> Result<(), TestError> {
        let free_port = get_free_port_listener().expect("No free ports");

        self.process = Some(
            Command::new("cargo")
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
                .expect("Could not start background devnet"),
        );

        let healthcheck_uri = Uri::builder()
            .scheme("http")
            .authority(format!("{HOST}:{free_port}"))
            .path_and_query("/is_alive")
            .build()
            .expect("Cannot build URI");

        let mut retries = 0;
        while retries < MAX_RETRIES {
            if let Ok(alive_resp) = self.client.get(healthcheck_uri.clone()).await {
                assert_eq!(alive_resp.status(), StatusCode::OK);
                println!("Spawned background devnet at port {free_port}");
                return Ok(());
            }

            // otherwise there is an error, probably a ConnectError if Devnet is not yet up
            // so we retry after some sleep
            retries += 1;
            thread::sleep(time::Duration::from_millis(500));
        }

        Err(TestError::DevnetNotStartable)
    }
}

/// By implementing Drop, we ensure there are no zombie background Devnet processes
/// in case of an early test failure
impl Drop for BackgroundDevnet {
    fn drop(&mut self) {
        self.process.as_mut().expect("No process to kill").kill().expect("Cannot kill process");
    }
}
