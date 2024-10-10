//! Messaging module.
//!
//! This module contains code related to messaging feature.
//! The messaging is composed of two major actors:
//!   1. The Starknet sequencer, which is in charge of gathering messages from L1 and executing
//!      them.
//!   2. The Starknet Core Contract, an Ethereum contract, that is generating the logs to send
//!      message to L2 and computing/ref-counting messages hashes for messages sent to L1.
//!
//! Being a devnet, this project is embedding an Ethereum contract (MockStarknetMessaging)
//! that mocks the behavior of the Starknet Core Contract by adding a method to manually
//! increment the ref-counting of message hashes.
//! This ensures that messages can be consumed on L1 without actually waiting for the
//! proof to be generated (at it is done on Starknet in production).
//!
//! # Receive message from L1
//! The Starknet sequencer (the devnet being the sequencer in this project)
//! is in charge of fetching the logs from Starknet Core Contract from Ethereum network.
//! In this project, the logs are emitted by the MockStarknetMessaging contract method
//! `sendMessageToL2`.
//! Once a log is gathered, a `L1HandlerTransaction` is executed internally, without
//! being signed by any account.
//!
//! # Send message to L1
//! To send messages to L1, any Starknet contract can use the `send_message_to_l1` syscall.
//! This will have the effect of adding, in the transaction output, the content
//! of the message.
//! By collecting those messages from transactions output, the devnet
//! uses the mocked functionality of manually incrementing the ref-count of a message
//! to make it available for consumption on L1.
//! This is done my sending a transaction to the Ethereum node, to the MockStarknetMessaging
//! contract (`mockSendMessageFromL2` entrypoint).
use std::collections::HashMap;

use ethers::types::H256;
use starknet_rs_core::types::{BlockId, ExecutionResult, Felt, Hash256};
use starknet_types::rpc::messaging::{MessageToL1, MessageToL2};

use crate::error::{DevnetResult, Error, MessagingError};
use crate::starknet::Starknet;
use crate::traits::HashIdentified;
use crate::StarknetBlock;

pub mod ethereum;
pub use ethereum::EthereumMessaging;

#[derive(Default)]
pub struct MessagingBroker {
    /// The ethereum broker to send transaction / call contracts using ethers.
    pub(crate) ethereum: Option<EthereumMessaging>,
    /// The last local (starknet) block for which messages have been collected
    /// and sent.
    pub last_local_block: u64,
    /// A local queue of `MessageToL1` hashes generated by cairo contracts.
    /// For each time a message is supposed to be sent to L1, it is stored in this
    /// queue. The user may consume those messages using `consume_message_from_l2`
    /// to actually test `MessageToL1` emitted without running L1 node.
    pub l2_to_l1_messages_hashes: HashMap<H256, u64>,
    /// This list of messages that will be sent to L1 node at the next `postman/flush`.
    pub l2_to_l1_messages_to_flush: Vec<MessageToL1>,
    /// Mapping of L1 transaction hash to a chronological sequence of generated L2 transactions.
    pub l1_to_l2_tx_hashes: HashMap<H256, Vec<Felt>>,
}

impl MessagingBroker {
    /// Configures the ethereum broker.
    ///
    /// # Arguments
    ///
    /// * `ethereum_messaging` - The `EthereumMessaging` to use as broker.
    pub fn configure_ethereum(&mut self, ethereum_messaging: EthereumMessaging) {
        self.ethereum = Some(ethereum_messaging);
    }

    /// Returns the url of the ethereum node currently in used, or `None` otherwise.
    pub fn ethereum_url(&self) -> Option<String> {
        self.ethereum.as_ref().map(|m| m.node_url())
    }

    /// Returns a reference to the ethereum instance if configured, an error otherwise.
    pub fn ethereum_ref(&self) -> DevnetResult<&EthereumMessaging> {
        self.ethereum.as_ref().ok_or(Error::MessagingError(MessagingError::NotConfigured))
    }

    /// Returns a mutable reference to the ethereum instance if configured, an error otherwise.
    pub fn ethereum_mut(&mut self) -> DevnetResult<&mut EthereumMessaging> {
        self.ethereum.as_mut().ok_or(Error::MessagingError(MessagingError::NotConfigured))
    }
}

impl Starknet {
    /// Configures the messaging from the given L1 node parameters.
    /// Calling this function multiple time will overwrite the previous
    /// configuration, if any.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The L1 node RPC URL.
    /// * `contract_address` - The messaging contract address deployed on L1 node.
    pub async fn configure_messaging(
        &mut self,
        rpc_url: &str,
        contract_address: Option<&str>,
    ) -> DevnetResult<String> {
        tracing::trace!("Configuring messaging: {}", rpc_url);

        self.messaging.configure_ethereum(EthereumMessaging::new(rpc_url, contract_address).await?);

        Ok(format!("0x{:x}", self.messaging.ethereum_ref()?.messaging_contract_address()))
    }

