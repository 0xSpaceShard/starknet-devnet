pub mod common;

mod impersonated_account_tests {
    use std::sync::Arc;

    use starknet_core::constants::STRK_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::contract::{CompiledClass, SierraClass};
    use starknet_rs_core::types::{BlockId, BlockTag, ExecutionResult, FieldElement};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{JsonRpcClient, Provider};
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transaction_receipt::FeeUnit;
    use starknet_types::traits::ToHexString;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        self, CAIRO_1_CASM_PATH, CAIRO_1_CONTRACT_PATH, CASM_COMPILED_CLASS_HASH,
    };
    use crate::common::utils::{spawn_forkable_devnet, ImpersonationAction};

    const IMPERSONATED_ACCOUNT_PRIVATE_KEY: FieldElement = FieldElement::ONE;
    // FieldElement::from_dec_str("100000000000")
    const AMOUNT_TO_TRANSFER: FieldElement = FieldElement::from_mont([
        18446740873709551617,
        18446744073709551615,
        18446744073709551615,
        576406352303423504,
    ]);

    #[derive(Clone)]
    enum TestCaseResult {
        Success,
        Failure { msg: String },
    }

    async fn get_account_for_impersonation_and_private_key(
        devnet: &BackgroundDevnet,
    ) -> (FieldElement, LocalWallet) {
        let (_, account_address) = devnet.get_first_predeployed_account().await;
        (
            account_address,
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
                IMPERSONATED_ACCOUNT_PRIVATE_KEY,
            )),
        )
    }

    fn get_invoke_transaction_request(amount_to_transfer: FieldElement) -> Call {
        Call {
            to: FieldElement::from_hex_be(STRK_ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                FieldElement::ONE,  // recipient
                amount_to_transfer, // low part of uint256
                FieldElement::ZERO, // high part of uint256
            ],
        }
    }

    async fn get_account_interacting_with_forked_devnet<'a>(
        origin_devnet: &'a BackgroundDevnet,
        forked_devnet: &'a BackgroundDevnet,
    ) -> SingleOwnerAccount<&'a JsonRpcClient<HttpTransport>, LocalWallet> {
        let (account_address, private_key) =
            get_account_for_impersonation_and_private_key(origin_devnet).await;

        SingleOwnerAccount::new(
            &forked_devnet.json_rpc_client,
            private_key,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        )
    }

    #[tokio::test]
    async fn test_impersonated_of_a_predeployed_account_account_can_send_transaction() {
        let devnet = spawn_forkable_devnet().await.expect("Could not start Devnet");
        let (_, account_address) = devnet.get_first_predeployed_account().await;

        test_invoke_transaction(
            &devnet,
            &[ImpersonationAction::ImpersonateAccount(account_address)],
            TestCaseResult::Success,
        )
        .await;

        test_declare_transaction(
            &devnet,
            &[ImpersonationAction::ImpersonateAccount(account_address)],
            TestCaseResult::Success,
        )
        .await;
    }

    #[tokio::test]
    async fn non_impersonated_account_fails_to_make_a_transaction_and_receives_an_error_of_invalid_signature()
     {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let expected_result = TestCaseResult::Failure { msg: "invalid signature".to_string() };

        test_invoke_transaction(&origin_devnet, &[], expected_result.clone()).await;

        test_declare_transaction(&origin_devnet, &[], expected_result).await;
    }

    #[tokio::test]
    async fn test_auto_impersonate_allows_user_to_send_transactions() {
        let devnet = spawn_forkable_devnet().await.unwrap();
        test_invoke_transaction(
            &devnet,
            &[ImpersonationAction::AutoImpersonate],
            TestCaseResult::Success,
        )
        .await;

        test_declare_transaction(
            &devnet,
            &[ImpersonationAction::AutoImpersonate],
            TestCaseResult::Success,
        )
        .await
    }

    #[tokio::test]
    async fn test_impersonate_account_and_then_stop_impersonate_have_to_return_an_error_of_invalid_signature()
     {
        let origin_devnet = &spawn_forkable_devnet().await.unwrap();
        let (_, account_address) = origin_devnet.get_first_predeployed_account().await;
        let expected_result = TestCaseResult::Failure { msg: "invalid signature".to_string() };
        test_invoke_transaction(
            origin_devnet,
            &[
                ImpersonationAction::ImpersonateAccount(account_address),
                ImpersonationAction::StopImpersonatingAccount(account_address),
            ],
            expected_result.clone(),
        )
        .await;

        test_declare_transaction(
            origin_devnet,
            &[
                ImpersonationAction::ImpersonateAccount(account_address),
                ImpersonationAction::StopImpersonatingAccount(account_address),
            ],
            expected_result.clone(),
        )
        .await
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_stop_and_send_transaction_fails_with_invalid_signature_error()
     {
        let origin_devnet = &spawn_forkable_devnet().await.unwrap();
        let expected_result = TestCaseResult::Failure { msg: "invalid signature".to_string() };
        test_invoke_transaction(
            origin_devnet,
            &[ImpersonationAction::AutoImpersonate, ImpersonationAction::StopAutoImpersonate],
            expected_result.clone(),
        )
        .await;

        test_declare_transaction(
            origin_devnet,
            &[ImpersonationAction::AutoImpersonate, ImpersonationAction::StopAutoImpersonate],
            expected_result,
        )
        .await;
    }

    #[tokio::test]
    async fn test_simulate_transaction() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let forked_devnet = origin_devnet.fork().await.unwrap();

        let account =
            get_account_interacting_with_forked_devnet(&origin_devnet, &forked_devnet).await;

        let invoke_calls = vec![get_invoke_transaction_request(AMOUNT_TO_TRANSFER)];

        // vector of tuples of steps (impersonation action, do validation, expected_result)
        let steps = vec![
            (
                Some(ImpersonationAction::ImpersonateAccount(account.address())),
                true,
                TestCaseResult::Success,
            ),
            (
                Some(ImpersonationAction::ImpersonateAccount(account.address())),
                false,
                TestCaseResult::Success,
            ),
            (None, false, TestCaseResult::Success),
            (None, true, TestCaseResult::Failure { msg: "invalid signature".to_string() }),
            (Some(ImpersonationAction::AutoImpersonate), true, TestCaseResult::Success),
            (Some(ImpersonationAction::AutoImpersonate), false, TestCaseResult::Success),
        ];

        for (impersonation_action_option, do_validate, expected_result) in steps {
            if let Some(impersonation_action) = impersonation_action_option {
                forked_devnet.execute_impersonation_action(&impersonation_action).await.unwrap();
            }

            let simulation_result =
                account.execute(invoke_calls.clone()).simulate(!do_validate, false).await;
            match expected_result {
                TestCaseResult::Success => {
                    simulation_result.expect("Expected simulation to succeed");
                }
                TestCaseResult::Failure { msg } => {
                    let err = simulation_result.err().unwrap();
                    assert!(format!("{:?}", err).to_lowercase().contains(&msg));
                }
            }

            forked_devnet.restart().await.unwrap();
        }
    }

    async fn test_declare_transaction(
        origin_devnet: &BackgroundDevnet,
        impersonation_actions: &[ImpersonationAction],
        expected_result: TestCaseResult,
    ) {
        let forked_devnet = origin_devnet.fork().await.unwrap();

        let mut account =
            get_account_interacting_with_forked_devnet(origin_devnet, &forked_devnet).await;

        for action in impersonation_actions.iter() {
            forked_devnet.execute_impersonation_action(action).await.unwrap();
        }

        let sierra_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), CAIRO_1_CONTRACT_PATH);
        let contract_artifact: SierraClass =
            serde_json::from_reader(std::fs::File::open(sierra_path).unwrap()).unwrap();

        let casm_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), CAIRO_1_CASM_PATH);
        let casm_contract_definition: CompiledClass =
            serde_json::from_reader(std::fs::File::open(casm_path).unwrap()).unwrap();
        let compiled_class_hash = (casm_contract_definition.class_hash()).unwrap();
        assert_eq!(Felt::from(compiled_class_hash).to_prefixed_hex_str(), CASM_COMPILED_CLASS_HASH);

        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        // We need to flatten the ABI into a string first
        let flattened_class = contract_artifact.flatten().unwrap();

        let declare_result =
            account.declare(Arc::new(flattened_class), compiled_class_hash).send().await;

        match expected_result {
            TestCaseResult::Success => {
                crate::common::utils::assert_tx_successful(
                    &declare_result.unwrap().transaction_hash,
                    &forked_devnet.json_rpc_client,
                )
                .await;
            }
            TestCaseResult::Failure { msg } => {
                let err = declare_result.err().unwrap();
                assert!(format!("{:?}", err).to_lowercase().contains(&msg));
            }
        }
    }

    async fn test_invoke_transaction(
        origin_devnet: &BackgroundDevnet,
        impersonation_actions: &[ImpersonationAction],
        expected_result: TestCaseResult,
    ) {
        let forked_devnet = origin_devnet.fork().await.unwrap();

        let account =
            get_account_interacting_with_forked_devnet(origin_devnet, &forked_devnet).await;

        for action in impersonation_actions.iter() {
            forked_devnet.execute_impersonation_action(action).await.unwrap();
        }

        let forked_account_initial_balance =
            forked_devnet.get_balance(&account.address(), FeeUnit::FRI).await.unwrap();

        let invoke_call = get_invoke_transaction_request(AMOUNT_TO_TRANSFER);

        let result = account.execute(vec![invoke_call]).send().await;

        match expected_result {
            TestCaseResult::Success => {
                let result = result.unwrap();

                let receipt = forked_devnet
                    .json_rpc_client
                    .get_transaction_receipt(result.transaction_hash)
                    .await
                    .unwrap();

                assert_eq!(receipt.execution_result(), &ExecutionResult::Succeeded);
                let forked_account_balance =
                    forked_devnet.get_balance(&account.address(), FeeUnit::FRI).await.unwrap();
                assert!(
                    forked_account_initial_balance >= AMOUNT_TO_TRANSFER + forked_account_balance
                );
            }
            TestCaseResult::Failure { msg } => {
                let err = result.err().unwrap();
                assert!(format!("{:?}", err).to_lowercase().contains(&msg));
            }
        }
    }
}
