pub mod common;

mod get_transaction_by_hash_integration_tests {
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::jsonrpc::{JsonRpcClientError, RpcError};
    use starknet_rs_providers::{Provider, ProviderError};

    // use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::util::BackgroundDevnet;
    use crate::api::models::transaction::BroadcastedDeclareTransactionV1;

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

        let add_tx = devnet
            .json_rpc_client
            .add_declare_transaction(
                BroadcastedDeclareTransactionV1(Box::new(
                    declare_txn_v1.clone(),
                )),
            )
            .await
            .unwrap();

        assert_eq!(0, 1);
        // assert_eq!(
        //     add_tx,
        //     FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap()
        // );
    }
}
