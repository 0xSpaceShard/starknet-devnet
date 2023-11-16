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
use starknet_rs_core::types::{BlockId, MsgToL1};
use starknet_types::rpc::transactions::L1HandlerTransaction;

use crate::error::{DevnetResult, Error, MessagingError};
use crate::starknet::Starknet;
use crate::traits::HashIdentified;
use crate::StarknetBlock;

mod ethereum;
pub use ethereum::EthereumMessaging;

impl Starknet {
    /// Returns the url of the messaging node currently in used, or `None` otherwise.
    pub fn messaging_url(&self) -> Option<String> {
        self.messaging.as_ref().map(|m| m.node_url())
    }

    /// Configures the messaging from the given L1 node parameters.
    /// Calling this function multiple time will overwrite the previous
    /// configuration, if any.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The L1 node RPC URL.
    /// * `contract_address` - The messaging contract address deployed on L1 node.
    /// * `private_key` - Private key associated with an EOA account to send transactions.
    pub async fn configure_messaging(
        &mut self,
        rpc_url: &str,
        contract_address: Option<&str>,
        private_key: Option<&str>,
    ) -> DevnetResult<()> {
        self.messaging =
            Some(EthereumMessaging::new(rpc_url, contract_address, private_key).await?);

        Ok(())
    }

    /// Collects all messages found between
    /// `from` to the Latest Starknet block, including both blocks.
    ///
    /// # Arguments
    /// * `from` - The block id from which (and including which) the messages are collected.
    pub async fn collect_messages_to_l1(&self, from: BlockId) -> DevnetResult<Vec<MsgToL1>> {
        let mut messages = vec![];

        for block in self.blocks.get_blocks(Some(from), None)? {
            messages.extend(self.get_block_messages(block)?);
        }

        Ok(messages)
    }

    /// Collects and sends to L1 all messages found between
    /// `from` to the Latest Starknet block, including both blocks.
    ///
    /// # Arguments
    /// * `from` - The block id from which (and including which) the messages are collected.
    pub async fn collect_and_send_messages_to_l1(
        &self,
        from: BlockId,
    ) -> DevnetResult<Vec<MsgToL1>> {
        if self.messaging.is_none() {
            return Err(Error::MessagingError(MessagingError::NotConfigured));
        }

        let messaging = self.messaging.as_ref().unwrap();

        let mut messages = vec![];

        for block in self.blocks.get_blocks(Some(from), None)? {
            messages.extend(self.get_block_messages(block)?);
        }

        messaging.send_mock_messages(&messages).await?;

        Ok(messages)
    }

    /// Fetches all messages from L1 and executes them by executing a `L1HandlerTransaction`
    /// for each one of them.
    pub async fn fetch_and_execute_messages_to_l2(
        &mut self,
    ) -> DevnetResult<Vec<L1HandlerTransaction>> {
        if self.messaging.is_none() {
            return Err(Error::MessagingError(MessagingError::NotConfigured));
        }

        let chain_id = self.chain_id().to_felt();
        let messaging = self.messaging.as_mut().unwrap();

        let transactions = messaging.fetch_messages(chain_id).await?;

        for transaction in &transactions {
            self.add_l1_handler_transaction(transaction.clone())?;
        }

        Ok(transactions)
    }

    /// Collects all messages for all the transactions of the the given block.
    ///
    /// # Arguments
    ///
    /// * `block` - The block from which messages are collected.
    fn get_block_messages(&self, block: &StarknetBlock) -> DevnetResult<Vec<MsgToL1>> {
        let mut messages = vec![];

        block.get_transactions().iter().for_each(|transaction_hash| {
            if let Ok(transaction) =
                self.transactions.get_by_hash(*transaction_hash).ok_or(Error::NoTransaction)
            {
                messages.extend(transaction.get_l2_to_l1_messages())
            }
        });

        Ok(messages)
    }
}
