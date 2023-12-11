use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use ethers::prelude::*;
use ethers::providers::{Http, Provider, ProviderError};
use ethers::types::{Address, BlockNumber, Log};
use k256::ecdsa::SigningKey;
use starknet_types::felt::Felt;
use starknet_types::rpc::contract_address::ContractAddress;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use starknet_types::traits::ToHexString;
use tracing::{trace, warn};

use crate::error::{DevnetResult, Error, MessagingError};

pub struct EthDevnetAccount {
    pub address: &'static str,
    pub private_key: &'static str,
}

/// Default account 0 for most used ethereum devnets (at least hardhat and anvil).
/// Mnemonic: test test test test test test test test test test test junk
/// Derivation path: m/44'/60'/0'/0/
pub const ETH_ACCOUNT_DEFAULT: EthDevnetAccount = EthDevnetAccount {
    address: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
    private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
};

// The provided artifact must contain "abi" and "bytecode" objects.
// The config check is required as the macro expects a literal string,
// and the path of the file is relative to the $CARGO_MANIFEST_DIR
// which differs depending the configuration (workspace or crate level).
// For this reason, the current approach is having the
// "contracts/artifacts/MockStarknetMessaging.json" path being valid at the
// crate level, and the workspace level.
// This is due to the fact that depending on the cargo command being run, the
// cargo manifest directory is not the same.
// This limitation is induced by the fact that the artifact path must be
// a literal string.
mod abigen {
    use ethers::prelude::abigen;
    abigen!(
        MockStarknetMessaging,
        "contracts/artifacts/MockStarknetMessaging.json",
        event_derives(serde::Serialize, serde::Deserialize)
    );
}

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
    // This value must be dumped to avoid re-fetching already processed
    // messages.
    last_fetched_block: u64,
    // A nonce verification may be added, with a nonce counter here.
    // If so, it must be dumped too.
}

