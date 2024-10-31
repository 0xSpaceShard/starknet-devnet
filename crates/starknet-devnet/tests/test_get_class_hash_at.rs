#![cfg(test)]
pub mod common;

mod get_class_hash_at_integration_tests {
    use starknet_devnet_core::constants::CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH;
    use starknet_core::types::{BlockId, BlockTag, Felt, StarknetError};
    use starknet_providers::{Provider, ProviderError};
    use starknet_devnet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;

    #[tokio::test]
    async fn get_class_hash_at_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let retrieved_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), contract_address)
            .await
            .unwrap();

        assert_eq!(
            retrieved_hash,
            felt_from_prefixed_hex(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap()
        );
    }

    #[tokio::test]
    async fn get_class_hash_at_for_undeployed_address() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let undeployed_address = Felt::from_hex_unchecked("0x1234");

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), undeployed_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    async fn get_class_hash_at_by_block_number() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let result =
            devnet.json_rpc_client.get_class_hash_at(BlockId::Number(0), contract_address).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_class_hash_at_by_block_hash() {
        let devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--state-archive-capacity", "full"])
                .await
                .expect("Could not start Devnet");
        let contract_address = felt_from_prefixed_hex(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Hash(Felt::ONE), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetError::BlockNotFound) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}
