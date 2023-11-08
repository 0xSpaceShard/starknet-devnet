pub mod common;

mod get_transaction_by_block_id_and_index_integration_tests {
    use hyper::Body;
    use serde_json::json;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::get_json_body;

    #[tokio::test]
    async fn get_transaction_by_block_id_and_index_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": "0x1",
                "amount": 1
            })
            .to_string(),
        );
        let resp = devnet.post_json("/mint".into(), req_body).await.unwrap();
        let mut resp_body = get_json_body(resp).await;
        let tx_hash_value = resp_body["tx_hash"].take();

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
                FieldElement::from_hex_be(tx_hash_value.as_str().unwrap()).unwrap()
            );
        } else {
            panic!("Could not unpack the transaction from {result:?}");
        }
    }

    #[tokio::test]
    async fn get_transaction_by_block_id_and_index_wrong_index() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let req_body = Body::from(
            json!({
                "address": "0x1",
                "amount": 1
            })
            .to_string(),
        );
        devnet.post_json("/mint".into(), req_body).await.unwrap();

        let result = devnet
            .json_rpc_client
            .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 1)
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::InvalidTransactionIndex),
                ..
            }) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }

    #[tokio::test]
    async fn get_transaction_by_block_id_and_index_wrong_block() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_by_block_id_and_index(BlockId::Tag(BlockTag::Latest), 1)
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