    /// Retrieves the ethereum node URL, if configured.
    pub fn get_ethereum_url(&self) -> Option<String> {
        self.messaging.ethereum_url()
    }

    /// Sets the latest local block processed by messaging.
    pub fn set_latest_local_block(&self) -> Option<String> {
        self.messaging.ethereum_url()
    }

    /// Collects all messages found between
    /// the current messaging latest block and the Latest Starknet block,
    /// including both blocks.
    /// This function register the messages in two fashions:
    /// 1. Add each message to the `l2_to_l1_messages_to_flush`.
    /// 2. Increment the counter for the hash of each message into `l2_to_l1_messages_hashes`.
    ///
    /// Returns all the messages currently collected and not flushed.
    pub async fn collect_messages_to_l1(&mut self) -> DevnetResult<Vec<MessageToL1>> {
        let from_block = self.messaging.last_local_block;

        match self.blocks.get_blocks(Some(BlockId::Number(from_block)), None) {
            Ok(blocks) => {
                let mut messages = vec![];

                let mut last_processed_block: u64 = 0;
                for block in blocks {
                    messages.extend(self.get_block_messages(block)?);
                    last_processed_block = block.header.block_number.0;
                }

                for message in &messages {
                    let hash = H256(*message.hash().as_bytes());
                    let count = self.messaging.l2_to_l1_messages_hashes.entry(hash).or_insert(0);
                    *count += 1;
                }

                // +1 to avoid latest block to be processed twice.
                self.messaging.last_local_block = last_processed_block + 1;

                self.messaging.l2_to_l1_messages_to_flush.extend(messages);

                Ok(self.messaging.l2_to_l1_messages_to_flush.clone())
            }
            Err(Error::NoBlock) => {
                // We're 1 block ahead of latest block, no messages can be collected.
                Ok(self.messaging.l2_to_l1_messages_to_flush.clone())
            }
            Err(e) => Err(e),
        }
    }

    /// Sends (flush) all the messages in `l2_to_l1_messages_to_flush` to L1 node.
    /// Returns the list of sent messages.
    pub async fn send_messages_to_l1(&mut self) -> DevnetResult<Vec<MessageToL1>> {
        let ethereum = self.messaging.ethereum_ref()?;
        ethereum.send_mock_messages(&self.messaging.l2_to_l1_messages_to_flush).await?;

        let messages = self.messaging.l2_to_l1_messages_to_flush.clone();
        self.messaging.l2_to_l1_messages_to_flush.clear();

        Ok(messages)
    }

    /// Consumes a `MessageToL1` that is registered in `l2_to_l1_messages`.
    /// If the count related to the message is hash is already 0, an error is returned,
    /// the message's hash otherwise.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to consume.
    pub async fn consume_l2_to_l1_message(
        &mut self,
        message: &MessageToL1,
    ) -> DevnetResult<Hash256> {
        // Ensure latest messages are collected before consuming the message.
        self.collect_messages_to_l1().await?;

        let hash = H256(*message.hash().as_bytes());
        let count = self.messaging.l2_to_l1_messages_hashes.entry(hash).or_insert(0);

        if *count > 0 {
            *count -= 1;
            Ok(message.hash())
        } else {
            Err(Error::MessagingError(MessagingError::MessageToL1NotPresent(hash.to_string())))
        }
    }

    /// Fetches all messages from L1 and converts the ethereum log into `MessageToL2`.
    pub async fn fetch_messages_to_l2(&mut self) -> DevnetResult<Vec<MessageToL2>> {
        let ethereum = self.messaging.ethereum_mut()?;
        let messages = ethereum.fetch_messages().await?;
        Ok(messages)
    }

    /// Collects all messages for all the transactions of the given block.
    ///
    /// # Arguments
    ///
    /// * `block` - The block from which messages are collected.
    fn get_block_messages(&self, block: &StarknetBlock) -> DevnetResult<Vec<MessageToL1>> {
        let mut messages = vec![];

        block.get_transactions().iter().for_each(|transaction_hash| {
            if let Ok(transaction) =
                self.transactions.get_by_hash(*transaction_hash).ok_or(Error::NoTransaction)
            {
                // As we will send the messages to L1 node, we don't want to include
                // the messages of reverted transactions.
                if let ExecutionResult::Succeeded = transaction.execution_result {
                    messages.extend(transaction.get_l2_to_l1_messages())
                }
            }
        });

        Ok(messages)
    }
}
