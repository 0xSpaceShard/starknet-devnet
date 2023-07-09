use serde_json::json;
use server::rpc_core::error::RpcError;
use starknet_core::transactions::declare_transaction::DeclareTransactionV1;
use starknet_core::TransactionError;
use starknet_types::contract_class::ContractClass;

use super::error::ApiError;
use super::models::DeclareTransactionOutput;
use super::RpcResult;
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::models::contract_class::DeprecatedContractClass;
use crate::api::models::transaction::{
    BroadcastedDeclareTransaction, BroadcastedDeclareTransactionV1,
};
use crate::api::models::FeltHex;

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
                .add_declare_transaction_v1((*broadcasted_declare_txn).try_into()?)
                .map_err(|err| match err {
                    starknet_types::error::Error::TransactionError(
                        TransactionError::ClassAlreadyDeclared(_),
                    ) => ApiError::ClassAlreadyDeclared,
                    _ => ApiError::InvalidContractClass,
                })?,
            BroadcastedDeclareTransaction::V2(_) => todo!(),
        };

        Ok(DeclareTransactionOutput {
            transaction_hash: FeltHex(transaction_hash),
            class_hash: FeltHex(class_hash),
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

impl TryFrom<BroadcastedDeclareTransactionV1>
    for starknet_core::transactions::declare_transaction::DeclareTransactionV1
{
    type Error = ApiError;
    fn try_from(value: BroadcastedDeclareTransactionV1) -> RpcResult<Self> {
        Ok(DeclareTransactionV1 {
            sender_address: value.sender_address.0,
            max_fee: value.common.max_fee.0,
            signature: value.common.signature.iter().map(|x| x.0).collect(),
            nonce: value.common.nonce.0,
            contract_class: ContractClass::try_from(value.contract_class)?,
            class_hash: None,
            transaction_hash: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use starknet_core::{Starknet, StarknetConfig};
    use starknet_types::traits::ToHexString;

    use crate::api::json_rpc::JsonRpcHandler;
    use crate::api::models::transaction::BroadcastedDeclareTransactionV1;
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

        println!("{}", result.transaction_hash.0.to_prefixed_hex_str());
    }

    fn setup() -> JsonRpcHandler {
        let config = StarknetConfig {
            seed: 123,
            total_accounts: 1,
            predeployed_accounts_initial_balance: 1000.into(),
        };
        let starknet = Starknet::new(&config).unwrap();
        let api = Api::new(starknet);
        JsonRpcHandler { api }
    }

    #[test]
    fn parsed_base64_gzipped_json_contract_class_correctly_computed_hash() {
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();

        let _broadcasted_declare_transaction_v1: super::BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();
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
