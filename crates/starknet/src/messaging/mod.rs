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

use crate::error::{DevnetResult, Error};
use crate::starknet::Starknet;
use crate::traits::HashIdentified;
use crate::StarknetBlock;

mod ethereum;

impl Starknet {
    /// Collects and sends to L1 all messages found between
    /// `from` to the Latest Starknet block, including both blocks.
    ///
    /// # Arguments
    /// * `from` - The block id from which (and including which) the messages are collected.
    async fn collect_and_send_messages_to_l1(&self, from: BlockId) -> DevnetResult<()> {
        let mut messages = vec![];

        // TODO: check if it's the latest block we have as expected here
        // for the upper limit.
        for block in self.blocks.get_blocks(Some(from), None)? {
            messages.extend(self.get_block_messages(block)?);
        }

        // For each message -> send TX to L1 with ether-rs.

        Ok(())
    }

    // Same for fetch_and_execute_messages_from_l1(&self, ethereum_url? ethereum_from_block?).
    // gather them and add l1 handler tx already validated?

    /// Collects all messages for all the transactions of the the given block.
    ///
    /// # Arguments
    /// * `block` - The block from which messages are collected.
    fn get_block_messages(&self, block: &StarknetBlock) -> DevnetResult<Vec<MsgToL1>> {
        let mut messages = vec![];

        let transactions = block.get_transactions().iter().for_each(|transaction_hash| {
            let transaction = self
                .transactions
                .get_by_hash(*transaction_hash)
                .ok_or(Error::NoTransaction)
                .map(|transaction| messages.extend(transaction.get_l2_to_l1_messages()));
        });

        Ok(messages)
    }
}
