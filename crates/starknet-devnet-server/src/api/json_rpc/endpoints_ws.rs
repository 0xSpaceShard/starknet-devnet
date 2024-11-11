use starknet_core::error::Error;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::starknet_api::block::BlockStatus;

use super::error::ApiError;
use super::models::{BlockInput, SubscriptionIdInput, TransactionBlockInput};
use super::{JsonRpcHandler, JsonRpcSubscriptionRequest};
use crate::rpc_core::request::Id;
use crate::subscribe::{NewTransactionStatus, SocketId, Subscription, SubscriptionNotification};
/// The definitions of JSON-RPC read endpoints defined in starknet_ws_api.json
impl JsonRpcHandler {
    pub async fn execute_ws(
        &self,
        request: JsonRpcSubscriptionRequest,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        match request {
            JsonRpcSubscriptionRequest::NewHeads(data) => {
                self.subscribe_new_heads(data, rpc_request_id, socket_id).await
            }
            JsonRpcSubscriptionRequest::TransactionStatus(data) => {
                self.subscribe_tx_status(data, rpc_request_id, socket_id).await
            }
            JsonRpcSubscriptionRequest::PendingTransactions => todo!(),
            JsonRpcSubscriptionRequest::Events => todo!(),
            JsonRpcSubscriptionRequest::Unsubscribe(SubscriptionIdInput { subscription_id }) => {
                let mut sockets = self.api.sockets.lock().await;
                let socket_context = sockets.get_mut(&socket_id).ok_or(
                    ApiError::StarknetDevnetError(Error::UnexpectedInternalError {
                        msg: format!("Unregistered socket ID: {socket_id}"),
                    }),
                )?;

                socket_context.unsubscribe(rpc_request_id, subscription_id).await?;
                Ok(())
            }
        }
    }

    /// Returns (starting block number, latest block number). Returns an error in case the starting
    /// block does not exist or there are too many blocks.
    async fn convert_to_block_number_range(
        &self,
        mut starting_block_id: BlockId,
    ) -> Result<(u64, u64), ApiError> {
        let starknet = self.api.starknet.lock().await;

        // convert pending to latest to prevent getting block_number = 0
        starting_block_id = match starting_block_id {
            BlockId::Tag(BlockTag::Pending) => BlockId::Tag(BlockTag::Latest),
            other => other,
        };

        // checking the block's existence; aborted blocks treated as not found
        let query_block = match starknet.get_block(&starting_block_id) {
            Ok(block) => match block.status() {
                BlockStatus::Rejected => Err(ApiError::BlockNotFound),
                _ => Ok(block),
            },
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(other) => Err(ApiError::StarknetDevnetError(other)),
        }?;

        let latest_block = starknet.get_block(&BlockId::Tag(BlockTag::Latest))?;

        let query_block_number = query_block.block_number().0;
        let latest_block_number = latest_block.block_number().0;

        let blocks_back_amount = if query_block_number > latest_block_number {
            0
        } else {
            latest_block_number - query_block_number
        };

        if blocks_back_amount > 1024 {
            return Err(ApiError::TooManyBlocksBack);
        }

        Ok((query_block_number, latest_block_number))
    }

    /// starknet_subscribeNewHeads
    /// Checks if an optional block ID is provided. Validates that the block exists and is not too
    /// many blocks in the past. If it is a valid block, the user is notified of all blocks from the
    /// old up to the latest, and subscribed to new ones. If no block ID specified, the user is just
    /// subscribed to new blocks.
    async fn subscribe_new_heads(
        &self,
        block_input: Option<BlockInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let block_id = if let Some(BlockInput { block }) = block_input {
            block.into()
        } else {
            // if no block ID input, this eventually just subscribes the user to new blocks
            BlockId::Tag(BlockTag::Latest)
        };

        let (query_block_number, latest_block_number) =
            self.convert_to_block_number_range(block_id).await?;

        // perform the actual subscription
        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id).ok_or(ApiError::StarknetDevnetError(
            Error::UnexpectedInternalError { msg: format!("Unregistered socket ID: {socket_id}") },
        ))?;
        let subscription_id =
            socket_context.subscribe(rpc_request_id, Subscription::NewHeads).await;

        if let BlockId::Tag(_) = block_id {
            // if the specified block ID is a tag (i.e. latest/pending), no old block handling
            return Ok(());
        }

        // Notifying of old blocks. latest_block_number inclusive?
        // Yes, only if block_id != latest/pending (handled above)
        let starknet = self.api.starknet.lock().await;
        for block_n in query_block_number..=latest_block_number {
            let old_block = starknet
                .get_block(&BlockId::Number(block_n))
                .map_err(ApiError::StarknetDevnetError)?;

            let old_header = old_block.into();
            let notification = SubscriptionNotification::NewHeadsNotification(Box::new(old_header));
            socket_context.notify(subscription_id, notification).await;
        }

        Ok(())
    }

    async fn subscribe_tx_status(
        &self,
        transaction_block_input: TransactionBlockInput,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let TransactionBlockInput { transaction_hash, block } = transaction_block_input;

        let block_id = if let Some(block_id) = block {
            block_id.0
        } else {
            // if no block ID input, this eventually just subscribes the user to new blocks
            BlockId::Tag(BlockTag::Latest)
        };

        let (query_block_number, latest_block_number) =
            self.convert_to_block_number_range(block_id).await?;

        // perform the actual subscription
        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id).ok_or(ApiError::StarknetDevnetError(
            Error::UnexpectedInternalError { msg: format!("Unregistered socket ID: {socket_id}") },
        ))?;
        let subscription_id =
            socket_context.subscribe(rpc_request_id, Subscription::TransactionStatus).await;

        // TODO if tx present, but in a block before the one specified, no point in subscribing -
        // its status shall never change (unless considering block abortion). It would make
        // sense to just add a ReorgSubscription

        let starknet = self.api.starknet.lock().await;
        starknet.get_transaction_trace_by_hash(transaction_hash).unwrap();
        match (
            starknet.get_transaction_receipt_by_hash(&transaction_hash),
            starknet.get_transaction_execution_and_finality_status(transaction_hash),
        ) {
            (Ok(receipt), Ok(status)) => {
                let notification =
                    SubscriptionNotification::TransactionStatusNotification(NewTransactionStatus {
                        transaction_hash,
                        status,
                    });
                match receipt.get_block_number() {
                    Some(block_number)
                        if query_block_number <= block_number
                            && block_number <= latest_block_number =>
                    {
                        // if the number of the block when the tx was added is between
                        // specified/query block number and latest, notify the client
                        socket_context.notify(subscription_id, notification).await;
                    }
                    None if block_id == BlockId::Tag(BlockTag::Pending) => {
                        // if tx stored but no block number, it means it's pending, so only notify
                        // if the specified block ID is pending
                        socket_context.notify(subscription_id, notification).await;
                    }
                    _ => tracing::error!("Impossible case reached in tx status subscription"),
                }
            }
            _ => {
                tracing::debug!("Tx status subscription: tx too old or not received");
                // No error needs to be returned: too-many-blocks-back
                // is the only error that can be returned by this subscription, but
                // this was handled earlier.
            }
        };

        Ok(())
    }
}
