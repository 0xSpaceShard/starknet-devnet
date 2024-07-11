pub mod common;

mod get_transaction_by_hash_integration_tests {
    use std::sync::Arc;

    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ETH_ERC20_CONTRACT_ADDRESS};
    use starknet_rs_accounts::{
        Account, AccountFactory, Call, ExecutionEncoding, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, StarknetError};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::{Provider, ProviderError};

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants;
    use crate::common::utils::{
        assert_tx_successful, get_deployable_account_signer,
        get_simple_contract_in_sierra_and_compiled_class_hash, resolve_path,
    };

    #[tokio::test]
    async fn get_declare_v1_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo0"])
            .await
            .expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(resolve_path(
            "../starknet-devnet-core/test_artifacts/cairo_0_test.json",
        ))
        .unwrap();

        let legacy_contract_class: LegacyContractClass =
            serde_json::from_str(&json_string).unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::Legacy,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let declare_transaction = account
            .declare_legacy(Arc::new(legacy_contract_class))
            .nonce(Felt::ZERO)
            .send()
            .await
            .unwrap();

        assert_tx_successful(&declare_transaction.transaction_hash, &devnet.json_rpc_client).await;
    }

    #[tokio::test]
    async fn get_declare_v2_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (contract_class, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

        let (signer, address) = devnet.get_first_predeployed_account().await;
        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            address,
            constants::CHAIN_ID,
            ExecutionEncoding::Legacy,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let declare_result = account
            .declare(Arc::new(contract_class), casm_hash)
            .nonce(Felt::ZERO)
            .send()
            .await
            .unwrap();

        assert_tx_successful(&declare_result.transaction_hash, &devnet.json_rpc_client).await;
    }

    #[tokio::test]
    async fn get_deploy_account_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let signer = get_deployable_account_signer();

        let factory = OpenZeppelinAccountFactory::new(
            Felt::from_hex(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            constants::CHAIN_ID,
            signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        let salt = Felt::from_hex("0x123").unwrap();
        let deployment = factory.deploy(salt);
        let deployment_address = deployment.address();
        let fee_estimation =
            factory.deploy(salt).fee_estimate_multiplier(1.0).estimate_fee().await.unwrap();

        // fund the account before deployment
        let mint_amount = fee_estimation.overall_fee * Felt::TWO;
        devnet.mint(deployment_address, mint_amount.try_into().unwrap()).await;

        let deploy_account_result = deployment.send().await.unwrap();
        assert_tx_successful(&deploy_account_result.transaction_hash, &devnet.json_rpc_client)
            .await;
    }

    #[tokio::test]
    async fn get_invoke_v1_transaction_by_hash_happy_path() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let invoke_tx_result = account
            .execute(vec![Call {
                to: Felt::from_hex(ETH_ERC20_CONTRACT_ADDRESS).unwrap(),
                selector: get_selector_from_name("transfer").unwrap(),
                calldata: vec![
                    Felt::ONE,                                 // recipient
                    Felt::from_dec_str("1000000000").unwrap(), // low part of uint256
                    Felt::ZERO,                                // high part of uint256
                ],
            }])
            .send()
            .await
            .unwrap();

        assert_tx_successful(&invoke_tx_result.transaction_hash, &devnet.json_rpc_client).await;
    }

    #[tokio::test]
    async fn get_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_by_hash(Felt::from_hex("0x0").unwrap())
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetError::TransactionHashNotFound) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
