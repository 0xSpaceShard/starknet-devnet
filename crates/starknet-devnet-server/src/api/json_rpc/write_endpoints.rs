use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction,
};

use super::error::{ApiError, StrictRpcResult};
use super::models::{
    DeclareTransactionOutput, DeployAccountTransactionOutput, InvokeTransactionOutput,
};
use super::StarknetResponse;
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
        }))
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
        }))
    }

    pub async fn add_invoke_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
    ) -> StrictRpcResult {
        let transaction_hash = self.api.starknet.write().await.add_invoke_transaction(request)?;

        Ok(StarknetResponse::AddInvokeTransaction(InvokeTransactionOutput { transaction_hash }))
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
