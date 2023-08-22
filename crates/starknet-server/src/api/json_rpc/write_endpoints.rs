use server::rpc_core::error::RpcError;
use starknet_types::felt::Felt;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
use starknet_types::rpc::transactions::broadcasted_declare_transaction_v2::BroadcastedDeclareTransactionV2;
use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
use starknet_types::rpc::transactions::broadcasted_invoke_transaction_v1::BroadcastedInvokeTransactionV1;
use starknet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedInvokeTransaction,
};

use super::error::ApiError;
use super::models::{
    DeclareTransactionOutput, DeployAccountTransactionOutput, InvokeTransactionOutput,
};
use crate::api::json_rpc::error::RpcResult;
use crate::api::json_rpc::JsonRpcHandler;

impl JsonRpcHandler {
    pub(crate) async fn add_declare_transaction(
        &self,
        request: BroadcastedDeclareTransaction,
    ) -> RpcResult<DeclareTransactionOutput> {
        let (transaction_hash, class_hash) = match request {
            BroadcastedDeclareTransaction::V1(broadcasted_declare_txn) => self
                .api
                .starknet
                .write()
                .await
                .add_declare_transaction_v1(*broadcasted_declare_txn)?,
            BroadcastedDeclareTransaction::V2(broadcasted_declare_txn) => self
                .api
                .starknet
                .write()
                .await
                .add_declare_transaction_v2(*broadcasted_declare_txn)?,
        };

        Ok(DeclareTransactionOutput { transaction_hash, class_hash })
    }

    pub(crate) async fn add_deploy_account_transaction(
        &self,
        request: BroadcastedDeployAccountTransaction,
    ) -> RpcResult<DeployAccountTransactionOutput> {
        let (transaction_hash, contract_address) = self
            .api
            .starknet
            .write()
            .await
            .add_deploy_account_transaction(request)
            .map_err(|err| match err {
                starknet_core::error::Error::StateError(
                    starknet_in_rust::core::errors::state_errors::StateError::MissingClassHash(),
                ) => ApiError::ClassHashNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(DeployAccountTransactionOutput { transaction_hash, contract_address })
    }

    pub(crate) async fn add_invoke_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
    ) -> RpcResult<InvokeTransactionOutput> {
        let hash = match request {
            BroadcastedInvokeTransaction::V0(_) => {
                Err(ApiError::UnsupportedAction { msg: "Invoke V0 is not supported".into() })
            }
            BroadcastedInvokeTransaction::V1(invoke_transaction) => {
                let res = self
                    .api
                    .starknet
                    .write()
                    .await
                    .add_invoke_transaction_v1(invoke_transaction)?;

                Ok(res)
            }
        }?;

        Ok(InvokeTransactionOutput { transaction_hash: hash })
    }
}

#[cfg(test)]
mod tests {
    use starknet_core::constants::{
        DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_HOST,
        DEVNET_DEFAULT_INITIAL_BALANCE, DEVNET_DEFAULT_PORT, DEVNET_DEFAULT_TEST_SEED,
        DEVNET_DEFAULT_TIMEOUT, DEVNET_DEFAULT_TOTAL_ACCOUNTS,
    };
    use starknet_core::starknet::{Starknet, StarknetConfig};
    use starknet_types::rpc::transactions::broadcasted_declare_transaction_v1::BroadcastedDeclareTransactionV1;
    use starknet_types::rpc::transactions::broadcasted_deploy_account_transaction::BroadcastedDeployAccountTransaction;
    use starknet_types::traits::ToHexString;

    use crate::api::json_rpc::JsonRpcHandler;
    use crate::api::Api;

    #[tokio::test]
    async fn add_declare_transaction_v1_should_be_successful() {
        let declare_txn_v1 = test_broadcasted_declare_transaction_v1();
        let json_rpc_handler = setup();
        let result = json_rpc_handler
            .add_declare_transaction(
                starknet_types::rpc::transactions::BroadcastedDeclareTransaction::V1(Box::new(
                    declare_txn_v1.clone(),
                )),
            )
            .await
            .unwrap();

        // Data taken from transaction execution to https://alpha4.starknet.io/gateway/add_transaction
        // which resulted in transaction_hash
        // 0x1d50d192f54d8d75e73c8ab8fb7159e70bfdbccc322abb43a081889a3043627 Could be checked in https://testnet.starkscan.co/tx/0x1d50d192f54d8d75e73c8ab8fb7159e70bfdbccc322abb43a081889a3043627
        assert_eq!(
            result.class_hash.to_prefixed_hex_str(),
            "0x399998c787e0a063c3ac1d2abac084dcbe09954e3b156d53a8c43a02aa27d35"
        );
    }

    #[test]
    fn check_correct_deserialization_of_deploy_account_transaction_request() {
        test_deploy_account_transaction();
    }

    fn setup() -> JsonRpcHandler {
        let config: StarknetConfig = StarknetConfig {
            seed: DEVNET_DEFAULT_TEST_SEED,
            total_accounts: DEVNET_DEFAULT_TOTAL_ACCOUNTS,
            predeployed_accounts_initial_balance: DEVNET_DEFAULT_INITIAL_BALANCE.into(),
            host: DEVNET_DEFAULT_HOST.into(),
            port: DEVNET_DEFAULT_PORT,
            timeout: DEVNET_DEFAULT_TIMEOUT,
            gas_price: DEVNET_DEFAULT_GAS_PRICE,
            chain_id: DEVNET_DEFAULT_CHAIN_ID,
        };
        let starknet = Starknet::new(&config).unwrap();
        let api = Api::new(starknet);
        JsonRpcHandler { api }
    }

    /// The example uses declare_v1.json from test_data/rpc/declare_v1.json
    /// Which declares the example from https://www.cairo-lang.org/docs/hello_starknet/intro.html#your-first-contract
    /// The example was compiled locally and send via Postman to https://alpha4.starknet.io/gateway/add_transaction
    #[test]
    fn parsed_base64_gzipped_json_contract_class_correctly() {
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();

        let _broadcasted_declare_transaction_v1: super::BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();
    }

    fn test_deploy_account_transaction() -> BroadcastedDeployAccountTransaction {
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/deploy_account.json"
        ))
        .unwrap();

        let broadcasted_deploy_account_transaction: BroadcastedDeployAccountTransaction =
            serde_json::from_str(&json_string).unwrap();

        broadcasted_deploy_account_transaction
    }

    fn test_broadcasted_declare_transaction_v1() -> BroadcastedDeclareTransactionV1 {
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();

        let broadcasted_declare_transaction_v1: super::BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        broadcasted_declare_transaction_v1
    }
}
