use starknet_core::error::Error;
use starknet_rs_core::types::{
    BlockId as ImportedBlockId, Felt, L1DataAvailabilityMode as ImportedL1DataAvailabilityMode,
    MaybePreConfirmedBlockWithTxHashes,
};
use starknet_rs_providers::{Provider, ProviderError};
use starknet_types::contract_address::ContractAddress;
use starknet_types::emitted_event::{EmittedEvent, SubscriptionEmittedEvent};
use starknet_types::felt::TransactionHash;
use starknet_types::rpc::block::{BlockHeader, BlockId, BlockStatus, BlockTag};
use starknet_types::rpc::transactions::TransactionFinalityStatus;
use starknet_types::starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_types::starknet_api::data_availability::L1DataAvailabilityMode;

use super::JsonRpcHandler;
use super::error::ApiError;
use super::models::{
    EventsSubscriptionInput, SubscriptionBlockIdInput, SubscriptionIdInput, TransactionHashInput,
    TransactionReceiptSubscriptionInput, TransactionSubscriptionInput,
};
use crate::api::models::JsonRpcSubscriptionRequest;
use crate::rpc_core::request::Id;
use crate::subscribe::{
    AddressFilter, NewTransactionStatus, NotificationData, SocketId, StatusFilter, Subscription,
};

/// The definitions of JSON-RPC read endpoints defined in starknet_ws_api.json
impl JsonRpcHandler {
    pub async fn execute_ws_subscription(
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
    ) -> Result<(u64, u64, Option<(u64, u64)>), ApiError> {
        let starknet = self.api.starknet.lock().await;

        // Convert pre-confirmed to latest to prevent getting block_number = 0
        starting_block_id = match starting_block_id {
            BlockId::Tag(BlockTag::PreConfirmed) => BlockId::Tag(BlockTag::Latest),
            other => other,
        };

        let query_block_number = match starting_block_id {
            BlockId::Number(n) => n,
            block_id => {
                let block = match starknet.get_block(&block_id) {
                    Ok(block) => match block.status() {
                        BlockStatus::Rejected => return Err(ApiError::BlockNotFound),
                        _ => Ok::<_, ApiError>(block),
                    },
                    Err(Error::NoBlock) => return Err(ApiError::BlockNotFound),
                    Err(other) => return Err(ApiError::StarknetDevnetError(other)),
                }?;
                block.block_number().0
            }
        };

        let latest_block_number =
            starknet.get_block(&BlockId::Tag(BlockTag::Latest))?.block_number().0;

        if query_block_number > latest_block_number {
            return Err(ApiError::BlockNotFound);
        }
        if latest_block_number - query_block_number > 1024 {
            return Err(ApiError::TooManyBlocksBack);
        }

        // Check if forking is configured and return the block range from the forking origin
        let origin_block_range = match (
            starknet.config.fork_config.url.as_ref(),
            starknet.config.fork_config.block_number,
        ) {
            (Some(_url), Some(fork_block_number)) => {
                // If the query block number is less than or equal to the fork block number,
                // we need to fetch blocks from the origin
                if query_block_number <= fork_block_number {
                    Some((query_block_number, fork_block_number))
                } else {
                    None
                }
            }
            _ => None, // No fork configuration or block number
        };

        let validated_start_block_number =
            if let Some(origin) = origin_block_range { origin.1 + 1 } else { query_block_number };

        Ok((validated_start_block_number, latest_block_number, origin_block_range))
    }

