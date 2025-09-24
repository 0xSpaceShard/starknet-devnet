use starknet_rs_core::types::TransactionExecutionStatus;
use starknet_types::contract_address::ContractAddress;
use starknet_types::felt::{TransactionHash, felt_from_prefixed_hex};
use starknet_types::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::block::{BlockId, BlockTag};
use starknet_types::rpc::gas_modification::GasModificationRequest;
use starknet_types::rpc::transaction_receipt::FeeUnit;
use starknet_types::rpc::transactions::l1_handler_transaction::L1HandlerTransaction;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction,
};

use super::error::{ApiError, StrictRpcResult};
use super::models::{
    DeclareTransactionOutput, DeployAccountTransactionOutput, DevnetResponse, JsonRpcResponse,
    StarknetResponse, TransactionHashOutput,
};
use crate::api::JsonRpcHandler;
use crate::api::account_helpers::{get_balance, get_erc20_fee_unit_address};
use crate::api::models::{
    AbortedBlocks, AbortingBlocks, AcceptOnL1Request, AcceptedOnL1Blocks, CreatedBlock, DumpPath,
    FlushParameters, FlushedMessages, IncreaseTime, IncreaseTimeResponse, MessageHash,
    MessagingLoadAddress, MintTokensRequest, MintTokensResponse, PostmanLoadL1MessagingContract,
    RestartParameters, SetTime, SetTimeResponse,
};
use crate::dump_util::{dump_events, load_events};
use crate::rpc_core::error::RpcError;
use crate::rpc_core::request::RpcMethodCall;
use crate::rpc_core::response::ResponseResult;
use crate::rpc_handler::RpcHandler;

impl JsonRpcHandler {
    pub async fn add_declare_transaction(
        &self,
        request: BroadcastedDeclareTransaction,
    ) -> StrictRpcResult {
        let (transaction_hash, class_hash) =
            self.api.starknet.lock().await.add_declare_transaction(request).map_err(
                |err| match err {
                    starknet_core::error::Error::CompiledClassHashMismatch => {
                        ApiError::CompiledClassHashMismatch
                    }
                    starknet_core::error::Error::ClassAlreadyDeclared { .. } => {
                        ApiError::ClassAlreadyDeclared
                    }
                    starknet_core::error::Error::ContractClassSizeIsTooLarge => {
                        ApiError::ContractClassSizeIsTooLarge
                    }
                    unknown_error => ApiError::StarknetDevnetError(unknown_error),
                },
            )?;

        Ok(StarknetResponse::AddDeclareTransaction(DeclareTransactionOutput {
            transaction_hash,
            class_hash,
        })
        .into())
    }

    pub async fn add_deploy_account_transaction(
        &self,
        request: BroadcastedDeployAccountTransaction,
    ) -> StrictRpcResult {
        let (transaction_hash, contract_address) =
            self.api.starknet.lock().await.add_deploy_account_transaction(request).map_err(
                |err| match err {
                    starknet_core::error::Error::StateError(
                        starknet_core::error::StateError::NoneClassHash(_),
                    ) => ApiError::ClassHashNotFound,
                    unknown_error => ApiError::StarknetDevnetError(unknown_error),
                },
            )?;

        Ok(StarknetResponse::AddDeployAccountTransaction(DeployAccountTransactionOutput {
            transaction_hash,
            contract_address,
        })
        .into())
    }

