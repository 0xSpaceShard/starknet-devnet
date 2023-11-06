// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_restart {
    use hyper::StatusCode;
    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ERC20_CONTRACT_ADDRESS};
    use starknet_rs_accounts::{AccountFactory, OpenZeppelinAccountFactory};
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_core::utils::get_storage_var_address;
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::constants::CHAIN_ID;
    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::get_deployable_account_signer;

    #[tokio::test]
    async fn assert_restartable() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp = devnet.restart().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn assert_tx_not_present_after_restart() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        // generate tx
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
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        // change storage
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
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        // deploy new account
        let account_signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let salt = FieldElement::ONE;
        let deployment = account_factory.deploy(salt);
        let deployment_address = deployment.address();
        devnet.mint(deployment_address, 1e18 as u128).await;
        deployment.send().await.unwrap();

        // assert deployment address has the deployed class
        devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), deployment_address)
            .await
            .unwrap();

        devnet.restart().await.unwrap();

        // expect ContractNotFound error since account not present anymore
        match devnet
            .json_rpc_client
            .get_class_at(BlockId::Tag(BlockTag::Latest), deployment_address)
            .await
        {
            Err(ProviderError::StarknetError(StarknetErrorWithMessage { code, .. })) => {
                match code {
                    MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound) => (),
                    _ => panic!("Invalid error: {:?}", code),
                }
            }
            other => panic!("Invalid response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn assert_gas_price_unaffected_by_restart() {
        todo!();
    }

    #[tokio::test]
    async fn assert_predeployed_account_still_prefunded_and_usable() {
        todo!();
    }
}
