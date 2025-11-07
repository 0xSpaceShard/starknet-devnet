#![allow(clippy::expect_used)]
use std::collections::BTreeMap;
use std::str::FromStr;

use alloy::hex::ToHexExt;
use alloy::primitives::{Address, B256, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::{BlockNumberOrTag, Filter, Log};
use alloy::signers::Signer;
use alloy::signers::local::{LocalSignerError, PrivateKeySigner};
use alloy::sol;
use alloy::sol_types::SolEvent;
use alloy::transports::RpcError;
use starknet_rs_core::types::{Felt, Hash256};
use starknet_types::felt::felt_from_prefixed_hex;
use starknet_types::rpc::contract_address::ContractAddress;
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};
use tracing::{trace, warn};
use url::Url;

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

impl<T> From<RpcError<T>> for Error {
    fn from(e: RpcError<T>) -> Self {
        Error::MessagingError(MessagingError::AlloyError(format!(
            "RpcError: {:?}",
            e.as_error_resp()
        )))
    }
}

impl From<LocalSignerError> for Error {
    fn from(e: LocalSignerError) -> Self {
        Error::MessagingError(MessagingError::AlloyError(format!("LocalSignerError: {e}")))
    }
}

sol! {
    #[sol(rpc)]
    event LogMessageToL2(
        address indexed from_address,
        uint256 indexed to_address,
        uint256 indexed selector,
        uint256[] payload,
        uint256 nonce,
        uint256 fee
    );
}
sol! {
    #[sol(rpc)]
    MockStarknetMessaging,
    "contracts/l1-l2-artifacts/MockStarknetMessaging.json",
}

async fn assert_address_contains_any_code(
    provider: &dyn Provider,
    address: Address,
) -> DevnetResult<()> {
    let messaging_contract_code = provider.get_code_at(address).await.map_err(|e| {
        Error::MessagingError(MessagingError::AlloyError(format!(
            "Failed retrieving contract code at address {address}: {e}"
        )))
    })?;

    if messaging_contract_code.is_empty() {
        return Err(Error::MessagingError(MessagingError::AlloyError(format!(
            "The specified address ({address:#x}) contains no contract"
        ))));
    }

    Ok(())
}

#[derive(Clone)]
/// Ethereum related configuration and types.
pub struct EthereumMessaging {
    wallet: PrivateKeySigner,
    messaging_contract_address: Address,
    node_url: Url,
    /// This value must be dumped to avoid re-fetching already processed messages.
    pub(crate) last_fetched_block: u64,
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
    /// * `deployer_account_private_key` - The private key of the funded account on L1 node to
    pub async fn new(
        rpc_url: &str,
        contract_address: Option<&str>,
        deployer_account_private_key: Option<&str>,
    ) -> DevnetResult<EthereumMessaging> {
        let node_url: Url = rpc_url.parse().map_err(|e| {
            Error::MessagingError(MessagingError::AlloyError(format!(
                "Failed to parse RPC URL '{rpc_url}': {e}"
            )))
        })?;

        let provider = ProviderBuilder::new().connect_http(node_url.clone());

        let chain_id = provider.get_chain_id().await?;
        let block_number = provider.get_block_number().await?;
        let last_fetched_block = block_number.try_into().unwrap();

        let private_key = match deployer_account_private_key {
            Some(private_key) => private_key,
            None => ETH_ACCOUNT_DEFAULT.private_key,
        };

        let wallet = PrivateKeySigner::from_str(private_key)?.with_chain_id(chain_id.into());

        let mut ethereum = EthereumMessaging {
            wallet,
            messaging_contract_address: Address::ZERO,
            node_url,
            last_fetched_block,
        };

        if let Some(address) = contract_address {
            ethereum.messaging_contract_address = Address::from_str(address).map_err(|e| {
                Error::MessagingError(MessagingError::AlloyError(format!(
                    "Address {address} can't be parsed from string: {e}",
                )))
            })?;

            assert_address_contains_any_code(&provider, ethereum.messaging_contract_address)
                .await?;
        } else {
            let cancellation_delay_seconds = U256::from(60 * 60 * 24);
            ethereum.messaging_contract_address =
                ethereum.deploy_messaging_contract(cancellation_delay_seconds).await?;
        }

        Ok(ethereum)
    }
    /// Returns the url of the ethereum node currently in used.
    pub fn node_url(&self) -> String {
        self.node_url.to_string()
    }

    /// Returns address of the messaging contract on L1 node.
    pub fn messaging_contract_address(&self) -> Address {
        self.messaging_contract_address
    }