    pub async fn add_invoke_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
    ) -> StrictRpcResult {
        let transaction_hash = self.api.starknet.lock().await.add_invoke_transaction(request)?;

        Ok(StarknetResponse::TransactionHash(TransactionHashOutput { transaction_hash }).into())
    }

    /// devnet_impersonateAccount
    pub async fn impersonate_account(&self, address: ContractAddress) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        starknet.impersonate_account(address)?;
        Ok(JsonRpcResponse::Empty)
    }

    /// devnet_stopImpersonateAccount
    pub async fn stop_impersonating_account(&self, address: ContractAddress) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        starknet.stop_impersonating_account(&address);
        Ok(JsonRpcResponse::Empty)
    }

    /// devnet_autoImpersonate | devnet_stopAutoImpersonate
    pub async fn set_auto_impersonate(&self, auto_impersonation: bool) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        starknet.set_auto_impersonate_account(auto_impersonation)?;
        Ok(JsonRpcResponse::Empty)
    }

    /// devnet_dump
    pub async fn dump(&self, path: Option<DumpPath>) -> StrictRpcResult {
        let starknet = self.api.starknet.lock().await;
        if starknet.config.dump_on.is_none() {
            return Err(ApiError::DumpError {
                msg: "Please provide --dump-on mode on startup.".to_string(),
            });
        }

        let path = path
            .as_ref()
            .map(|DumpPath { path }| path.clone())
            .or_else(|| starknet.config.dump_path.clone())
            .unwrap_or_default();

        drop(starknet);
        let dumpable_events = self.api.dumpable_events.lock().await;

        if !path.is_empty() {
            dump_events(&dumpable_events, &path)
                .map_err(|err| ApiError::DumpError { msg: err.to_string() })?;
            return Ok(DevnetResponse::DevnetDump(None).into());
        }

        Ok(DevnetResponse::DevnetDump(Some(dumpable_events.clone())).into())
    }

    /// devnet_load
    pub async fn load(&self, path: String) -> StrictRpcResult {
        let events = load_events(self.starknet_config.dump_on, &path)?;
        // Necessary to restart before loading; restarting messaging to allow re-execution
        self.restart(Some(RestartParameters { restart_l1_to_l2_messaging: true })).await?;
        self.re_execute(&events).await.map_err(ApiError::RpcError)?;

        Ok(JsonRpcResponse::Empty)
    }

    /// devnet_postmanLoad
    pub async fn postman_load(&self, data: PostmanLoadL1MessagingContract) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        let messaging_contract_address = starknet
            .configure_messaging(
                &data.network_url,
                data.messaging_contract_address.as_deref(),
                data.deployer_account_private_key.as_deref(),
            )
            .await?;

        Ok(DevnetResponse::MessagingContractAddress(MessagingLoadAddress {
            messaging_contract_address,
        })
        .into())
    }

    /// devnet_postmanFlush
    pub async fn postman_flush(&self, data: Option<FlushParameters>) -> StrictRpcResult {
        let is_dry_run = if let Some(params) = data { params.dry_run } else { false };

        // Need to handle L1 to L2 first in case those messages create L2 to L1 messages.
        let mut messages_to_l2 = vec![];
        let mut generated_l2_transactions = vec![];
        if !is_dry_run {
            // Fetch and execute messages to L2.
            // It is important that self.api.starknet is dropped immediately to allow rpc execution
            messages_to_l2 =
                self.api.starknet.lock().await.fetch_messages_to_l2().await.map_err(|e| {
                    ApiError::RpcError(RpcError::internal_error_with(format!(
                        "Error in fetching messages to L2: {e}"
                    )))
                })?;

            for message in &messages_to_l2 {
                let rpc_call = message.try_into().map_err(|e| {
                    ApiError::RpcError(RpcError::internal_error_with(format!(
                        "Error in converting message to L2 RPC call: {e}"
                    )))
                })?;
                let tx_hash = execute_rpc_tx(self, rpc_call).await.map_err(ApiError::RpcError)?;
                generated_l2_transactions.push(tx_hash);
            }
        };

        // Collect and send messages to L1.
        let mut starknet = self.api.starknet.lock().await;
        let messages_to_l1 = starknet.collect_messages_to_l1().await.map_err(|e| {
            ApiError::RpcError(RpcError::internal_error_with(format!(
                "Error in collecting messages to L1: {e}"
            )))
        })?;

        let l1_provider = if is_dry_run {
            "dry run".to_string()
        } else {
            starknet.send_messages_to_l1().await.map_err(|e| {
                ApiError::RpcError(RpcError::internal_error_with(format!(
                    "Error in sending messages to L1: {e}"
                )))
            })?;
            starknet.get_ethereum_url().unwrap_or("Not set".to_string())
        };

        let flushed_messages = FlushedMessages {
            messages_to_l1,
            messages_to_l2,
            generated_l2_transactions,
            l1_provider,
        };

        Ok(DevnetResponse::FlushedMessages(flushed_messages).into())
    }

    /// devnet_postmanSendMessageToL2
    pub async fn postman_send_message_to_l2(&self, message: MessageToL2) -> StrictRpcResult {
        let transaction = L1HandlerTransaction::try_from_message_to_l2(message)?;
        let transaction_hash =
            self.api.starknet.lock().await.add_l1_handler_transaction(transaction)?;
        Ok(DevnetResponse::TransactionHash(TransactionHashOutput { transaction_hash }).into())
    }

    /// devnet_postmanConsumeMessageFromL2
    pub async fn postman_consume_message_from_l2(&self, message: MessageToL1) -> StrictRpcResult {
        let message_hash =
            self.api.starknet.lock().await.consume_l2_to_l1_message(&message).await?;
        Ok(DevnetResponse::MessageHash(MessageHash { message_hash }).into())
    }

    /// devnet_createBlock
    pub async fn create_block(&self) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;

        starknet.create_block()?;
        let block = starknet.get_latest_block()?;

        Ok(DevnetResponse::CreatedBlock(CreatedBlock { block_hash: block.block_hash() }).into())
    }

    /// devnet_abortBlocks
    pub async fn abort_blocks(&self, data: AbortingBlocks) -> StrictRpcResult {
        let aborted = self.api.starknet.lock().await.abort_blocks(data.starting_block_id)?;
        Ok(DevnetResponse::AbortedBlocks(AbortedBlocks { aborted }).into())
    }

    /// devnet_acceptOnL1
    pub async fn accept_on_l1(&self, data: AcceptOnL1Request) -> StrictRpcResult {
        let accepted = self.api.starknet.lock().await.accept_on_l1(data.starting_block_id)?;
        Ok(DevnetResponse::AcceptedOnL1Blocks(AcceptedOnL1Blocks { accepted }).into())
    }

    /// devnet_setGasPrice
    pub async fn set_gas_price(&self, data: GasModificationRequest) -> StrictRpcResult {
        let modified_gas =
            self.api.starknet.lock().await.set_next_block_gas(data).map_err(ApiError::from)?;

        Ok(DevnetResponse::GasModification(modified_gas).into())
    }

    /// devnet_restart
    pub async fn restart(&self, data: Option<RestartParameters>) -> StrictRpcResult {
        self.api.dumpable_events.lock().await.clear();

        let restart_params = data.unwrap_or_default();
        self.api.starknet.lock().await.restart(restart_params.restart_l1_to_l2_messaging)?;

        self.api.sockets.lock().await.clear();

        Ok(JsonRpcResponse::Empty)
    }

    /// devnet_setTime
    pub async fn set_time(&self, data: SetTime) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        let generate_block = data.generate_block.unwrap_or(true);
        starknet.set_time(data.time, generate_block)?;
        let block_hash = if generate_block {
            let last_block = starknet.get_latest_block()?;
            Some(last_block.block_hash())
        } else {
            None
        };
        Ok(DevnetResponse::SetTime(SetTimeResponse { block_timestamp: data.time, block_hash })
            .into())
    }

    /// devnet_increaseTime
    pub async fn increase_time(&self, data: IncreaseTime) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        starknet.increase_time(data.time)?;

        let last_block = starknet.get_latest_block()?;

        Ok(DevnetResponse::IncreaseTime(IncreaseTimeResponse {
            timestamp_increased_by: data.time,
            block_hash: last_block.block_hash(),
        })
        .into())
    }

    /// devnet_mint
    pub async fn mint(&self, request: MintTokensRequest) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        let unit = request.unit.unwrap_or(FeeUnit::FRI);
        let erc20_address = get_erc20_fee_unit_address(unit);

        // increase balance
        let tx_hash = starknet.mint(request.address, request.amount, erc20_address).await?;

        let tx = starknet.get_transaction_execution_and_finality_status(tx_hash)?;
        match tx.execution_status {
            TransactionExecutionStatus::Succeeded => {
                let new_balance = get_balance(
                    &mut starknet,
                    request.address,
                    erc20_address,
                    BlockId::Tag(BlockTag::PreConfirmed),
                )?;
                let new_balance = new_balance.to_str_radix(10);

                Ok(DevnetResponse::MintTokens(MintTokensResponse { new_balance, unit, tx_hash })
                    .into())
            }
            TransactionExecutionStatus::Reverted => Err(ApiError::MintingReverted {
                tx_hash,
                revert_reason: tx.failure_reason.map(|reason| {
                    if reason.contains("u256_add Overflow") {
                        "The requested minting amount overflows the token contract's total_supply."
                            .into()
                    } else {
                        reason
                    }
                }),
            }),
        }
    }
}

