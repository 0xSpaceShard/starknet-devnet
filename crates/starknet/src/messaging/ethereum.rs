use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use ethers::prelude::*;
use ethers::providers::{Http, Provider, ProviderError};
use ethers::types::{Address, BlockNumber, Log};
use k256::ecdsa::SigningKey;
use tracing::trace;

use crate::error::{DevnetResult, Error, MessagingError};

abigen!(
    MockStarknetMessaging,
    "contracts/MockStarknetMessaging.json",
    event_derives(serde::Serialize, serde::Deserialize)
);

#[derive(Debug, PartialEq, Eq, EthEvent)]
pub struct LogMessageToL2 {
    #[ethevent(indexed)]
    from_address: Address,
    #[ethevent(indexed)]
    to_address: U256,
    #[ethevent(indexed)]
    selector: U256,
    payload: Vec<U256>,
    nonce: U256,
    fee: U256,
}

impl From<ProviderError> for Error {
    fn from(e: ProviderError) -> Self {
        Error::MessagingError(MessagingError::EthersError(format!(
            "ProviderError: {}",
            e.to_string()
        )))
    }
}

impl From<WalletError> for Error {
    fn from(e: WalletError) -> Self {
        Error::MessagingError(MessagingError::EthersError(format!(
            "WalletError: {}",
            e.to_string()
        )))
    }
}

pub struct EthereumMessaging {
    provider: Arc<Provider<Http>>,
    provider_signer: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
    messaging_contract_address: Address,
}

impl EthereumMessaging {
    /// Instanciates a new `EthereumMessaging`.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The L1 node RPC URL.
    /// * `contract_address` - The messaging contract address deployed on L1 node.
    /// * `private_key` - Private key associated with an EOA account to send transactions.
    pub async fn new(
        rpc_url: &str,
        contract_address: &str,
        private_key: &str,
    ) -> DevnetResult<EthereumMessaging> {
        let provider = Provider::<Http>::try_from(rpc_url).map_err(|e| {
            Error::MessagingError(MessagingError::EthersError(format!(
                "Can't parse L1 node URL: {} ({})",
                rpc_url, e
            )))
        })?;

        let chain_id = provider.get_chainid().await?;

        let wallet: LocalWallet =
            private_key.parse::<LocalWallet>()?.with_chain_id(chain_id.as_u32());

        let provider_signer = SignerMiddleware::new(provider.clone(), wallet);
        let messaging_contract_address = Address::from_str(contract_address).map_err(|e| {
            Error::MessagingError(MessagingError::EthersError(format!(
                "Address can't be parse from string: {} ({})",
                contract_address, e
            )))
        })?;

        Ok(EthereumMessaging {
            provider: Arc::new(provider),
            provider_signer: Arc::new(provider_signer),
            messaging_contract_address,
        })
    }

    /// Fetches logs in the given block range and returns a `HashMap` with the list of logs for each block number.
    ///
    /// There is no pagination on ethereum, and no hard limit on block range.
    /// Fetching too much blocks may result in RPC request error.
    /// For this reason, the caller may wisely choose the range.
    ///
    /// # Arguments
    ///
    /// * `from_block` - The first (included) block of which logs must be fetched.
    /// * `to_block` - The last (included) block of which logs must be fetched.
    pub async fn fetch_logs(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> DevnetResult<HashMap<u64, Vec<Log>>> {
        trace!("Fetching logs for blocks {} - {}.", from_block, to_block);

        let mut block_to_logs: HashMap<u64, Vec<Log>> = HashMap::new();

        let log_msg_to_l2_topic =
            H256::from_str("0xdb80dd488acf86d17c747445b0eabb5d57c541d3bd7b6b87af987858e5066b2b")
                .unwrap();

        let filters = Filter {
            block_option: FilterBlockOption::Range {
                from_block: Some(BlockNumber::Number(from_block.into())),
                to_block: Some(BlockNumber::Number(to_block.into())),
            },
            address: Some(ValueOrArray::Value(self.messaging_contract_address)),
            topics: [Some(ValueOrArray::Value(Some(log_msg_to_l2_topic))), None, None, None],
        };

        self.provider
            .get_logs(&filters)
            .await?
            .into_iter()
            .filter(|log| log.block_number.is_some())
            .map(|log| {
                (
                    log.block_number
                        .unwrap()
                        .try_into()
                        .expect("Block number couldn't be converted to u64."),
                    log,
                )
            })
            .for_each(|(block_num, log)| {
                block_to_logs
                    .entry(block_num)
                    .and_modify(|v| v.push(log.clone()))
                    .or_insert(vec![log]);
            });

        Ok(block_to_logs)
    }
}
