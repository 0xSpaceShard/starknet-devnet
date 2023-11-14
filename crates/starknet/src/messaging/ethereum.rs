use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use ethers::prelude::*;
use ethers::providers::{Http, Provider, ProviderError};
use ethers::types::{Address, BlockNumber, Log};
use k256::ecdsa::SigningKey;
use starknet_rs_core::types::{FieldElement, Hash256, MsgToL1};
use starknet_types::felt::Felt;
use starknet_types::rpc::contract_address::ContractAddress;
use starknet_types::rpc::transactions::L1HandlerTransaction;
use tracing::{trace, warn};

use crate::error::{DevnetResult, Error, MessagingError};

pub struct EthDevnetAccount {
    pub address: &'static str,
    pub private_key: &'static str,
}

/// Default account 0 for most used ethereum devnets (at least hardhat and anvil).
/// Mnemonic: test test test test test test test test test test test junk
/// Derivation path: m/44'/60'/0'/0/
const ETH_ACCOUNT_DEFAULT: EthDevnetAccount = EthDevnetAccount {
    address: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
};

// The provided artifact must contain "abi" and "bytecode" objects.
//
// TODO: for now, the path is duplicated. Need to find the best way to
// include MockStarknetMessaging.json, even when tests are run like `cargo test -p starknet -j 6`,
// as the path is relative to the Cargo manifest.
abigen!(
    MockStarknetMessaging,
    "contracts/artifacts/MockStarknetMessaging.json",
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
        contract_address: Option<&str>,
        private_key: Option<&str>,
    ) -> DevnetResult<EthereumMessaging> {
        let provider = Provider::<Http>::try_from(rpc_url).map_err(|e| {
            Error::MessagingError(MessagingError::EthersError(format!(
                "Can't parse L1 node URL: {} ({})",
                rpc_url, e
            )))
        })?;

        let chain_id = provider.get_chainid().await?;

        let private_key = private_key.unwrap_or(ETH_ACCOUNT_DEFAULT.private_key);

        let wallet: LocalWallet =
            private_key.parse::<LocalWallet>()?.with_chain_id(chain_id.as_u32());

        let provider_signer = SignerMiddleware::new(provider.clone(), wallet);

        let mut ethereum = EthereumMessaging {
            provider: Arc::new(provider),
            provider_signer: Arc::new(provider_signer),
            messaging_contract_address: Address::zero(),
            last_fetched_block: 0,
        };

        if let Some(address) = contract_address {
            ethereum.messaging_contract_address = Address::from_str(address).map_err(|e| {
                Error::MessagingError(MessagingError::EthersError(format!(
                    "Address can't be parse from string: {} ({})",
                    address, e
                )))
            })?;
        } else {
            // TODO: this may come from the configuration.
            let cancellation_delay_seconds: U256 = (60 * 60 * 24).into();
            ethereum.messaging_contract_address =
                ethereum.deploy_messaging_contract(cancellation_delay_seconds).await?;
        }

        Ok(ethereum)
    }

    /// Returns the url of the ethereum node currently in used.
    pub fn node_url(&self) -> String {
        self.provider.url().to_string()
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
            let message_hash = U256::from_big_endian(message.hash().as_bytes());
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

    /// Mocks the consumption of a message on L1 by providing the message content.
    /// To compute the hash in the mocked function on L1, the whole message payload,
    /// emitter and receiver must be provided.
    /// Returns the message hash of the message that is expected to be consumed.
    ///
    /// # Arguments
    ///
    /// * `l1_contract_address` - The L1 contract address that should consume the message.
    /// * `l2_contract_address` - The L2 contract address that sent the message.
    /// * `payload` - The message payload.
    pub async fn consume_mock_message(&self, message: &MsgToL1) -> DevnetResult<Hash256> {
        let starknet_messaging = MockStarknetMessaging::new(
            self.messaging_contract_address,
            self.provider_signer.clone(),
        );

        let from_address = felt_rs_to_u256(&message.from_address)?;
        let to_address = felt_rs_to_u256(&message.to_address)?;
        let payload = felts_rs_to_u256s(&message.payload)?;
        let message_hash = message.hash();

        match starknet_messaging
            .mock_consume_message_from_l2(from_address, to_address, payload)
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
                    "Message {} consumed on L1 with transaction hash {:#x}",
                    message_hash,
                    receipt.transaction_hash,
                );
            }
            None => {
                warn!("No receipt for L1 transaction.");
                return Err(Error::MessagingError(MessagingError::EthersError(format!(
                    "No receipt for transaction to consume message on L1 with hash {}",
                    message_hash
                ))));
            }
        };

        Ok(message_hash)
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

    /// Deploys an instance of the `MockStarknetMessaging` contract and returns it's address.
    ///
    /// # Arguments
    ///
    /// * `cancellation_delay_seconds` - Cancellation delay in seconds passed to the contract's
    ///   constructor.
    pub async fn deploy_messaging_contract(
        &self,
        cancellation_delay_seconds: U256,
    ) -> DevnetResult<Address> {
        let contract =
            MockStarknetMessaging::deploy(self.provider_signer.clone(), cancellation_delay_seconds)
                .map_err(|e| {
                    Error::MessagingError(MessagingError::EthersError(format!(
                        "Error formatting messaging contract deploy request: {}",
                        e
                    )))
                })?
                .send()
                .await
                .map_err(|e| {
                    Error::MessagingError(MessagingError::EthersError(format!(
                        "Error deploying messaging contract: {}",
                        e
                    )))
                })?;

        Ok(contract.address())
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

#[cfg(test)]
mod tests {

    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_types::chain_id::ChainId;

    use super::*;

    #[test]
    fn l1_handler_tx_from_log_parse_ok() {
        let from_address = "0x000000000000000000000000be3C44c09bc1a3566F3e1CA12e5AbA0fA4Ca72Be";
        let to_address = "0x039dc79e64f4bb3289240f88e0bae7d21735bef0d1a51b2bf3c4730cb16983e1";
        let selector = "0x02f15cff7b0eed8b9beb162696cf4e3e0e35fa7032af69cd1b7d2ac67a13f40f";
        let nonce = 783082_u128;
        let fee = 30000_u128;

        // Payload two values: [1, 2].
        let payload_buf = hex::decode("000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000bf2ea0000000000000000000000000000000000000000000000000000000000007530000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002").unwrap();

        let calldata: Vec<Felt> =
            vec![Felt::from_prefixed_hex_str(from_address).unwrap(), 1.into(), 2.into()];

        let transaction_hash = Felt::from_prefixed_hex_str(
            "0x6182c63599a9638272f1ce5b5cadabece9c81c2d2b8f88ab7a294472b8fce8b",
        )
        .unwrap();

        let log = Log {
            address: H160::from_str("0xde29d060D45901Fb19ED6C6e959EB22d8626708e").unwrap(),
            topics: vec![
                H256::from_str(
                    "0xdb80dd488acf86d17c747445b0eabb5d57c541d3bd7b6b87af987858e5066b2b",
                )
                .unwrap(),
                H256::from_str(from_address).unwrap(),
                H256::from_str(to_address).unwrap(),
                H256::from_str(selector).unwrap(),
            ],
            data: payload_buf.into(),
            ..Default::default()
        };

        let chain_id = ChainId::Testnet.to_felt();

        let expected = L1HandlerTransaction {
            contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(to_address).unwrap(),
            )
            .unwrap(),
            entry_point_selector: Felt::from_prefixed_hex_str(selector).unwrap(),
            calldata,
            nonce: nonce.into(),
            paid_fee_on_l1: fee,
            transaction_hash,
            ..Default::default()
        };

        let transaction: L1HandlerTransaction =
            l1_handler_tx_from_log(log, chain_id).expect("bad log format");

        assert_eq!(transaction, expected);
    }

    #[test]
    fn compute_message_hash_ok() {
        let from_address = get_selector_from_name("from_address").unwrap();
        let to_address = get_selector_from_name("to_address").unwrap();
        let payload = vec![FieldElement::ONE, FieldElement::TWO];

        let message = MsgToL1 { from_address, to_address, payload };

        let expected_hash = U256::from_str_radix(
            "0x5ba1d2e131360f15e26dd4f6ff10550685611cc25f75e7950b704adb04b36162",
            16,
        )
        .unwrap();

        let hash = compute_message_hash(&message);

        assert_eq!(hash, expected_hash);
    }
}
