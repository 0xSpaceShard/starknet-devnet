use serde_json::json;
use server::rpc_core::error::RpcError;
use starknet_core::transactions::declare_transaction::DeclareTransactionV1;
use starknet_core::transactions::declare_transaction_v2::DeclareTransactionV2;
use starknet_core::transactions::deploy_account_transaction::DeployAccountTransaction;
use starknet_types::contract_class::ContractClass;
use starknet_types::felt::Felt;

use super::error::ApiError;
use super::models::{DeclareTransactionOutput, DeployAccountTransactionOutput};
use super::RpcResult;
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::models::contract_class::DeprecatedContractClass;
use crate::api::models::transaction::{
    BroadcastedDeclareTransaction, BroadcastedDeclareTransactionV1,
    BroadcastedDeclareTransactionV2, BroadcastedDeployAccountTransaction,
};
use crate::api::models::{ContractAddressHex, FeltHex};

impl JsonRpcHandler {
    pub(crate) async fn add_declare_transaction(
        &self,
        request: BroadcastedDeclareTransaction,
    ) -> RpcResult<DeclareTransactionOutput> {
        let chain_id = self.api.starknet.read().await.config.chain_id.to_felt();
        let (transaction_hash, class_hash) = match request {
            BroadcastedDeclareTransaction::V1(broadcasted_declare_txn) => {
                self.api.starknet.write().await.add_declare_transaction_v1(
                    (convert_to_declare_transaction_v1(*broadcasted_declare_txn, chain_id.into()))?,
                )?
            }
            BroadcastedDeclareTransaction::V2(broadcasted_declare_txn) => {
                self.api.starknet.write().await.add_declare_transaction_v2(
                    convert_to_declare_transaction_v2(*broadcasted_declare_txn, chain_id.into()),
                )?
            }
        };

        Ok(DeclareTransactionOutput {
            transaction_hash: FeltHex(transaction_hash),
            class_hash: FeltHex(class_hash),
        })
    }

    pub(crate) async fn add_deploy_account_transaction(
        &self,
        request: BroadcastedDeployAccountTransaction,
    ) -> RpcResult<DeployAccountTransactionOutput> {
        let chain_id = self.api.starknet.read().await.config.chain_id.to_felt();
        let (transaction_hash, contract_address) = self
            .api
            .starknet
            .write()
            .await
            .add_deploy_account_transaction(convert_to_deploy_account_transaction(
                request,
                chain_id.into(),
            )?)
            .map_err(|err| match err {
                starknet_core::error::Error::StateError(
                    starknet_in_rust::core::errors::state_errors::StateError::MissingClassHash(),
                ) => ApiError::ClassHashNotFound,
                unknown_error => ApiError::StarknetDevnetError(unknown_error),
            })?;

        Ok(DeployAccountTransactionOutput {
            transaction_hash: FeltHex(transaction_hash),
            contract_address: ContractAddressHex(contract_address),
        })
    }
}

impl TryFrom<DeprecatedContractClass> for ContractClass {
    type Error = ApiError;

    fn try_from(value: DeprecatedContractClass) -> RpcResult<Self> {
        let abi_json = serde_json::to_value(value.abi).map_err(|_| {
            ApiError::RpcError(RpcError::invalid_params("abi: Unable to parse to JSON"))
        })?;
        let entry_points_json = serde_json::to_value(value.entry_points_by_type).map_err(|_| {
            ApiError::RpcError(RpcError::invalid_params(
                "entry_points_by_type: Unable to parse to JSON",
            ))
        })?;

        Ok(ContractClass::Cairo0(starknet_types::contract_class::Cairo0ContractClass::Json(
            json!({
                "program": value.program,
                "abi": abi_json,
                "entry_points_by_type": entry_points_json,
            }),
        )))
    }
}

fn convert_to_declare_transaction_v1(
    value: BroadcastedDeclareTransactionV1,
    chain_id: Felt,
) -> RpcResult<DeclareTransactionV1> {
    Ok(DeclareTransactionV1::new(
        value.sender_address.0,
        value.common.max_fee.0,
        value.common.signature.iter().map(|x| x.0).collect(),
        value.common.nonce.0,
        ContractClass::try_from(value.contract_class)?,
        chain_id,
    ))
}

fn convert_to_deploy_account_transaction(
    broadcasted_txn: BroadcastedDeployAccountTransaction,
    chain_id: Felt,
) -> RpcResult<DeployAccountTransaction> {
    DeployAccountTransaction::new(
        broadcasted_txn.constructor_calldata.iter().map(|felt_hex| felt_hex.0).collect(),
        broadcasted_txn.common.max_fee.0,
        broadcasted_txn.common.signature.iter().map(|felt_hex| felt_hex.0).collect(),
        broadcasted_txn.common.nonce.0,
        broadcasted_txn.class_hash.0,
        broadcasted_txn.contract_address_salt.0,
        chain_id,
        broadcasted_txn.common.version.0,
    )
    .map_err(|err| {
        ApiError::RpcError(RpcError::invalid_params(format!(
            "Unable to create DeployAccountTransaction: {}",
            err
        )))
    })
}

fn convert_to_declare_transaction_v2(
    value: BroadcastedDeclareTransactionV2,
    chain_id: Felt,
) -> DeclareTransactionV2 {
    DeclareTransactionV2::new(
        ContractClass::from(value.contract_class),
        value.compiled_class_hash.0,
        value.sender_address.0,
        value.common.max_fee.0,
        value.common.signature.iter().map(|x| x.0).collect(),
        value.common.nonce.0,
        chain_id,
    )
}
#[cfg(test)]
mod tests {
    use starknet_core::constants::{
        DEVNET_DEFAULT_CHAIN_ID, DEVNET_DEFAULT_GAS_PRICE, DEVNET_DEFAULT_HOST,
        DEVNET_DEFAULT_INITIAL_BALANCE, DEVNET_DEFAULT_PORT, DEVNET_DEFAULT_TEST_SEED,
        DEVNET_DEFAULT_TIMEOUT, DEVNET_DEFAULT_TOTAL_ACCOUNTS,
    };
    use starknet_core::starknet::{Starknet, StarknetConfig};
    use starknet_types::traits::ToHexString;

    use crate::api::json_rpc::JsonRpcHandler;
    use crate::api::models::transaction::{
        BroadcastedDeclareTransactionV1, BroadcastedDeployAccountTransaction,
    };
    use crate::api::Api;

    #[tokio::test]
    async fn add_declare_transaction_v1_should_be_successful() {
        let declare_txn_v1 = test_broadcasted_declare_transaction_v1();
        let json_rpc_handler = setup();
        let result = json_rpc_handler
            .add_declare_transaction(
                crate::api::models::transaction::BroadcastedDeclareTransaction::V1(Box::new(
                    declare_txn_v1.clone(),
                )),
            )
            .await
            .unwrap();

        // Data taken from transaction execution to https://alpha4.starknet.io/gateway/add_transaction
        // which resulted in transaction_hash
        // 0x1d50d192f54d8d75e73c8ab8fb7159e70bfdbccc322abb43a081889a3043627 Could be checked in https://testnet.starkscan.co/tx/0x1d50d192f54d8d75e73c8ab8fb7159e70bfdbccc322abb43a081889a3043627
        assert_eq!(
            result.class_hash.0.to_prefixed_hex_str(),
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
