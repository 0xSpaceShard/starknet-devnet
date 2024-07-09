use starknet_types::contract_address::ContractAddress;
use starknet_types::messaging::{MessageToL1, MessageToL2};
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction,
};

use super::error::{ApiError, StrictRpcResult};
use super::models::{
    DeclareTransactionOutput, DeployAccountTransactionOutput, TransactionHashOutput,
};
use super::{DevnetResponse, StarknetResponse};
use crate::api::http::endpoints::blocks::{abort_blocks_impl, create_block_impl};
use crate::api::http::endpoints::dump_load::{dump_impl, load_impl};
use crate::api::http::endpoints::mint_token::mint_impl;
use crate::api::http::endpoints::postman::{
    postman_consume_message_from_l2_impl, postman_flush_impl, postman_load_impl,
    postman_send_message_to_l2_impl,
};
use crate::api::http::endpoints::restart_impl;
use crate::api::http::endpoints::time::{increase_time_impl, set_time_impl};
use crate::api::http::models::{
    AbortingBlocks, DumpPath, FlushParameters, IncreaseTime, LoadPath, MintTokensRequest,
    PostmanLoadL1MessagingContract, SetTime,
};
use crate::api::json_rpc::JsonRpcHandler;

impl JsonRpcHandler {
    pub async fn add_declare_transaction(
        &self,
        request: BroadcastedDeclareTransaction,
    ) -> StrictRpcResult {
        let (transaction_hash, class_hash) =
            self.api.starknet.write().await.add_declare_transaction(request)?;

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
            self.api.starknet.write().await.add_deploy_account_transaction(request).map_err(
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
        let transaction_hash = self.api.starknet.write().await.add_invoke_transaction(request)?;

        Ok(StarknetResponse::TransactionHash(TransactionHashOutput { transaction_hash }).into())
    }

    /// devnet_impersonateAccount
    pub async fn impersonate_account(&self, address: ContractAddress) -> StrictRpcResult {
        let mut starknet = self.api.starknet.write().await;
        starknet.impersonate_account(address)?;
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_stopImpersonateAccount
    pub async fn stop_impersonating_account(&self, address: ContractAddress) -> StrictRpcResult {
        let mut starknet = self.api.starknet.write().await;
        starknet.stop_impersonating_account(&address);
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_autoImpersonate | devnet_stopAutoImpersonate
    pub async fn set_auto_impersonate(&self, auto_impersonation: bool) -> StrictRpcResult {
        let mut starknet = self.api.starknet.write().await;
        starknet.set_auto_impersonate_account(auto_impersonation)?;
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_dump
    pub async fn dump(&self, path: DumpPath) -> StrictRpcResult {
        let dump = dump_impl(&self.api, path).await.map_err(ApiError::from)?;
        Ok(DevnetResponse::DevnetDump(dump).into())
    }

    /// devnet_load
    pub async fn load(&self, path: LoadPath) -> StrictRpcResult {
        load_impl(&self.api, path).await.map_err(ApiError::from)?;
        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_postmanLoad
    pub async fn postman_load(&self, data: PostmanLoadL1MessagingContract) -> StrictRpcResult {
        Ok(DevnetResponse::MessagingContractAddress(
            postman_load_impl(&self.api, data).await.map_err(ApiError::from)?,
        )
        .into())
    }

    /// devnet_postmanFlush
    pub async fn postman_flush(&self, data: FlushParameters) -> StrictRpcResult {
        Ok(DevnetResponse::FlushedMessages(
            postman_flush_impl(&self.api, data).await.map_err(ApiError::from)?,
        )
        .into())
    }

    /// devnet_postmanSendMessageToL2
    pub async fn postman_send_message_to_l2(&self, message: MessageToL2) -> StrictRpcResult {
        let transaction_hash =
            postman_send_message_to_l2_impl(&self.api, message).await.map_err(ApiError::from)?;

        Ok(DevnetResponse::TransactionHash(TransactionHashOutput {
            transaction_hash: transaction_hash.transaction_hash,
        })
        .into())
    }

    /// devnet_postmanConsumeMessageFromL2
    pub async fn postman_consume_message_from_l2(&self, message: MessageToL1) -> StrictRpcResult {
        let message_hash = postman_consume_message_from_l2_impl(&self.api, message)
            .await
            .map_err(ApiError::from)?;

        Ok(DevnetResponse::MessageHash(message_hash).into())
    }

    /// devnet_createBlock
    pub async fn create_block(&self) -> StrictRpcResult {
        let created_block = create_block_impl(&self.api).await.map_err(ApiError::from)?;
        Ok(DevnetResponse::CreatedBlock(created_block).into())
    }

    /// devnet_abortBlocks
    pub async fn abort_blocks(&self, data: AbortingBlocks) -> StrictRpcResult {
        let aborted_blocks = abort_blocks_impl(&self.api, data).await.map_err(ApiError::from)?;

        Ok(DevnetResponse::AbortedBlocks(aborted_blocks).into())
    }

    /// devnet_restart
    pub async fn restart(&self) -> StrictRpcResult {
        restart_impl(&self.api).await.map_err(ApiError::from)?;

        Ok(super::JsonRpcResponse::Empty)
    }

    /// devnet_setTime
    pub async fn set_time(&self, data: SetTime) -> StrictRpcResult {
        let set_time_response = set_time_impl(&self.api, data).await.map_err(ApiError::from)?;
        Ok(DevnetResponse::SetTime(set_time_response).into())
    }

    /// devnet_increaseTime
    pub async fn increase_time(&self, data: IncreaseTime) -> StrictRpcResult {
        let increase_time_response =
            increase_time_impl(&self.api, data).await.map_err(ApiError::from)?;

        Ok(DevnetResponse::IncreaseTime(increase_time_response).into())
    }

    /// devnet_mint
    pub async fn mint(&self, request: MintTokensRequest) -> StrictRpcResult {
        let mint_tokens_response = mint_impl(&self.api, request).await.map_err(ApiError::from)?;

        Ok(DevnetResponse::MintTokens(mint_tokens_response).into())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::json_rpc::models::{
        BroadcastedDeclareTransactionEnumWrapper, BroadcastedDeployAccountTransactionEnumWrapper,
    };
    use crate::test_utils::exported_test_utils::{declare_v1_str, deploy_account_str};

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