async fn execute_rpc_tx(
    rpc_handler: &JsonRpcHandler,
    rpc_call: RpcMethodCall,
) -> Result<TransactionHash, RpcError> {
    match rpc_handler.on_call(rpc_call).await.result {
        ResponseResult::Success(result) => {
            let tx_hash_hex = result
                .get("transaction_hash")
                .ok_or(RpcError::internal_error_with(format!(
                    "Message execution did not yield a transaction hash: {result:?}"
                )))?
                .as_str()
                .ok_or(RpcError::internal_error_with(format!(
                    "Message execution result contains invalid transaction hash: {result:?}"
                )))?;
            let tx_hash = felt_from_prefixed_hex(tx_hash_hex).map_err(|e| {
                RpcError::internal_error_with(format!(
                    "Message execution resulted in an invalid tx hash: {tx_hash_hex}: {e}"
                ))
            })?;
            Ok(tx_hash)
        }
        ResponseResult::Error(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use crate::api::models::BroadcastedDeployAccountTransactionEnumWrapper;

    #[test]
    fn check_correct_deserialization_of_deploy_account_transaction_request() {
        let _: BroadcastedDeployAccountTransactionEnumWrapper = serde_json::from_str(
            r#"{
                "type":"DEPLOY_ACCOUNT",
                "resource_bounds": {
                    "l1_gas": {
                        "max_amount": "0x1",
                        "max_price_per_unit": "0x2"
                    },
                    "l1_data_gas": {
                        "max_amount": "0x1",
                        "max_price_per_unit": "0x2"
                    },
                    "l2_gas": {
                        "max_amount": "0x1",
                        "max_price_per_unit": "0x2"
                    }
                },
                "tip": "0xabc",
                "paymaster_data": [],
                "version": "0x3",
                "signature": ["0xFF", "0xAA"],
                "nonce": "0x0",
                "contract_address_salt": "0x01",
                "class_hash": "0x01",
                "constructor_calldata": ["0x01"],
                "nonce_data_availability_mode": "L1",
                "fee_data_availability_mode": "L1"
            }"#,
        )
        .unwrap();
    }
}
