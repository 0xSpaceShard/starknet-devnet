pub mod common;

mod get_class_hash_at_integration_tests {
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::jsonrpc::{JsonRpcClientError, RpcError};
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::util::BackgroundDevnet;

    #[tokio::test]
    async fn get_class_hash_at_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let retrieved_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();

        assert_eq!(
            retrieved_hash,
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap()
        );
    }

    #[tokio::test]
    async fn get_class_hash_at_for_undeployed_address() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let undeployed_address = "0x1234";
        let contract_address = FieldElement::from_hex_be(undeployed_address).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn get_class_hash_at_by_block_number() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Number(0), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::Other(JsonRpcClientError::RpcError(RpcError::Unknown(
                json_rpc_error,
            ))) => {
                // cannot apply matching for the exact values because `message` must be String
                assert_eq!(json_rpc_error.code, -1);
                assert_eq!(
                    json_rpc_error.message,
                    "Specifying block by number is currently not enabled"
                );
            }
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn get_class_hash_at_by_block_hash() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(
                BlockId::Hash(FieldElement::from_hex_be("0x1").unwrap()),
                contract_address,
            )
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::Other(JsonRpcClientError::RpcError(RpcError::Unknown(
                json_rpc_error,
            ))) => {
                // cannot apply matching for the exact values because `message` must be String
                assert_eq!(json_rpc_error.code, -1);
                assert_eq!(
                    json_rpc_error.message,
                    "Specifying block by hash is currently not enabled"
                );
            }
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}
