use std::io::Read;

use base64::Engine;
use serde_json::json;
use server::rpc_core::error::RpcError;
use starknet_core::transactions::declare_transaction::DeclareTransactionV1;
use starknet_core::TransactionError;
use starknet_types::contract_class::ContractClass;

use super::error::ApiError;
use super::RpcResult;
use super::models::DeclareTransactionOutput;
use crate::api::json_rpc::JsonRpcHandler;
use crate::api::models::FeltHex;
use crate::api::models::contract_class::DeprecatedContractClass;
use crate::api::models::transaction::{
    BroadcastedDeclareTransaction, BroadcastedDeclareTransactionV1,
};

impl JsonRpcHandler {
    pub(crate) async fn add_declare_transaction(
        &self,
        request: BroadcastedDeclareTransaction,
    ) -> RpcResult<DeclareTransactionOutput> {
        let (transaction_hash, class_hash) = match request {
            BroadcastedDeclareTransaction::V1(broadcasted_declare_txn) => {
                self.api
                    .starknet
                    .write()
                    .await
                    .add_declare_transaction_v1(broadcasted_declare_txn.try_into()?)
                    .map_err(|err| match err {
                        starknet_types::error::Error::TransactionError(
                            TransactionError::ClassAlreadyDeclared(_),
                        ) => ApiError::ClassAlreadyDeclared,
                        _ => ApiError::InvalidContractClass,
                    })?
            }
            BroadcastedDeclareTransaction::V2(_) => todo!(),
        };

        Ok(DeclareTransactionOutput{transaction_hash: FeltHex(transaction_hash), class_hash: FeltHex(class_hash)})
    }

    fn convert_base64_gziped_json_string_to_json(json_str: &str) -> RpcResult<serde_json::Value> {
        let bytes = base64::engine::general_purpose::STANDARD.decode(json_str).map_err(|_| {
            ApiError::RpcError(RpcError::invalid_params("program: Unable to decode base64 string"))
        })?;

        let mut decoder = flate2::read::GzDecoder::new(bytes.as_slice());
        let mut decoded = String::new();
        decoder.read_to_string(&mut decoded).map_err(|_| {
            ApiError::RpcError(RpcError::invalid_params("program: Unable to decode gzipped bytes"))
        })?;

        let program_json = serde_json::from_str(&decoded).map_err(|_| {
            ApiError::RpcError(RpcError::invalid_params("program: Unable to parse to JSON"))
        })?;

        Ok(program_json)
    }
}

impl TryFrom<DeprecatedContractClass> for ContractClass {
    type Error = ApiError;

    fn try_from(value: DeprecatedContractClass) -> RpcResult<Self> {
        let program_json =
            JsonRpcHandler::convert_base64_gziped_json_string_to_json(&value.program)?;
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
                "program": program_json,
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
    use base64::Engine;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use serde_json::Value;
    use starknet_core::{Starknet, StarknetConfig};
    use starknet_types::traits::ToHexString;

    use crate::api::Api;
    use crate::api::json_rpc::JsonRpcHandler;
    use crate::api::models::transaction::BroadcastedDeclareTransactionV1;

    #[tokio::test]
    async fn add_declare_transaction_v1_should_be_successful() {
        let declare_txn_v1 = test_broadcasted_declare_transaction_v1();
        let json_rpc_handler = setup();
        let result = json_rpc_handler.add_declare_transaction(
            crate::api::models::transaction::BroadcastedDeclareTransaction::V1(declare_txn_v1.clone())
        ).await
        .unwrap();

        assert!(false);
    }

    fn setup() -> JsonRpcHandler {
        let config = StarknetConfig { seed: 123, total_accounts: 1, predeployed_accounts_initial_balance: 1000.into()};
        let starknet = Starknet::new(&config).unwrap();
        let api = Api::new(starknet);
        JsonRpcHandler{ api }
    }

    #[test]
    fn parse_base64_gzipped_json_successfully() {
        let base64_gzipped_json = simple_json_program_gzipped_and_converted_to_base64();
        super::JsonRpcHandler::convert_base64_gziped_json_string_to_json(&base64_gzipped_json)
            .unwrap();
    }

    fn simple_json_program_gzipped_and_converted_to_base64() -> String {
        let json_str = r#"{
            "builtins":["pedersen","range_check","ecdsa","bitwise"]
        }"#;

        let json_obj: Value = serde_json::from_str(json_str).unwrap();

        let mut en = GzEncoder::new(Vec::new(), Compression::fast());
        serde_json::to_writer(&mut en, &json_obj).unwrap();
        let gzip_compressed = en.finish().unwrap();
        base64::engine::general_purpose::STANDARD.encode(&gzip_compressed)
    }

    fn test_broadcasted_declare_transaction_v1() -> BroadcastedDeclareTransactionV1{
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
