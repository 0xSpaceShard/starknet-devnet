pub mod common;

mod get_class_tests {
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::util::BackgroundDevnet;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV1, FieldElement,
    };
    use starknet_rs_providers::Provider;

    #[tokio::test]
    async fn get_class_at() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let res = devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await;

        let res = res.unwrap();
    }

    #[tokio::test]
    async fn get_class() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let retrieved_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();

        let res =
            devnet.json_rpc_client.get_class(BlockId::Tag(BlockTag::Latest), retrieved_hash).await;

        let res = res.unwrap();
    }

    // #[tokio::test]
    // async fn get_class() {
    //     let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
    //     let json_string = std::fs::read_to_string(concat!(
    //         env!("CARGO_MANIFEST_DIR"),
    //         "/test_data/rpc/declare_v1.json"
    //     ))
    //     .unwrap();
    //     let declare_txn_v1: BroadcastedDeclareTransactionV1 =
    //         serde_json::from_str(&json_string).unwrap();
    //
    //     let declare_transaction = devnet
    //         .json_rpc_client
    //         .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
    //             declare_txn_v1.clone(),
    //         ))
    //         .await
    //         .unwrap();
    // }
}
