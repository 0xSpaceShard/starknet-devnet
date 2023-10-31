use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use ethers::prelude::*;
use ethers::providers::{Http, Provider, ProviderError};
use ethers::types::{Address, BlockNumber, Log};
use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};
use starknet_rs_core::types::{FieldElement, MsgToL1};
use starknet_types::felt::Felt;
use starknet_types::rpc::contract_address::ContractAddress;
use starknet_types::rpc::transactions::L1HandlerTransaction;
use tracing::{trace, warn};

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
        Error::MessagingError(MessagingError::EthersError(format!("ProviderError: {}", e)))
    }
}

impl From<WalletError> for Error {
    fn from(e: WalletError) -> Self {
        Error::MessagingError(MessagingError::EthersError(format!("WalletError: {}", e)))
    }
}

/// Ethereum related configuration and types.
pub struct EthereumMessaging {
    provider: Arc<Provider<Http>>,
    provider_signer: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
    messaging_contract_address: Address,
    // Keep track of the sync.
    // TODO: Must be also dumped in the future to
    // avoid fetching already fetched messages. Or use the nonce instead
    // to not re-send already sent messages.
    last_fetched_block: u64,
    // TODO: add a message nonce verification too.
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
            last_fetched_block: 0,
        })
    }

    /// Fetches all the messages that were not already fetched from the L1 node.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain ID to include in the transaction hash computation.
    pub async fn fetch_messages(
        &mut self,
        chain_id: Felt,
    ) -> DevnetResult<Vec<L1HandlerTransaction>> {
        let chain_latest_block: u64 = self
            .provider
            .get_block_number()
            .await?
            .try_into()
            .expect("Can't convert latest block number into u64.");

        // +1 as the from_block counts as 1 block fetched.
        // let to_block = if from_block + max_blocks + 1 < chain_latest_block {
        //     from_block + max_blocks
        // } else {
        //     chain_latest_block
        // };

        // For now we fetch all the blocks, without attempting to limit
        // the number of block as the RPC of dev nodes are more permissive.
        let to_block = chain_latest_block;
        // +1 exclude the latest fetched block the last time this function was called.
        let from_block = self.last_fetched_block + 1;

        let mut l1_handler_txs = vec![];

        self.fetch_logs(from_block, to_block).await?.into_iter().for_each(
            |(block_number, block_logs)| {
                trace!(
                    "Converting logs of block {block_number} into L1HandlerTransaction ({} logs)",
                    block_logs.len(),
                );

                block_logs.into_iter().for_each(|log| {
                    if let Ok(tx) = l1_handler_tx_from_log(log, chain_id) {
                        l1_handler_txs.push(tx)
                    }
                })
            },
        );

        self.last_fetched_block = to_block;
        Ok(l1_handler_txs)
    }

    /// Sends the list of given messages to L1. The messages are sent to
    /// the mocked contract, `mockSendMessageFromL2` entrypoint.
    ///
    /// # Arguments
    ///
    /// * `messages` - The list of messages to be sent.
    pub async fn send_mock_messages(&self, messages: &[MsgToL1]) -> DevnetResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let starknet_messaging = MockStarknetMessaging::new(
            self.messaging_contract_address,
            self.provider_signer.clone(),
        );

        for message in messages {
            let message_hash = compute_message_hash(message);
            trace!("Sending message to L1: [{:064x}]", message_hash);

            let from_address = felt_rs_to_u256(&message.from_address)?;
            let to_address = felt_rs_to_u256(&message.to_address)?;
            let payload = felts_rs_to_u256s(&message.payload)?;

            match starknet_messaging
                .mock_send_message_from_l2(from_address, to_address, payload)
                .send()
                .await
                .map_err(|e| Error::MessagingError(MessagingError::EthersError(
                    format!("Error sending transaction on ethereum: {}", e)
                )))?
            // wait for the tx to be mined
                .await?
            {
                Some(receipt) => {
                    trace!(
                        "Message {:064x} sent on L1 with transaction hash {:#x}",
                        message_hash,
                        receipt.transaction_hash,
                    );
                }
                None => {
                    warn!("No receipt for L1 transaction.");
                    // TODO: do we want to stop here? Or continue to the next message?
                    // We may return the list of messages not processed to let the caller
                    // retry?
                }
            };
        }

        Ok(())
    }

    /// Fetches logs in the given block range and returns a `HashMap` with the list of logs for each
    /// block number.
    ///
    /// There is no pagination on ethereum, and no hard limit on block range.
    /// Fetching too much blocks may result in RPC request error.
    /// For this reason, the caller may wisely choose the range.
    ///
    /// # Arguments
    ///
    /// * `from_block` - The first (included) block of which logs must be fetched.
    /// * `to_block` - The last (included) block of which logs must be fetched.
    async fn fetch_logs(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> DevnetResult<HashMap<u64, Vec<Log>>> {
        trace!("Fetching logs for blocks {} - {}.", from_block, to_block);

        let mut block_to_logs: HashMap<u64, Vec<Log>> = HashMap::new();

        // `sendMessageToL2` topic.
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

/// Converts an ethereum log into a `L1HandlerTransaction`.
///
/// # Arguments
///
/// * `log` - The log to be converted.
/// * `chain_id` - The L1 node chain id.
fn l1_handler_tx_from_log(log: Log, chain_id: Felt) -> DevnetResult<L1HandlerTransaction> {
    let parsed_log = <LogMessageToL2 as EthLogDecode>::decode_log(&log.into()).map_err(|e| {
        Error::MessagingError(MessagingError::EthersError(format!("Log parsing failed {}", e)))
    })?;

    let from_address = address_to_felt_devnet(&parsed_log.from_address)?;
    let contract_address = ContractAddress::new(u256_to_felt_devnet(&parsed_log.to_address)?)?;
    let entry_point_selector = u256_to_felt_devnet(&parsed_log.selector)?;
    let nonce = u256_to_felt_devnet(&parsed_log.nonce)?;
    let paid_fee_on_l1: u128 = parsed_log.fee.try_into().map_err(|_| {
        Error::MessagingError(MessagingError::EthersError(format!(
            "Fee does not fit into u128 {}",
            parsed_log.fee
        )))
    })?;

    let mut calldata = vec![from_address];
    for u in parsed_log.payload {
        calldata.push(u256_to_felt_devnet(&u)?);
    }

    Ok(L1HandlerTransaction {
        contract_address,
        entry_point_selector,
        calldata,
        nonce,
        paid_fee_on_l1,
        ..Default::default()
    }
    .with_hash(chain_id))
}

/// Computes the hash of a `MsgToL1`.
/// TODO: this must be removed once https://github.com/xJonathanLEI/starknet-rs/pull/476
/// is merged.
///
/// # Arguments
///
/// * `message` - The message on which the hash must be computed.
fn compute_message_hash(message: &MsgToL1) -> U256 {
    let mut buf: Vec<u8> = vec![];
    buf.extend(message.from_address.to_bytes_be());
    buf.extend(message.to_address.to_bytes_be());
    buf.extend(FieldElement::from(message.payload.len()).to_bytes_be());
    message.payload.iter().for_each(|p| buf.extend(p.to_bytes_be()));

    let mut hasher = Keccak256::new();
    hasher.update(buf);
    let hash = hasher.finalize();
    let hash_bytes = hash.as_slice();
    U256::from_big_endian(hash_bytes)
}

/// Converts a vector of `FieldElement` to a vector of `U256`.
///
/// # Arguments
///
/// * `felts` - The `FieldElement`s to be converted.
fn felts_rs_to_u256s(felts: &[FieldElement]) -> DevnetResult<Vec<U256>> {
    let mut buf: Vec<U256> = vec![];

    felts.iter().for_each(|p| buf.extend(felt_rs_to_u256(p)));

    Ok(buf)
}

/// Converts a `FieldElement` to a `U256`.
///
/// # Arguments
///
/// * `felt` - The `FieldElement` to be converted.
fn felt_rs_to_u256(felt: &FieldElement) -> DevnetResult<U256> {
    U256::from_str_radix(format!("{:#064x}", felt).as_str(), 16).map_err(|_| {
        Error::MessagingError(MessagingError::EthersError(format!(
            "Error converting {} into U256",
            felt
        )))
    })
}

/// Converts an `U256` into a `Felt`.
///
/// # Arguments
///
/// * `v` - The `U256` to be converted.
fn u256_to_felt_devnet(v: &U256) -> DevnetResult<Felt> {
    Ok(Felt::from_prefixed_hex_str(format!("0x{:064x}", v).as_str())?)
}

/// Converts an `Address` into a `Felt`.
///
/// # Arguments
///
/// * `address` - The `Address` to be converted.
fn address_to_felt_devnet(address: &Address) -> DevnetResult<Felt> {
    Ok(Felt::from_prefixed_hex_str(format!("0x{:064x}", address).as_str())?)
}
