use starknet_core::error::Error;
use starknet_rs_core::types::{BlockId, BlockTag};
use starknet_types::starknet_api::block::BlockStatus;

use super::error::ApiError;
use super::models::{BlockInput, SubscriptionIdInput};
use super::{JsonRpcHandler, JsonRpcSubscriptionRequest};
use crate::rpc_core::request::Id;
use crate::subscribe::{SocketId, SubscriptionNotification};

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
            JsonRpcSubscriptionRequest::TransactionStatus => todo!(),
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

    /// starknet_subscribeNewHeads
    /// Checks if an optional block ID is provided. Validates that the block exists and is not too
    /// many blocks in the past. If it is a valid block, the user is notified of all blocks from the
    /// old up to the latest, and subscribed to new ones. If no block ID specified, the user is just
    /// subscribed to new blocks.
    pub async fn subscribe_new_heads(
        &self,
        block_input: Option<BlockInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let latest_tag = BlockId::Tag(BlockTag::Latest);
        let block_id = if let Some(BlockInput { block }) = block_input {
            block.into()
        } else {
            // if no block ID input, this eventually just subscribes the user to new blocks
            latest_tag
        };

        let starknet = self.api.starknet.lock().await;

        // checking the block's existence; aborted blocks treated as not found
        let query_block = match starknet.get_block(&block_id) {
            Ok(block) => match block.status() {
                BlockStatus::Rejected => Err(ApiError::BlockNotFound),
                _ => Ok(block),
            },
            Err(Error::NoBlock) => Err(ApiError::BlockNotFound),
            Err(other) => Err(ApiError::StarknetDevnetError(other)),
        }?;

        let latest_block = starknet.get_block(&latest_tag)?;

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

        // perform the actual subscription
        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id).ok_or(ApiError::StarknetDevnetError(
            Error::UnexpectedInternalError { msg: format!("Unregistered socket ID: {socket_id}") },
        ))?;
        let subscription_id = socket_context.subscribe(rpc_request_id).await;

        if let BlockId::Tag(_) = block_id {
            // if the specified block ID is a tag (i.e. latest/pending), no old block handling
            return Ok(());
        }

        // Notifying of old blocks. latest_block_number inclusive?
        // Yes, only if block_id != latest/pending (handled above)
        for block_n in query_block_number..=latest_block_number {
            let old_block = starknet
                .get_block(&BlockId::Number(block_n))
                .map_err(ApiError::StarknetDevnetError)?;

            let notification = SubscriptionNotification::NewHeadsNotification(old_block.into());
            socket_context.notify(subscription_id, notification).await;
        }

        Ok(())
    }
}
