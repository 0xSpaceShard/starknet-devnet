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
        BlockId, BlockTag, DeployAccountTransactionResult, FieldElement, FunctionCall,
        MaybePendingTransactionReceipt, TransactionFinalityStatus, TransactionReceipt,
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
    async fn spawnable_with_custom_account_cairo_1() {
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

        let config = devnet.get_config().await.unwrap();
        assert_eq!(config["account_contract_class_hash"], expected_hash);
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

    /// Utility for deploying accounts of `class_hash`
    async fn deploy_account(
        devnet: &BackgroundDevnet,
        class_hash: FieldElement,
    ) -> DeployAccountTransactionResult {
        let signer = get_deployable_account_signer();

        let account_factory =
            OpenZeppelinAccountFactory::new(class_hash, CHAIN_ID, signer, devnet.clone_provider())
                .await
                .unwrap();

        let salt = FieldElement::THREE;
        let deployment = account_factory
            .deploy(salt)
            .max_fee(FieldElement::from(1e18 as u128))
            .nonce(FieldElement::ZERO);
        let account_address = deployment.address();
        devnet.mint(account_address, 1e18 as u128).await;

        let account_deployment = deployment.send().await.unwrap();
        assert_eq!(account_deployment.contract_address, account_address);
        account_deployment
    }

    /// Common body for tests defined below
    async fn can_deploy_new_account_test_body(devnet_args: &[&str]) {
        let devnet = BackgroundDevnet::spawn_with_additional_args(devnet_args).await.unwrap();

        let class_hash = FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap();
        let account_deployment = deploy_account(&devnet, class_hash).await;

        let deploy_account_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(account_deployment.transaction_hash)
            .await
            .unwrap();

        match deploy_account_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::DeployAccount(receipt)) => {
                assert_eq!(receipt.finality_status, TransactionFinalityStatus::AcceptedOnL2);
            }
            _ => panic!("Invalid receipt {:?}", deploy_account_receipt),
        }
    }

    #[tokio::test]
    async fn can_deploy_new_cairo1_account() {
        can_deploy_new_account_test_body(&["--account-class", "cairo1"]).await;
    }

    #[tokio::test]
    async fn can_deploy_new_cairo1_account_when_cairo0_selected() {
        can_deploy_new_account_test_body(&["--account-class", "cairo0"]).await;
    }

    #[tokio::test]
    async fn can_deploy_new_custom_account() {
        let args = ["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH];
        can_deploy_new_account_test_body(&args).await;
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
        account
            .execute(invoke_calls)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

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
        let args = ["--account-class-custom", CAIRO_1_ACCOUNT_CONTRACT_SIERRA_PATH];
        can_declare_deploy_invoke_using_predeployed_test_body(&args).await;
    }

    async fn assert_supports_isrc6(devnet: &BackgroundDevnet, account_address: FieldElement) {
        // https://github.com/OpenZeppelin/cairo-contracts/blob/89a450a88628ec3b86273f261b2d8d1ca9b1522b/src/account/interface.cairo#L7
        let interface_id_hex = "0x2ceccef7f994940b3962a6c67e0ba4fcd37df7d131417c604f91e03caecc1cd";
        let interface_id = FieldElement::from_hex_be(interface_id_hex).unwrap();

        let call = FunctionCall {
            contract_address: account_address,
            entry_point_selector: get_selector_from_name("supports_interface").unwrap(),
            calldata: vec![interface_id],
        };

        let supports =
            devnet.json_rpc_client.call(call, BlockId::Tag(BlockTag::Latest)).await.unwrap();
        assert_eq!(supports, vec![FieldElement::ONE]);
    }

    #[tokio::test]
    async fn test_interface_support_of_predeployed_account() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();
        let (_, account_address) = devnet.get_first_predeployed_account().await;

        assert_supports_isrc6(&devnet, account_address).await;
    }

    #[tokio::test]
    async fn test_interface_support_of_newly_deployed_account() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let class_hash = FieldElement::from_hex_be(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH).unwrap();
        let account_deployment = deploy_account(&devnet, class_hash).await;

        assert_supports_isrc6(&devnet, account_deployment.contract_address).await;
    }
}
