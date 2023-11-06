// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_restart {
    use hyper::StatusCode;
    use starknet_core::constants::ERC20_CONTRACT_ADDRESS;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_core::utils::get_storage_var_address;
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::devnet::BackgroundDevnet;

    #[tokio::test]
    async fn assert_restartable() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp = devnet.restart().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn assert_tx_not_present_after_restart() {
        // generate tx
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let dummy_address = FieldElement::from_hex_be("0x1").unwrap();
        let mint_hash = devnet.mint(dummy_address, 100).await;
        assert!(devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await.is_ok());

        let restart_resp = devnet.restart().await.unwrap();
        assert_eq!(restart_resp.status(), StatusCode::OK);

        match devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await.unwrap_err() {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            }) => (),
            other => panic!("Invalid error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn assert_storage_restarted() {
        // change storage
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let dummy_address = FieldElement::from_hex_be("0x1").unwrap();
        let mint_amount = 100;
        devnet.mint(dummy_address, mint_amount).await;

        // define storage retriever
        let storage_key = get_storage_var_address("ERC20_balances", &[dummy_address]).unwrap();
        let get_storage = || {
            devnet.json_rpc_client.get_storage_at(
                FieldElement::from_hex_be(ERC20_CONTRACT_ADDRESS).unwrap(),
                storage_key,
                BlockId::Tag(BlockTag::Latest),
            )
        };

        let storage_value_before = get_storage().await.unwrap();
        assert_eq!(storage_value_before, FieldElement::from(mint_amount));

        devnet.restart().await.unwrap();

        let storage_value_after = get_storage().await.unwrap();
        assert_eq!(storage_value_after, FieldElement::ZERO);
    }

    #[tokio::test]
    async fn assert_account_deployment_reverted() {
        todo!();
    }

    #[tokio::test]
    async fn assert_gas_price_unaffected_by_restart() {
        todo!();
    }

    #[tokio::test]
    async fn assert_predeployed_account_still_prefunded() {
        todo!();
    }
}