impl EthereumMessaging {
    /// Instantiates a new `EthereumMessaging`.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The L1 node RPC URL.
    /// * `contract_address` - The messaging contract address deployed on L1 node.
    pub async fn new(
        rpc_url: &str,
        contract_address: Option<&str>,
    ) -> DevnetResult<EthereumMessaging> {
        let provider = Provider::<Http>::try_from(rpc_url).map_err(|e| {
            Error::MessagingError(MessagingError::EthersError(format!(
                "Can't parse L1 node URL: {} ({})",
                rpc_url, e
            )))
        })?;

        let chain_id = provider.get_chainid().await?;

        let private_key = ETH_ACCOUNT_DEFAULT.private_key;

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
                    "Address can't be parsed from string: {} ({})",
                    address, e
                )))
            })?;
        } else {
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

    /// Returns address of the messaging contract on L1 node.
    pub fn messaging_contract_address(&self) -> Address {
        self.messaging_contract_address
    }

    /// Fetches all the messages that were not already fetched from the L1 node.
    pub async fn fetch_messages(&mut self) -> DevnetResult<Vec<MessageToL2>> {
        let chain_latest_block: u64 =
            self.provider.get_block_number().await?.try_into().map_err(|e| {
                Error::MessagingError(MessagingError::EthersError(format!(
                    "Can't convert ethereum latest block number into u64: {}",
                    e
                )))
            })?;

        // For now we fetch all the blocks, without attempting to limit
        // the number of block as the RPC of dev nodes are more permissive.
        let to_block = chain_latest_block;
        // +1 exclude the latest fetched block the last time this function was called.
        let from_block = self.last_fetched_block + 1;

        let mut messages = vec![];

        self.fetch_logs(from_block, to_block).await?.into_iter().for_each(
            |(block_number, block_logs)| {
                trace!(
                    "Converting logs of block {block_number} into MessageToL2 ({} logs)",
                    block_logs.len(),
                );

                block_logs.into_iter().for_each(|log| match message_to_l2_from_log(log) {
                    Ok(m) => messages.push(m),
                    Err(e) => {
                        warn!("Log from L1 node couldn't be converted to `MessageToL2`: {}", e)
                    }
                })
            },
        );

        self.last_fetched_block = to_block;
        Ok(messages)
    }

    /// Sends the list of given messages to L1. The messages are sent to
    /// the mocked contract, `mockSendMessageFromL2` entrypoint.
    ///
    /// # Arguments
    ///
    /// * `messages` - The list of messages to be sent.
    pub async fn send_mock_messages(&self, messages: &[MessageToL1]) -> DevnetResult<()> {
        if messages.is_empty() {
            return Ok(());
        }

        let starknet_messaging = abigen::MockStarknetMessaging::new(
            self.messaging_contract_address,
            self.provider_signer.clone(),
        );

        for message in messages {
            let message_hash = U256::from_big_endian(message.hash().as_bytes());
            trace!("Sending message to L1: [{:064x}]", message_hash);

            let from_address = felt_devnet_to_u256(&(message.from_address.into()))?;
            let to_address = felt_devnet_to_u256(&(message.to_address.clone().into()))?;
            let payload = felts_devnet_to_u256s(&message.payload)?;

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
                    return Err(Error::MessagingError(MessagingError::EthersError(format!(
                        "No receipt found for the tx of message hash: {:064x}",
                        message_hash
                    ))));
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
                .expect("Invalid MessageToL2 topic");

        let filters = Filter {
            block_option: FilterBlockOption::Range {
                from_block: Some(BlockNumber::Number(from_block.into())),
                to_block: Some(BlockNumber::Number(to_block.into())),
            },
            address: Some(ValueOrArray::Value(self.messaging_contract_address)),
            topics: [Some(ValueOrArray::Value(Some(log_msg_to_l2_topic))), None, None, None],
        };

        for log in self
            .provider
            .get_logs(&filters)
            .await?
            .into_iter()
            .filter(|log| log.block_number.is_some())
        {
            // Safe to unwrap, we filtered with `is_some()` only.
            let block_number = log.block_number.unwrap().try_into().map_err(|e| {
                Error::MessagingError(MessagingError::EthersError(format!(
                    "Ethereum block number into u64: {}",
                    e
                )))
            })?;

            block_to_logs
                .entry(block_number)
                .and_modify(|v| v.push(log.clone()))
                .or_insert(vec![log]);
        }

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
        // Default value from anvil and hardat multiplied by 20.
        let gas_price: U256 = 20000000000_u128.into();

        let contract = abigen::MockStarknetMessaging::deploy(
            self.provider_signer.clone(),
            cancellation_delay_seconds,
        )
        .map_err(|e| {
            Error::MessagingError(MessagingError::EthersError(format!(
                "Error formatting messaging contract deploy request: {}",
                e
            )))
        })?
        .gas_price(gas_price)
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

/// Converts an ethereum log into a `MessageToL2`.
///
/// # Arguments
///
/// * `log` - The log to be converted.
pub fn message_to_l2_from_log(log: Log) -> DevnetResult<MessageToL2> {
    let parsed_log = <LogMessageToL2 as EthLogDecode>::decode_log(&log.into()).map_err(|e| {
        Error::MessagingError(MessagingError::EthersError(format!("Log parsing failed {}", e)))
    })?;

    let from_address = address_to_felt_devnet(&parsed_log.from_address)?;
    let contract_address = ContractAddress::new(u256_to_felt_devnet(&parsed_log.to_address)?)?;
    let entry_point_selector = u256_to_felt_devnet(&parsed_log.selector)?;
    let nonce = u256_to_felt_devnet(&parsed_log.nonce)?;
    let paid_fee_on_l1 = u256_to_felt_devnet(&parsed_log.fee)?;

    let mut payload = vec![];
    for u in parsed_log.payload {
        payload.push(u256_to_felt_devnet(&u)?);
    }

    Ok(MessageToL2 {
        l2_contract_address: contract_address,
        entry_point_selector,
        l1_contract_address: ContractAddress::new(from_address)?,
        payload,
        paid_fee_on_l1,
        nonce,
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

/// Converts an `Felt` into a `U256`.
///
/// # Arguments
///
/// * `v` - The `Felt` to be converted.
fn felt_devnet_to_u256(v: &Felt) -> DevnetResult<U256> {
    Ok(U256::from_str_radix(v.to_nonprefixed_hex_str().as_str(), 16).map_err(|e| {
        MessagingError::EthersError(format!("Cant't convert Felt into U256: {}", e))
    })?)
}

/// Converts a vector of `Felt` to a vector of `U256`.
///
/// # Arguments
///
/// * `felts` - The `Felt`s to be converted.
fn felts_devnet_to_u256s(felts: &[Felt]) -> DevnetResult<Vec<U256>> {
    let mut buf: Vec<U256> = vec![];

    felts.iter().for_each(|p| buf.extend(felt_devnet_to_u256(p)));

    Ok(buf)
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
    use super::*;

    #[test]
    fn test_message_to_l2_from_log() {
        // Test based on Goerli tx hash:
        // 0x6182c63599a9638272f1ce5b5cadabece9c81c2d2b8f88ab7a294472b8fce8b

        let from_address = "0x000000000000000000000000be3C44c09bc1a3566F3e1CA12e5AbA0fA4Ca72Be";
        let to_address = "0x039dc79e64f4bb3289240f88e0bae7d21735bef0d1a51b2bf3c4730cb16983e1";
        let selector = "0x02f15cff7b0eed8b9beb162696cf4e3e0e35fa7032af69cd1b7d2ac67a13f40f";
        let nonce = 783082_u128;
        let fee = 30000_u128;

        // Payload two values: [1, 2].
        let payload_buf = hex::decode("000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000bf2ea0000000000000000000000000000000000000000000000000000000000007530000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002").unwrap();

        let payload: Vec<Felt> = vec![1.into(), 2.into()];

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

        let expected_message = MessageToL2 {
            l1_contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(from_address).unwrap(),
            )
            .unwrap(),
            l2_contract_address: ContractAddress::new(
                Felt::from_prefixed_hex_str(to_address).unwrap(),
            )
            .unwrap(),
            entry_point_selector: Felt::from_prefixed_hex_str(selector).unwrap(),
            payload,
            nonce: nonce.into(),
            paid_fee_on_l1: fee.into(),
        };

        let message = message_to_l2_from_log(log).unwrap();

        assert_eq!(message, expected_message);
    }
}
