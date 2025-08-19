use starknet_core::error::Error;
use starknet_types::emitted_event::SubscriptionEmittedEvent;
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::block::{
    Block, BlockId, BlockResult, BlockStatus, BlockTag, PreConfirmedBlock,
};
use starknet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_types::rpc::transactions::{
    TransactionFinalityStatus, TransactionWithHash, Transactions,
};

use super::error::ApiError;
use super::models::{
    EventsSubscriptionInput, SubscriptionBlockIdInput, SubscriptionIdInput, TransactionHashInput,
    TransactionSubscriptionInput,
};
use super::{JsonRpcHandler, JsonRpcSubscriptionRequest};
use crate::rpc_core::request::Id;
use crate::subscribe::{
    AddressFilter, NewTransactionNotification, NewTransactionStatus, NotificationData, SocketId,
    StatusFilter, Subscription, TransactionFinalityStatusWithoutL1,
};

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
            JsonRpcSubscriptionRequest::TransactionStatus(TransactionHashInput {
                transaction_hash,
            }) => self.subscribe_tx_status(transaction_hash, rpc_request_id, socket_id).await,
            JsonRpcSubscriptionRequest::NewTransactions(data) => {
                self.subscribe_new_txs(data, rpc_request_id, socket_id).await
            }
            JsonRpcSubscriptionRequest::NewTransactionReceipts(data) => {
                self.subscribe_new_tx_receipts(data, rpc_request_id, socket_id).await
            }
            JsonRpcSubscriptionRequest::Events(data) => {
                self.subscribe_events(data, rpc_request_id, socket_id).await
            }
            JsonRpcSubscriptionRequest::Unsubscribe(SubscriptionIdInput { subscription_id }) => {
                let mut sockets = self.api.sockets.lock().await;
                let socket_context = sockets.get_mut(&socket_id)?;
                socket_context.unsubscribe(rpc_request_id, subscription_id).await
            }
        }
    }

    /// Returns (starting block number, latest block number). Returns an error in case the starting
    /// block does not exist or there are too many blocks.
    async fn get_validated_block_number_range(
        &self,
        mut starting_block_id: BlockId,
    ) -> Result<(u64, u64), ApiError> {
        let starknet = self.api.starknet.lock().await;

        // Convert pre-confirmed to latest to prevent getting block_number = 0
        starting_block_id = match starting_block_id {
            BlockId::Tag(BlockTag::PreConfirmed) => BlockId::Tag(BlockTag::Latest),
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

        // safe to subtract, ensured by previous checks
        if latest_block_number - query_block_number > 1024 {
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
        block_input: Option<SubscriptionBlockIdInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let block_id = if let Some(SubscriptionBlockIdInput { block_id }) = block_input {
            block_id.into()
        } else {
            // if no block ID input, this eventually just subscribes the user to new blocks
            BlockId::Tag(BlockTag::Latest)
        };

        let (query_block_number, latest_block_number) =
            self.get_validated_block_number_range(block_id).await?;

        // perform the actual subscription
        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;
        let subscription_id =
            socket_context.subscribe(rpc_request_id, Subscription::NewHeads).await;

        if let BlockId::Tag(_) = block_id {
            // if the specified block ID is a tag (i.e. latest/pre-confirmed), no old block handling
            return Ok(());
        }

        // Notifying of old blocks. latest_block_number inclusive?
        // Yes, only if block_id != latest/pre-confirmed (handled above)
        let starknet = self.api.starknet.lock().await;
        for block_n in query_block_number..=latest_block_number {
            let old_block = starknet
                .get_block(&BlockId::Number(block_n))
                .map_err(ApiError::StarknetDevnetError)?;

            let old_header = old_block.into();
            let notification = NotificationData::NewHeads(old_header);
            socket_context.notify(subscription_id, notification).await;
        }

        Ok(())
    }

    async fn get_tx_receipts(
        &self,
        block_id: &BlockId,
    ) -> Result<Vec<TransactionReceipt>, ApiError> {
        let starknet = self.api.starknet.lock().await;
        let block = starknet.get_block_with_receipts(block_id)?;

        let txs = match block {
            BlockResult::Block(block) => block.transactions,
            BlockResult::PreConfirmedBlock(pre_confirmed_block) => pre_confirmed_block.transactions,
        };

        let txs_with_receipt = match txs {
            Transactions::Hashes(_) | Transactions::Full(_) => {
                return Err(ApiError::StarknetDevnetError(Error::UnexpectedInternalError {
                    msg: format!("Invalid txs: {txs:?}"),
                }));
            }
            Transactions::FullWithReceipts(txs_with_receipt) => txs_with_receipt,
        };

        Ok(txs_with_receipt.into_iter().map(|tx_with_receipt| tx_with_receipt.receipt).collect())
    }

    async fn get_pre_confirmed_txs(&self) -> Result<Vec<TransactionWithHash>, ApiError> {
        let starknet = self.api.starknet.lock().await;
        let block = starknet.get_block_with_transactions(&BlockId::Tag(BlockTag::PreConfirmed))?;
        match block {
            BlockResult::PreConfirmedBlock(PreConfirmedBlock {
                transactions: Transactions::Full(txs),
                ..
            }) => Ok(txs),
            _ => {
                // Never reached if get_block_with_transactions properly implemented.
                Err(ApiError::StarknetDevnetError(Error::UnexpectedInternalError {
                    msg: "Invalid block".into(),
                }))
            }
        }
    }

    async fn get_latest_txs(&self) -> Result<Vec<TransactionWithHash>, ApiError> {
        let starknet = self.api.starknet.lock().await;
        let block = starknet.get_block_with_transactions(&BlockId::Tag(BlockTag::Latest))?;
        match block {
            BlockResult::Block(Block { transactions: Transactions::Full(txs), .. }) => Ok(txs),
            _ => {
                // Never reached if get_block_with_transactions properly implemented.
                Err(ApiError::StarknetDevnetError(Error::UnexpectedInternalError {
                    msg: "Invalid block".into(),
                }))
            }
        }
    }

    /// Does not return TOO_MANY_ADDRESSES_IN_FILTER
    pub async fn subscribe_new_txs(
        &self,
        maybe_subscription_input: Option<TransactionSubscriptionInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let status_filter = StatusFilter::new(
            maybe_subscription_input
                .as_ref()
                .and_then(|subscription_input| subscription_input.finality_status.clone())
                .unwrap_or_else(|| vec![TransactionFinalityStatusWithoutL1::AcceptedOnL2]),
        );

        let address_filter = AddressFilter::new(
            maybe_subscription_input
                .and_then(|subscription_input| subscription_input.sender_address)
                .unwrap_or_default(),
        );

        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;

        let subscription =
            Subscription::NewTransactions { address_filter, status_filter: status_filter.clone() };
        let subscription_id = socket_context.subscribe(rpc_request_id, subscription).await;

        if status_filter.passes(&TransactionFinalityStatusWithoutL1::PreConfirmed) {
            let pre_confirmed_txs = self.get_pre_confirmed_txs().await?;
            for tx in pre_confirmed_txs {
                let notification = NotificationData::NewTransaction(NewTransactionNotification {
                    tx,
                    finality_status: TransactionFinalityStatusWithoutL1::PreConfirmed,
                });
                socket_context.notify(subscription_id, notification).await;
            }
        }

        if status_filter.passes(&TransactionFinalityStatusWithoutL1::AcceptedOnL2) {
            let latest_txs = self.get_latest_txs().await?;
            for tx in latest_txs {
                let notification = NotificationData::NewTransaction(NewTransactionNotification {
                    tx,
                    finality_status: TransactionFinalityStatusWithoutL1::AcceptedOnL2,
                });
                socket_context.notify(subscription_id, notification).await;
            }
        }

        Ok(())
    }

    /// Does not return TOO_MANY_ADDRESSES_IN_FILTER
    /// TODO: refactor
    pub async fn subscribe_new_tx_receipts(
        &self,
        maybe_subscription_input: Option<TransactionSubscriptionInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let status_filter = StatusFilter::new(
            maybe_subscription_input
                .as_ref()
                .and_then(|subscription_input| subscription_input.finality_status.clone())
                .unwrap_or_else(|| vec![TransactionFinalityStatusWithoutL1::AcceptedOnL2]),
        );

        let address_filter = AddressFilter::new(
            maybe_subscription_input
                .and_then(|subscription_input| subscription_input.sender_address)
                .unwrap_or_default(),
        );

        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;

        let subscription = Subscription::NewTransactionReceipts {
            address_filter,
            status_filter: status_filter.clone(),
        };
        let subscription_id = socket_context.subscribe(rpc_request_id, subscription).await;

        for (finality_status, tag) in [
            (TransactionFinalityStatusWithoutL1::PreConfirmed, BlockTag::PreConfirmed),
            (TransactionFinalityStatusWithoutL1::AcceptedOnL2, BlockTag::Latest),
        ] {
            if status_filter.passes(&finality_status) {
                let receipts = self.get_tx_receipts(&BlockId::Tag(tag)).await?;
                for receipt in receipts {
                    let notification = NotificationData::NewTransactionReceipt(receipt);
                    socket_context.notify(subscription_id, notification).await;
                }
            }
        }

        Ok(())
    }

    async fn subscribe_tx_status(
        &self,
        transaction_hash: TransactionHash,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        // perform the actual subscription
        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;

        let subscription = Subscription::TransactionStatus { transaction_hash };
        let subscription_id = socket_context.subscribe(rpc_request_id, subscription).await;

        let starknet = self.api.starknet.lock().await;

        if let Some(tx) = starknet.transactions.get(&transaction_hash) {
            let notification = NotificationData::TransactionStatus(NewTransactionStatus {
                transaction_hash,
                status: tx.get_status(),
            });
            socket_context.notify(subscription_id, notification).await;
        } else {
            tracing::debug!("Tx status subscription: tx not yet received")
        }

        Ok(())
    }

    async fn subscribe_events(
        &self,
        maybe_subscription_input: Option<EventsSubscriptionInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let address = maybe_subscription_input
            .as_ref()
            .and_then(|subscription_input| subscription_input.from_address);

        let starting_block_id = maybe_subscription_input
            .as_ref()
            .and_then(|subscription_input| subscription_input.block_id.as_ref())
            .map(|b| b.into())
            .unwrap_or(BlockId::Tag(BlockTag::Latest));

        self.get_validated_block_number_range(starting_block_id).await?;

        let keys_filter = maybe_subscription_input
            .as_ref()
            .and_then(|subscription_input| subscription_input.keys.clone());

        let finality_status_filter = maybe_subscription_input
            .and_then(|subscription_input| subscription_input.finality_status)
            .unwrap_or(TransactionFinalityStatus::AcceptedOnL2);

        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;
        let subscription = Subscription::Events {
            address,
            keys_filter: keys_filter.clone(),
            finality_status_filter,
        };
        let subscription_id = socket_context.subscribe(rpc_request_id, subscription).await;

        let starknet = self.api.starknet.lock().await;
        let events = starknet.get_unlimited_events(
            Some(starting_block_id),
            Some(BlockId::Tag(BlockTag::PreConfirmed)),
            address,
            keys_filter,
            Some(finality_status_filter),
        )?;

        for event in events {
            let notification_data = NotificationData::Event(SubscriptionEmittedEvent {
                emitted_event: event,
                finality_status: finality_status_filter,
            });
            socket_context.notify(subscription_id, notification_data).await;
        }

        Ok(())
    }
}
