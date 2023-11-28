pub mod common;

mod get_class_hash_at_integration_tests {
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;

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
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    /// temporarily not enabled - pending
    async fn get_class_hash_at_by_block_number() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let contract_address = FieldElement::from_hex_be(PREDEPLOYED_ACCOUNT_ADDRESS).unwrap();

        let err = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Number(0), contract_address)
            .await
            .expect_err("Should have failed");

        match err {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }

    #[tokio::test]
    /// temporarily not enabled - pending
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
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {err:?}"),
        }
    }
}
