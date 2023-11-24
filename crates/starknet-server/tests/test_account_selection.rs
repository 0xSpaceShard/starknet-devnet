// must use `pub`: https://github.com/rust-lang/rust/issues/46379#issuecomment-548787629
pub mod common;

mod test_account_selection {
    use std::sync::Arc;

    use starknet_core::constants::{
        CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
    };
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{
        Account, AccountFactory, Call, ExecutionEncoding, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, FieldElement, FunctionCall, MaybePendingTransactionReceipt,
        TransactionReceipt,
    };
    use starknet_rs_core::utils::{
        get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::Provider;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::CHAIN_ID;
    use crate::common::utils::get_deployable_account_signer;

    #[tokio::test]
    async fn spawnable_with_cairo0() {
        BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo0"]).await.unwrap();
    }

    #[tokio::test]
    async fn spawnable_with_cairo1() {
        BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"]).await.unwrap();
    }

    #[tokio::test]
    async fn spawnable_with_custom_account() {
        BackgroundDevnet::spawn_with_additional_args(&[
            "--account-class-custom",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ])
        .await
        .unwrap();
    }

    /// Common body for tests defined below
    async fn correct_artifact_test_body(devnet_args: &[&str], expected_hash: &str) {
        let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await.unwrap();

        let (_, account_address) = devnet.get_first_predeployed_account().await;
        let retrieved_class_hash = devnet
            .json_rpc_client
            .get_class_hash_at(BlockId::Tag(BlockTag::Latest), account_address)
            .await
            .unwrap();
        let expected_class_hash = FieldElement::from_hex_be(expected_hash).unwrap();
        assert_eq!(retrieved_class_hash, expected_class_hash);
    }

    #[tokio::test]
    async fn correct_cairo1_artifact() {
        correct_artifact_test_body(
            &["--account-class", "cairo1"],
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
        )
        .await;
    }

    #[tokio::test]
    async fn correct_custom_artifact() {
        correct_artifact_test_body(
            &["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH],
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
        )
        .await;
    }

    /// Common body for tests defined below
    async fn can_deploy_new_account_test_body(devnet_args: &[&str]) {
        let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await.unwrap();

        let signer = get_deployable_account_signer();

        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap(),
            CHAIN_ID,
            signer,
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        let new_account_nonce = FieldElement::ZERO;
        let salt = FieldElement::THREE;
        let deployment = account_factory.deploy(salt).nonce(new_account_nonce);
        let new_account_address = deployment.address();
        devnet.mint(new_account_address, 1e18 as u128).await;

        let deploy_account_result = deployment.send().await.unwrap();

        let deploy_account_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deploy_account_result.transaction_hash)
            .await
            .unwrap();

        match deploy_account_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::DeployAccount(receipt)) => {
                assert_eq!(receipt.contract_address, new_account_address);
            }
            _ => {
                panic!("Invalid receipt {:?}", deploy_account_receipt);
            }
        }
    }

    #[tokio::test]
    async fn can_deploy_new_cairo1_account() {
        can_deploy_new_account_test_body(&["--account-class", "cairo1"]).await;
    }

    #[tokio::test]
    async fn can_deploy_new_custom_account() {
        can_deploy_new_account_test_body(&[
            "--account-class-custom",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ])
        .await;
    }

    /// Common body for tests defined below
    async fn can_declare_deploy_invoke_using_predeployed_test_body(devnet_args: &[&str]) {
        let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await.unwrap();

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        ));

        // get class
        let contract_json = dummy_cairo_0_contract_class();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());
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
        let contract_factory = ContractFactory::new(class_hash, account.clone());
        let salt = FieldElement::from_hex_be("0x123").unwrap();
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            class_hash,
            &UdcUniqueness::NotUnique,
            &constructor_calldata,
        );
        contract_factory
            .deploy(constructor_calldata, salt, false)
            .nonce(FieldElement::ONE)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .expect("Cannot deploy");

        // invoke
        let increase_amount = FieldElement::from(100u128);
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increase_amount],
        }];
        account.execute(invoke_calls).send().await.unwrap();

        // prepare the call used in checking the balance
        let call = FunctionCall {
            contract_address,
            entry_point_selector: get_selector_from_name("get_balance").unwrap(),
            calldata: vec![],
        };
        let balance_after_sufficient =
            devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
        assert_eq!(balance_after_sufficient, vec![increase_amount]);
    }

    #[tokio::test]
    async fn can_declare_deploy_invoke_using_predeployed_cairo1() {
        can_declare_deploy_invoke_using_predeployed_test_body(&["--account-class", "cairo1"]).await;
    }

    #[tokio::test]
    async fn can_declare_deploy_invoke_using_predeployed_custom() {
        can_declare_deploy_invoke_using_predeployed_test_body(&[
            "--account-class-custom",
            CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH,
        ])
        .await;
    }
}