    async fn fetch_origin_heads(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<Vec<BlockHeader>, ApiError> {
        let origin_caller = self.origin_caller.as_ref().ok_or_else(|| {
            ApiError::StarknetDevnetError(Error::UnexpectedInternalError {
                msg: "No origin caller available".into(),
            })
        })?;
        let mut headers = Vec::new();
        for block_n in start_block..=end_block {
            let block_id = ImportedBlockId::Number(block_n);
            match origin_caller.starknet_client.get_block_with_tx_hashes(block_id).await {
                Ok(MaybePreConfirmedBlockWithTxHashes::Block(origin_block)) => {
                    let origin_header = BlockHeader {
                        block_hash: origin_block.block_hash,
                        parent_hash: origin_block.parent_hash,
                        block_number: BlockNumber(origin_block.block_number),
                        l1_gas_price: origin_block.l1_gas_price.into(),
                        l2_gas_price: origin_block.l2_gas_price.into(),
                        new_root: origin_block.new_root,
                        sequencer_address: ContractAddress::new_unchecked(
                            origin_block.sequencer_address,
                        ),
                        timestamp: BlockTimestamp(origin_block.timestamp),
                        starknet_version: origin_block.starknet_version,
                        l1_data_gas_price: origin_block.l1_data_gas_price.into(),
                        l1_da_mode: match origin_block.l1_da_mode {
                            ImportedL1DataAvailabilityMode::Calldata => {
                                L1DataAvailabilityMode::Calldata
                            }
                            ImportedL1DataAvailabilityMode::Blob => L1DataAvailabilityMode::Blob,
                        },
                    };
                    headers.push(origin_header);
                }
                Err(ProviderError::StarknetError(
                    starknet_rs_core::types::StarknetError::BlockNotFound,
                )) => {
                    return Err(ApiError::BlockNotFound);
                }
                other => {
                    return Err(ApiError::StarknetDevnetError(
                        starknet_core::error::Error::UnexpectedInternalError {
                            msg: format!(
                                "Failed retrieval of block from forking origin. Got: {other:?}"
                            ),
                        },
                    ));
                }
            }
        }
        Ok(headers)
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

        let (query_block_number, latest_block_number, origin_range) =
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

        if let Some((origin_start, origin_end)) = origin_range {
            let origin_headers = self.fetch_origin_heads(origin_start, origin_end).await?;
            for header in origin_headers {
                let notification = NotificationData::NewHeads(header);
                socket_context.notify(subscription_id, notification).await;
            }
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
                .and_then(|input| input.finality_status.as_ref())
                .map_or_else(
                    || vec![TransactionFinalityStatus::AcceptedOnL2],
                    |statuses| {
                        statuses.iter().cloned().map(TransactionFinalityStatus::from).collect()
                    },
                ),
        );

        let address_filter = AddressFilter::new(
            maybe_subscription_input
                .and_then(|subscription_input| subscription_input.sender_address)
                .unwrap_or_default(),
        );

        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;

        let subscription = Subscription::NewTransactions { address_filter, status_filter };
        socket_context.subscribe(rpc_request_id, subscription).await;

        Ok(())
    }

    /// Does not return TOO_MANY_ADDRESSES_IN_FILTER
    pub async fn subscribe_new_tx_receipts(
        &self,
        maybe_subscription_input: Option<TransactionReceiptSubscriptionInput>,
        rpc_request_id: Id,
        socket_id: SocketId,
    ) -> Result<(), ApiError> {
        let status_filter = StatusFilter::new(
            maybe_subscription_input
                .as_ref()
                .and_then(|input| input.finality_status.as_ref())
                .map_or_else(
                    || vec![TransactionFinalityStatus::AcceptedOnL2],
                    |statuses| {
                        statuses.iter().cloned().map(TransactionFinalityStatus::from).collect()
                    },
                ),
        );

        let address_filter = AddressFilter::new(
            maybe_subscription_input
                .and_then(|subscription_input| subscription_input.sender_address)
                .unwrap_or_default(),
        );

        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;

        let subscription = Subscription::NewTransactionReceipts { address_filter, status_filter };
        socket_context.subscribe(rpc_request_id, subscription).await;

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

    async fn fetch_origin_events(
        &self,
        from_block: u64,
        to_block: u64,
        address: Option<ContractAddress>,
        keys_filter: Option<Vec<Vec<Felt>>>,
    ) -> Result<Vec<EmittedEvent>, ApiError> {
        const DEFAULT_CHUNK_SIZE: u64 = 1000;
        let mut continuation_token: Option<String> = None;
        let mut all_events = Vec::new();

        // Fetch all events with pagination
        loop {
            let events_chunk = self
                .get_origin_events(
                    from_block,
                    to_block,
                    continuation_token,
                    address,
                    keys_filter.clone(),
                    DEFAULT_CHUNK_SIZE,
                )
                .await?;

            // Extend our collection with events from this chunk
            all_events.extend(events_chunk.events);

            // Update continuation token or break if done
            match events_chunk.continuation_token {
                Some(token) => continuation_token = Some(token),
                None => break,
            }
        }

        Ok(all_events)
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
            .and_then(|subscription_input| subscription_input.block_id.as_ref().map(BlockId::from))
            .unwrap_or(BlockId::Tag(BlockTag::Latest));

        let (_, _, origin_range) = self.get_validated_block_number_range(starting_block_id).await?;

        let keys_filter = maybe_subscription_input
            .as_ref()
            .and_then(|subscription_input| subscription_input.keys.clone());

        let finality_status = maybe_subscription_input
            .and_then(|subscription_input| subscription_input.finality_status)
            .unwrap_or(TransactionFinalityStatus::AcceptedOnL2);

        let mut sockets = self.api.sockets.lock().await;
        let socket_context = sockets.get_mut(&socket_id)?;
        let subscription = Subscription::Events {
            address,
            keys_filter: keys_filter.clone(),
            status_filter: StatusFilter::new(vec![finality_status]),
        };
        let subscription_id = socket_context.subscribe(rpc_request_id, subscription).await;

        // If we're in a fork and need events from the origin
        if let Some((origin_start, origin_end)) = origin_range {
            let origin_events = self
                .fetch_origin_events(origin_start, origin_end, address, keys_filter.clone())
                .await?;

            for event in origin_events {
                let notification_data = NotificationData::Event(SubscriptionEmittedEvent {
                    emitted_event: event,
                    finality_status,
                });
                socket_context.notify(subscription_id, notification_data).await;
            }
        }

        // Get events from local chain
        let events = self.api.starknet.lock().await.get_unlimited_events(
            Some(starting_block_id),
            Some(BlockId::Tag(BlockTag::PreConfirmed)), // Last block; filtering by status
            address,
            keys_filter,
            Some(finality_status),
        )?;

        for event in events {
            let notification_data = NotificationData::Event(SubscriptionEmittedEvent {
                emitted_event: event,
                finality_status,
            });
            socket_context.notify(subscription_id, notification_data).await;
        }

        Ok(())
    }
}
