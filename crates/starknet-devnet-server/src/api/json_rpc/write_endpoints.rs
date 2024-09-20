use starknet_types::contract_address::ContractAddress;
use starknet_types::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::gas_modification::GasModificationRequest;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction,
};

use super::error::{ApiError, StrictRpcResult};
use super::models::{
    DeclareTransactionOutput, DeployAccountTransactionOutput, TransactionHashOutput,
};
use super::{DevnetResponse, StarknetResponse};
use crate::api::http::endpoints::dump_load::dump_impl;
use crate::api::http::endpoints::mint_token::mint_impl;
use crate::api::http::endpoints::postman::{
    postman_consume_message_from_l2_impl, postman_flush_impl, postman_load_impl,
    postman_send_message_to_l2_impl,
};
use crate::api::http::endpoints::time::{increase_time_impl, set_time_impl};
use crate::api::http::models::{
    AbortedBlocks, AbortingBlocks, CreatedBlock, DumpPath, FlushParameters, IncreaseTime,
    MintTokensRequest, PostmanLoadL1MessagingContract, RestartParameters, SetTime,
};
use crate::api::json_rpc::JsonRpcHandler;
use crate::dump_util::load_events;

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
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_stopImpersonateAccount
    pub async fn stop_impersonating_account(&self, address: ContractAddress) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        starknet.stop_impersonating_account(&address);
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_autoImpersonate | devnet_stopAutoImpersonate
    pub async fn set_auto_impersonate(&self, auto_impersonation: bool) -> StrictRpcResult {
        let mut starknet = self.api.starknet.lock().await;
        starknet.set_auto_impersonate_account(auto_impersonation)?;
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_dump
    pub async fn dump(&self, path: Option<DumpPath>) -> StrictRpcResult {
        let dump = dump_impl(&self.api, path).await.map_err(ApiError::from)?;
        Ok(DevnetResponse::DevnetDump(dump).into())
    }

    /// devnet_load
    pub async fn load(&self, path: String) -> StrictRpcResult {
        let events = load_events(self.starknet_config.dump_on, &path)?;
        // Necessary to restart before loading; restarting messaging to allow re-execution
        self.restart(Some(RestartParameters { restart_messaging_to_l2: true })).await?;
        self.re_execute(&events).await.map_err(ApiError::RpcError)?;

        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_postmanLoad
    pub async fn postman_load(&self, data: PostmanLoadL1MessagingContract) -> StrictRpcResult {
        postman_load_impl(&self.api, data).await
    }

    /// devnet_postmanFlush
    pub async fn postman_flush(&self, data: Option<FlushParameters>) -> StrictRpcResult {
        postman_flush_impl(&self.api, data, self).await
    }

    /// devnet_postmanSendMessageToL2
    pub async fn postman_send_message_to_l2(&self, message: MessageToL2) -> StrictRpcResult {
        postman_send_message_to_l2_impl(&self.api, message).await
    }

    /// devnet_postmanConsumeMessageFromL2
    pub async fn postman_consume_message_from_l2(&self, message: MessageToL1) -> StrictRpcResult {
        postman_consume_message_from_l2_impl(&self.api, message).await
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
        let aborted = self.api.starknet.lock().await.abort_blocks(data.starting_block_id.into())?;
        Ok(DevnetResponse::AbortedBlocks(AbortedBlocks { aborted }).into())
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
        self.api.starknet.lock().await.restart(restart_params.restart_messaging_to_l2)?;

        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_setTime
    pub async fn set_time(&self, data: SetTime) -> StrictRpcResult {
        set_time_impl(&self.api, data).await
    }

    /// devnet_increaseTime
    pub async fn increase_time(&self, data: IncreaseTime) -> StrictRpcResult {
        increase_time_impl(&self.api, data).await
    }

    /// devnet_mint
    pub async fn mint(&self, request: MintTokensRequest) -> StrictRpcResult {
        mint_impl(&self.api, request).await
    }
}

#[cfg(test)]
mod tests {
    use crate::api::json_rpc::models::{
        BroadcastedDeclareTransactionEnumWrapper, BroadcastedDeployAccountTransactionEnumWrapper,
    };
    use crate::test_utils::{declare_v1_str, deploy_account_str};

    #[test]
    fn check_correct_deserialization_of_deploy_account_transaction_request() {
        test_deploy_account_transaction();
    }

    /// The example uses declare_v1.json from test_data/rpc/declare_v1.json
    /// Which declares the example from https://www.cairo-lang.org/docs/hello_starknet/intro.html#your-first-contract
    /// The example was compiled locally and send via Postman to https://alpha4.starknet.io/gateway/add_transaction
    #[test]
    fn parsed_base64_gzipped_json_contract_class_correctly() {
        let json_string = declare_v1_str();

        let _broadcasted_declare_transaction_v1: BroadcastedDeclareTransactionEnumWrapper =
            serde_json::from_str(&json_string).unwrap();
    }

    fn test_deploy_account_transaction() -> BroadcastedDeployAccountTransactionEnumWrapper {
        let json_string = deploy_account_str();

        let broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransactionEnumWrapper =
            serde_json::from_str(&json_string).unwrap();

        broadcasted_deploy_account_transaction
    }
}
