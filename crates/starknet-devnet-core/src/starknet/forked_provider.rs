use std::str::FromStr;
use std::sync::{mpsc, Arc};

use starknet_rs_core::types::BlockId;
use starknet_rs_ff::FieldElement;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider};
use url::Url;

enum ForkedRequest {
    GetNonceAt(FieldElement, std::sync::mpsc::Sender<FieldElement>),
}

struct ForkedProvider {
    client: Arc<JsonRpcClient<HttpTransport>>,
    receiver: tokio::sync::mpsc::Receiver<ForkedRequest>,
}

impl ForkedProvider {
    fn new(fork_url: &str, rx: tokio::sync::mpsc::Receiver<ForkedRequest>) -> Self {
        let json_rpc_client =
            JsonRpcClient::new(HttpTransport::new(Url::from_str(fork_url).unwrap()));

        Self { client: Arc::new(json_rpc_client), receiver: rx }
    }

    async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            tokio::task::spawn(Self::handle_message(msg, self.client.clone()));
        }
    }

    async fn handle_message(msg: ForkedRequest, client: Arc<JsonRpcClient<HttpTransport>>) {
        match msg {
            ForkedRequest::GetNonceAt(address, sender) => {
                let nonce = client
                    .get_nonce(BlockId::Tag(starknet_rs_core::types::BlockTag::Latest), address)
                    .await
                    .unwrap();

                sender.send(nonce).unwrap();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ForkedOrigin {
    sender: tokio::sync::mpsc::Sender<ForkedRequest>,
}

impl ForkedOrigin {
    fn new(rpc_url: &str) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);
        let provider = ForkedProvider::new(rpc_url, receiver);

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime")
                .block_on(provider.run());
        });

        Self { sender }
    }

    fn get_nonce(&self, address: FieldElement) -> FieldElement {
        let (tx, rx) = mpsc::channel();
        self.sender.try_send(ForkedRequest::GetNonceAt(address, tx)).unwrap();
        rx.recv().unwrap()
    }
}