    /// Fetches all the messages that were not already fetched from the L1 node.
    pub async fn fetch_messages(&mut self) -> DevnetResult<Vec<MessageToL2>> {
        let provider =
            ProviderBuilder::new().wallet(self.wallet.clone()).connect_http(self.node_url.clone());
        let latest_block = provider.get_block_number().await?;
        let to_block = latest_block.try_into().unwrap();

        // +1 exclude the latest fetched block the last time this function was called.
        let from_block = self.last_fetched_block + 1;
        let mut messages = vec![];

        self.fetch_logs(from_block, to_block).await?.into_iter().for_each(
            |(block_number, block_logs)| {
                trace!(
                    "Converting {} logs of block {block_number} into MessageToL2",
                    block_logs.len(),
                );

                block_logs.into_iter().for_each(|log| match message_to_l2_from_log(log) {
                    Ok(m) => messages.push(m),
                    Err(e) => warn!("Log from L1 node cannot be converted to MessageToL2: {e}"),
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

        let provider =
            ProviderBuilder::new().wallet(self.wallet.clone()).connect_http(self.node_url.clone());
        let contract = MockStarknetMessaging::new(self.messaging_contract_address, provider);

        for message in messages {
            let message_hash = U256::from_be_bytes(*message.hash().as_bytes());
            trace!("Sending message to L1: [{:064x}]", message_hash);

            let from_address = felt_to_u256(message.from_address.into());
            let to_address = felt_to_u256(message.to_address.clone().into());
            let payload = message.payload.iter().map(|f| felt_to_u256(*f)).collect::<Vec<_>>();

            let tx = contract
                .mockSendMessageFromL2(from_address, to_address, payload)
                .send()
                .await
                .map_err(|e| {
                    Error::MessagingError(MessagingError::AlloyError(format!(
                        "Failed to send mock message from L2: {e}"
                    )))
                })?;
            // Wait for transaction receipt
            match tx.get_receipt().await {
                Ok(receipt) => trace!(
                    "Message {message_hash:064x} sent on L1 with transaction hash {:#x}",
                    receipt.transaction_hash
                ),
                Err(_) => {
                    return Err(Error::MessagingError(MessagingError::AlloyError(format!(
                        "No receipt found for the tx of message hash: {message_hash:064x}",
                    ))));
                }
            }
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
    ) -> DevnetResult<BTreeMap<u64, Vec<Log>>> {
        trace!("Fetching logs for blocks {} - {}.", from_block, to_block);

        let mut block_to_logs = BTreeMap::<u64, Vec<Log>>::new();

        let provider = ProviderBuilder::new().connect_http(self.node_url.clone());

        // `sendMessageToL2` topic.
        let log_msg_to_l2_topic =
            B256::from_str("0xdb80dd488acf86d17c747445b0eabb5d57c541d3bd7b6b87af987858e5066b2b")
                .map_err(|err| {
                    Error::MessagingError(MessagingError::ConversionError(err.to_string()))
                })?;

        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Number(from_block.into()))
            .to_block(BlockNumberOrTag::Number(to_block.into()))
            .address(self.messaging_contract_address)
            .event_signature(log_msg_to_l2_topic);

        let logs = provider.get_logs(&filter).await?;

        for log in logs {
            if let Some(block_number) = log.block_number {
                let block_number: u64 = block_number.try_into().unwrap();
                block_to_logs.entry(block_number).or_default().push(log);
            }
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
        let provider =
            ProviderBuilder::new().wallet(self.wallet.clone()).connect_http(self.node_url.clone());
        let contract = MockStarknetMessaging::deploy(provider, cancellation_delay_seconds)
            .await
            .map_err(|e| {
                Error::MessagingError(MessagingError::AlloyError(format!(
                    "Failed deploying MockStarknetMessaging contract: {e}"
                )))
            })?;

        Ok(contract.address().clone())
    }
}

/// Converts an ethereum log into a `MessageToL2`.
///
/// # Arguments
///
/// * `log` - The log to be converted.
pub fn message_to_l2_from_log(log: Log) -> DevnetResult<MessageToL2> {
    let l1_transaction_hash =
        log.transaction_hash.map(|h| Hash256::from_bytes(h.to_vec().try_into().unwrap()));

    let decoded = LogMessageToL2::decode_log(&log.inner).map_err(|e| {
        Error::MessagingError(MessagingError::AlloyError(format!("Log parsing failed {e}")))
    })?;

    let from_address = address_to_felt(&decoded.from_address)?;
    let contract_address = ContractAddress::new(u256_to_felt(&decoded.to_address)?)?;
    let entry_point_selector = u256_to_felt(&decoded.selector)?;
    let nonce = u256_to_felt(&decoded.nonce)?;
    let paid_fee_on_l1 = u256_to_felt(&decoded.fee)?;
    let payload = decoded.payload.iter().map(u256_to_felt).collect::<Result<_, _>>()?;

    Ok(MessageToL2 {
        l1_transaction_hash,
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
fn u256_to_felt(v: &U256) -> DevnetResult<Felt> {
    Ok(felt_from_prefixed_hex(&format!("0x{}", v.to_string()))?)
}

/// Converts an `Felt` into a `U256`.
///
/// # Arguments
///
/// * `f` - The `Felt` to be converted.
fn felt_to_u256(f: Felt) -> U256 {
    U256::from_be_bytes(f.to_bytes_be())
}

/// Converts an `Address` into a `Felt`.
///
/// # Arguments
///
/// * `address` - The `Address` to be converted.
fn address_to_felt(address: &Address) -> DevnetResult<Felt> {
    Ok(felt_from_prefixed_hex(&format!("0x{}", address.encode_hex()))?)
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

        let log = alloy::rpc::types::RawLog {
            address: alloy::primitives::Address::from_str(
                "0xde29d060D45901Fb19ED6C6e959EB22d8626708e",
            )
            .unwrap(),
            topics: vec![
                B256::from_str(
                    "0xdb80dd488acf86d17c747445b0eabb5d57c541d3bd7b6b87af987858e5066b2b",
                )
                .unwrap(),
                B256::from_str(from_address).unwrap(),
                B256::from_str(to_address).unwrap(),
                B256::from_str(selector).unwrap(),
            ],
            data: payload_buf.into(),
            ..Default::default()
        };

        let expected_message = MessageToL2 {
            l1_transaction_hash: None,
            l1_contract_address: ContractAddress::new(
                felt_from_prefixed_hex(from_address).unwrap(),
            )
            .unwrap(),
            l2_contract_address: ContractAddress::new(felt_from_prefixed_hex(to_address).unwrap())
                .unwrap(),
            entry_point_selector: felt_from_prefixed_hex(selector).unwrap(),
            payload,
            nonce: nonce.into(),
            paid_fee_on_l1: fee.into(),
        };

        let message = message_to_l2_from_log(log).unwrap();

        assert_eq!(message, expected_message);
    }
}
