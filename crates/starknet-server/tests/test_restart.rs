// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_restart {
    use std::path::Path;
    use std::sync::Arc;

    use hyper::StatusCode;
    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ERC20_CONTRACT_ADDRESS};
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{
        Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_core::utils::get_storage_var_address;
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::CHAIN_ID;
    use crate::common::utils::{get_deployable_account_signer, remove_file, send_ctrl_c_signal};

    #[tokio::test]
    async fn assert_restartable() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let resp = devnet.restart().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn assert_tx_and_block_not_present_after_restart() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        // generate dummy tx
        let mint_hash = devnet.mint(FieldElement::ONE, 100).await;
        assert!(devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await.is_ok());

        let restart_resp = devnet.restart().await.unwrap();
        assert_eq!(restart_resp.status(), StatusCode::OK);

        match devnet.json_rpc_client.get_transaction_by_hash(mint_hash).await {
            Err(ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            })) => (),
            other => panic!("Unexpected result: {other:?}"),
        }

        match devnet.json_rpc_client.get_block_with_txs(BlockId::Tag(BlockTag::Latest)).await {
            Err(ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                ..
            })) => (),
            other => panic!("Unexpected result: {other:?}"),
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

        // assert there is a class associated with the deployment address
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
        let expected_gas_price = 1_000_000_u64;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--gas-price",
            &expected_gas_price.to_string(),
        ])
        .await
        .unwrap();

        // get a predeployed account
        let (signer, address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        ));

        // prepare class for estimation of declaration
        let contract_json = dummy_cairo_0_contract_class();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());

        // check gas price via fee estimation
        let estimate_before = predeployed_account
            .declare_legacy(contract_artifact.clone())
            .estimate_fee()
            .await
            .unwrap();
        assert_eq!(estimate_before.gas_price, expected_gas_price);

        devnet.restart().await.unwrap();

        let estimate_after =
            predeployed_account.declare_legacy(contract_artifact).estimate_fee().await.unwrap();

        // assert gas_price and fee are equal to the values before restart
        assert_eq!(estimate_before.gas_price, estimate_after.gas_price);
        assert_eq!(estimate_before.overall_fee, estimate_after.overall_fee);
    }

    #[tokio::test]
    async fn assert_predeployed_account_is_prefunded_after_restart() {
        let initial_balance = 1_000_000_u32;
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--initial-balance",
            &initial_balance.to_string(),
        ])
        .await
        .unwrap();

        let predeployed_account_addresss = devnet.get_first_predeployed_account().await.1;

        let balance_before = devnet.get_balance(&predeployed_account_addresss).await.unwrap();
        assert_eq!(balance_before, FieldElement::from(initial_balance));

        devnet.restart().await.unwrap();

        let balance_after = devnet.get_balance(&predeployed_account_addresss).await.unwrap();
        assert_eq!(balance_before, balance_after);
    }

    #[tokio::test]
    async fn assert_dumping_not_affected_by_restart() {
        let dump_file_name = "dump_after_restart";
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            dump_file_name,
            "--dump-on",
            "exit",
        ])
        .await
        .unwrap();

        devnet.restart().await.unwrap();

        // send a dummy tx; otherwise there's no dump
        devnet.mint(FieldElement::ONE, 1).await;

        // assert dump file not already here
        assert!(!Path::new(dump_file_name).exists());

        // assert killing the process can still dump devnet
        send_ctrl_c_signal(&devnet.process).await;
        std::thread::sleep(std::time::Duration::from_secs(1)); // sleep to allow dump
        assert!(Path::new(dump_file_name).exists());

        remove_file(dump_file_name);
    }

    #[tokio::test]
    async fn assert_load_not_affecting_restart() {
        let dump_file_name = "dump_before_restart";
        let devnet = BackgroundDevnet::spawn_with_additional_args(&[
            "--dump-path",
            dump_file_name,
            "--dump-on",
            "exit",
        ])
        .await
        .unwrap();

        // send a dummy tx; otherwise there's no dump
        let tx_hash = devnet.mint(FieldElement::ONE, 1).await;

        send_ctrl_c_signal(&devnet.process).await;
        std::thread::sleep(std::time::Duration::from_secs(1)); // sleep to allow dump
        assert!(Path::new(dump_file_name).exists());

        let loaded_devnet =
            BackgroundDevnet::spawn_with_additional_args(&["--dump-path", dump_file_name])
                .await
                .unwrap();

        loaded_devnet.restart().await.unwrap();

        // asserting that restarting really clears the state, without re-executing txs from dump
        match loaded_devnet.json_rpc_client.get_transaction_by_hash(tx_hash).await {
            Err(ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            })) => (),
            other => panic!("Unexpected result: {other:?}"),
        }

        remove_file(dump_file_name);
    }
}
