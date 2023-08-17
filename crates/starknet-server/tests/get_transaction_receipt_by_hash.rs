pub mod common;

mod get_transaction_receipt_by_hash_integration_tests {

    use starknet_rs_core::types::{
        BroadcastedDeclareTransactionV1, FieldElement, StarknetError, TransactionStatus,
    };
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::util::BackgroundDevnet;

    #[tokio::test]
    async fn get_declare_v1_transaction_receipt_by_hash_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let declare_transaction = devnet
            .json_rpc_client
            .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                declare_txn_v1.clone(),
            ))
            .await
            .unwrap();

        let result: starknet_rs_core::types::MaybePendingTransactionReceipt = devnet
            .json_rpc_client
            .get_transaction_receipt(declare_transaction.transaction_hash)
            .await
            .unwrap();

        match result {
            starknet_rs_core::types::MaybePendingTransactionReceipt::Receipt(receipt) => {
                match receipt {
                    starknet_rs_core::types::TransactionReceipt::Declare(declare) => {
                        assert_eq!(declare.status, TransactionStatus::Rejected);
                    }
                    _ => panic!("Invalid error: {receipt:?}"),
                }
            }
            _ => panic!("Invalid error: {result:?}"),
        }
    }

    #[tokio::test]
    async fn get_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_receipt(FieldElement::from_hex_be("0x0").unwrap())
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
