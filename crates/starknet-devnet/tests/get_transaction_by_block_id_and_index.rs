pub mod common;

mod get_transaction_by_block_id_and_index_integration_tests {

    use serde_json::json;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::reqwest_client::{HttpEmptyResponseBody, PostReqwestSender};

    #[tokio::test]
    async fn get_transaction_by_block_id_and_index_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let resp: serde_json::Value = devnet
            .reqwest_client()
            .post_json_async(
                "/mint",
                json!({
                    "address": "0x1",
                    "amount": 1
                }),
            )
            .await
            .unwrap();
        let tx_hash_value = resp["tx_hash"].as_str().unwrap().to_string();

        let result = devnet
            .json_rpc_client
            .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 0)
            .await
            .unwrap();

        if let starknet_rs_core::types::Transaction::Invoke(
            starknet_rs_core::types::InvokeTransaction::V1(invoke_v1),
        ) = result
        {
            assert_eq!(
                invoke_v1.transaction_hash,
                FieldElement::from_hex_be(&tx_hash_value).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {result:?}");
        }
    }

    #[tokio::test]
    async fn get_transaction_by_block_id_and_index_wrong_index() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        devnet
            .reqwest_client()
            .post_json_async(
                "/mint",
                json!({
                    "address": "0x1",
                    "amount": 1
                }),
            )
            .await
            .map(|_: HttpEmptyResponseBody| ())
            .unwrap();

        let result = devnet
            .json_rpc_client
            .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 1)
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetError::InvalidTransactionIndex) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }

    #[tokio::test]
    async fn get_transaction_by_block_id_and_index_wrong_block() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_by_block_id_and_index(BlockId::Number(1), 1)
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
