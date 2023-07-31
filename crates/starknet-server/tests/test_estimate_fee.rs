pub mod common;

mod estimate_fee_tests {
    use std::sync::Arc;

    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_accounts::{
        Account, AccountFactory, AccountFactoryError, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{FeeEstimate, FieldElement, StarknetError};
    use starknet_rs_core::utils::get_contract_address;
    use starknet_rs_providers::ProviderError;

    use crate::common::constants::{CAIRO_0_CONTRACT_PATH, CHAIN_ID};
    use crate::common::util::{
        get_deployable_account_signer, get_predeployed_account_props, load_json,
        resolve_crates_path, BackgroundDevnet,
    };

    fn assert_fee_estimation(fee_estimation: &FeeEstimate) {
        assert_eq!(
            fee_estimation.gas_price * fee_estimation.gas_consumed,
            fee_estimation.overall_fee
        );
        assert!(fee_estimation.overall_fee > 0u64, "Checking fee_estimation: {fee_estimation:?}");
    }

    #[tokio::test]
    async fn estimate_fee_of_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // define the key of the new account - dummy value
        let new_account_signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            new_account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = FieldElement::ZERO;

        // fund address
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let deployment = account_factory.deploy(salt);
        let deployment_address = deployment.address();
        let fee_estimation = account_factory
            .deploy(salt)
            .fee_estimate_multiplier(1.0)
            .nonce(new_account_nonce)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);

        // fund the account before deployment
        let mint_amount = fee_estimation.overall_fee as u128 * 2;
        devnet.mint(deployment_address, mint_amount).await;

        // TODO uncomment the following section once starknet_in_rust fixes max_fee checking
        // try sending with insufficient max fee
        // let insufficient_max_fee = fee_estimation.overall_fee * 9 / 10; // 90% of estimate - not
        // enough let unsuccessful_deployment_tx = account_factory
        // .deploy(salt)
        // .max_fee(FieldElement::from(insufficient_max_fee))
        // .nonce(new_account_nonce)
        // .send()
        // .await
        // .unwrap();
        // todo!("Assert the tx is not accepted");

        // try sending with sufficient max fee
        let sufficient_max_fee = fee_estimation.overall_fee * 11 / 10;
        let _result = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from(sufficient_max_fee))
            .nonce(new_account_nonce)
            .send()
            .await
            .expect("Should deploy with sufficient fee");
        // TODO assert tx is accepted
    }

    #[tokio::test]
    async fn estimate_fee_of_invalid_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let new_account_signer = get_deployable_account_signer();
        let dummy_invalid_class_hash = FieldElement::from_hex_be("0x123").unwrap();
        let account_factory = OpenZeppelinAccountFactory::new(
            dummy_invalid_class_hash,
            CHAIN_ID,
            new_account_signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = FieldElement::ZERO;

        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let err = account_factory
            .deploy(salt)
            .nonce(new_account_nonce)
            .estimate_fee()
            .await
            .expect_err("Should have failed");
        match err {
            AccountFactoryError::Provider(ProviderError::StarknetError(
                StarknetError::ContractError,
            )) => (),
            other => panic!("Got wrong error: {other}"),
        }
    }

    #[tokio::test]
    async fn estimate_fee_of_declare_v1() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();

        // get class
        let contract_artifact_path = resolve_crates_path(CAIRO_0_CONTRACT_PATH);
        let contract_artifact: LegacyContractClass = load_json(&contract_artifact_path);

        // declare class
        let account =
            SingleOwnerAccount::new(devnet.clone_provider(), signer, account_address, CHAIN_ID);

        let fee_estimation = account
            .declare_legacy(Arc::new(contract_artifact))
            .nonce(FieldElement::ZERO)
            .fee_estimate_multiplier(1.0)
            .estimate_fee()
            .await
            .unwrap();
        assert_fee_estimation(&fee_estimation);
    }

    #[tokio::test]
    async fn estimate_fee_of_declare_v2() {
        todo!();
    }

    #[tokio::test]
    async fn estimate_fee_of_invoke() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();
        let account =
            SingleOwnerAccount::new(devnet.clone_provider(), signer, account_address, CHAIN_ID);

        // get class
        let contract_artifact_path = resolve_crates_path(CAIRO_0_CONTRACT_PATH);
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(load_json(&contract_artifact_path));
        let class_hash = contract_artifact.class_hash().unwrap();

        // declare class
        let declaration_result = account
            .declare_legacy(contract_artifact.clone())
            .nonce(FieldElement::ZERO)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();
        assert_eq!(declaration_result.class_hash, class_hash);

        // deploy instance of class
        let contract_factory = ContractFactory::new(class_hash, account);
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let constructor_calldata = vec![];
        let _contract_address =
            get_contract_address(salt, class_hash, &constructor_calldata, account_address);
        contract_factory
            .deploy(constructor_calldata, salt, true)
            .nonce(FieldElement::ONE)
            // max fee implicitly estimated
            .send()
            .await
            .expect("Cannot deploy");

        todo!("Add invoke estimation here");
    }

    #[tokio::test]
    async fn estimate_fee_of_multiple_txs() {
        todo!();
    }
}
