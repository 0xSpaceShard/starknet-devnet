use std::str::FromStr;
use std::sync::{mpsc, Arc};

use starknet_rs_core::types::BlockId;
use starknet_rs_ff::FieldElement;
use starknet_rs_providers::jsonrpc::HttpTransport;
use starknet_rs_providers::{JsonRpcClient, Provider, ProviderError};
use url::Url;

use crate::error::{DevnetResult, Error, ForkedProviderError};

type ProviderResult<T> = Result<T, ProviderError>;

enum ForkedRequest {
    GetNonceAt(FieldElement, std::sync::mpsc::Sender<ProviderResult<FieldElement>>),
}

struct ForkedProvider {
    client: Arc<JsonRpcClient<HttpTransport>>,
    receiver: tokio::sync::mpsc::Receiver<ForkedRequest>,
}

impl ForkedProvider {
    fn new(fork_url: Url, rx: tokio::sync::mpsc::Receiver<ForkedRequest>) -> Self {
        let json_rpc_client = JsonRpcClient::new(HttpTransport::new(fork_url));

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
                    .await;

                sender.send(nonce).expect("unable to send result from get_nonce");
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ForkedOrigin {
    sender: tokio::sync::mpsc::Sender<ForkedRequest>,
}

impl ForkedOrigin {
    fn new(rpc_url: Url) -> Self {
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

    fn get_nonce(&self, address: FieldElement) -> DevnetResult<FieldElement> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .try_send(ForkedRequest::GetNonceAt(address, tx))
            .map_err(|err| ForkedProviderError::InfrastructureError(err.to_string()))?;
        let nonce_result =
            rx.recv().map_err(|err| ForkedProviderError::InfrastructureError(err.to_string()))?;

        nonce_result
            .map_err(|err| Error::ForkedProviderError(ForkedProviderError::ProviderError(err)))
    }
}
