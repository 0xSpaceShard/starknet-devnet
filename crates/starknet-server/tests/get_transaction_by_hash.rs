pub mod common;

mod get_transaction_by_hash_integration_tests {
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError, BroadcastedDeclareTransactionV1};
    use starknet_rs_providers::jsonrpc::{JsonRpcClientError, RpcError};
    use starknet_rs_providers::{Provider, ProviderError};

    // use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::util::BackgroundDevnet;

    #[tokio::test]
    async fn get_declere_v1_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        // let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();

        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let add_transaction = devnet
            .json_rpc_client
            .add_declare_transaction(
                starknet_rs_core::types::BroadcastedDeclareTransaction::V1(declare_txn_v1.clone()),
            )
            .await
            .unwrap();

        let x = add_transaction.transaction_hash;

        let get_transaction = devnet
            .json_rpc_client
            .get_transaction_by_hash(add_transaction.transaction_hash)
            .await
            .unwrap();

        assert_eq!(1, 1);
    }
}
