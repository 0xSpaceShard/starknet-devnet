pub mod common;

mod impersonated_account_tests {
    use std::sync::Arc;

    use server::test_utils::exported_test_utils::assert_contains;
    use starknet_core::constants::STRK_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::{BlockId, BlockTag, ExecutionResult, Felt};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::jsonrpc::HttpTransport;
    use starknet_rs_providers::{JsonRpcClient, Provider};
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::felt_from_prefixed_hex;
    use starknet_types::rpc::transaction_receipt::FeeUnit;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::utils::{
        get_simple_contract_in_sierra_and_compiled_class_hash, ImpersonationAction,
    };

    const IMPERSONATED_ACCOUNT_PRIVATE_KEY: Felt = Felt::ONE;
    // Felt::from(100000000000)
    const AMOUNT_TO_TRANSFER: Felt = Felt::from_raw([
        576406352303423504,
        18446744073709551615,
        18446744073709551615,
        18446740873709551617,
    ]);

    async fn get_account_for_impersonation_and_private_key(
        devnet: &BackgroundDevnet,
    ) -> (Felt, LocalWallet) {
        let (_, account_address) = devnet.get_first_predeployed_account().await;
        (
            account_address,
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
                IMPERSONATED_ACCOUNT_PRIVATE_KEY,
            )),
        )
    }

    fn get_invoke_transaction_request(amount_to_transfer: Felt) -> Call {
        Call {
            to: felt_from_prefixed_hex(STRK_ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE,          // recipient
                amount_to_transfer, // low part of uint256
                Felt::ZERO,         // high part of uint256
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
            origin_devnet.json_rpc_client.chain_id().await.unwrap(),
            ExecutionEncoding::New,
        )
    }

    #[tokio::test]
    async fn test_account_impersonation_have_to_return_an_error_when_account_impersonation_is_disabled()
     {
        let origin_devnet =
            BackgroundDevnet::spawn_forkable_devnet().await.expect("Could not start Devnet");

        let args = [
            "--fork-network",
            origin_devnet.url.as_str(),
            "--accounts",
            "0",
            "--disable-account-impersonation",
        ];
        let forked_devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        let impersonation_err = forked_devnet
            .execute_impersonation_action(&ImpersonationAction::ImpersonateAccount(Felt::ONE))
            .await
            .unwrap_err();

        assert_anyhow_error_contains_message(
            impersonation_err,
            "account impersonation is disabled",
        );

        let impersonation_err = forked_devnet
            .execute_impersonation_action(&ImpersonationAction::AutoImpersonate)
            .await
            .unwrap_err();

        assert_anyhow_error_contains_message(
            impersonation_err,
            "account impersonation is disabled",
        );
    }

    #[tokio::test]
    async fn test_impersonated_of_a_predeployed_account_account_can_send_transaction() {
        let devnet =
            BackgroundDevnet::spawn_forkable_devnet().await.expect("Could not start Devnet");
        let (_, account_address) = devnet.get_first_predeployed_account().await;

        test_invoke_transaction(
            &devnet,
            &[ImpersonationAction::ImpersonateAccount(account_address)],
        )
        .await
        .unwrap();

        test_declare_transaction(
            &devnet,
            &[ImpersonationAction::ImpersonateAccount(account_address)],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn non_impersonated_account_fails_to_make_a_transaction_and_receives_an_error_of_invalid_signature()
     {
        let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

        let invoke_txn_err = test_invoke_transaction(&origin_devnet, &[]).await.unwrap_err();
        assert_anyhow_error_contains_message(invoke_txn_err, "invalid signature");

        let declare_txn_err = test_declare_transaction(&origin_devnet, &[]).await.unwrap_err();
        assert_anyhow_error_contains_message(declare_txn_err, "invalid signature");
    }

    #[tokio::test]
    async fn test_auto_impersonate_allows_user_to_send_transactions() {
        let devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();
        test_invoke_transaction(&devnet, &[ImpersonationAction::AutoImpersonate]).await.unwrap();

        test_declare_transaction(&devnet, &[ImpersonationAction::AutoImpersonate]).await.unwrap();
    }

    #[tokio::test]
    async fn test_impersonate_account_and_then_stop_impersonate_have_to_return_an_error_of_invalid_signature()
     {
        let origin_devnet = &BackgroundDevnet::spawn_forkable_devnet().await.unwrap();
        let (_, account_address) = origin_devnet.get_first_predeployed_account().await;
        let invoke_txn_err = test_invoke_transaction(
            origin_devnet,
            &[
                ImpersonationAction::ImpersonateAccount(account_address),
                ImpersonationAction::StopImpersonateAccount(account_address),
            ],
        )
        .await
        .unwrap_err();

        assert_anyhow_error_contains_message(invoke_txn_err, "invalid signature");

        let declare_txn_err = test_declare_transaction(
            origin_devnet,
            &[
                ImpersonationAction::ImpersonateAccount(account_address),
                ImpersonationAction::StopImpersonateAccount(account_address),
            ],
        )
        .await
        .unwrap_err();
        assert_anyhow_error_contains_message(declare_txn_err, "invalid signature");
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_stop_and_send_transaction_fails_with_invalid_signature_error()
     {
        let origin_devnet = &BackgroundDevnet::spawn_forkable_devnet().await.unwrap();

        let invoke_txn_err = test_invoke_transaction(
            origin_devnet,
            &[ImpersonationAction::AutoImpersonate, ImpersonationAction::StopAutoImpersonate],
        )
        .await
        .unwrap_err();
        assert_anyhow_error_contains_message(invoke_txn_err, "invalid signature");

        let declare_txn_err = test_declare_transaction(
            origin_devnet,
            &[ImpersonationAction::AutoImpersonate, ImpersonationAction::StopAutoImpersonate],
        )
        .await
        .unwrap_err();
        assert_anyhow_error_contains_message(declare_txn_err, "invalid signature");
    }

    #[tokio::test]
    async fn test_simulate_transaction() {
        let origin_devnet = BackgroundDevnet::spawn_forkable_devnet().await.unwrap();
        let forked_devnet = origin_devnet.fork().await.unwrap();

        let account =
            get_account_interacting_with_forked_devnet(&origin_devnet, &forked_devnet).await;

        let invoke_calls = vec![get_invoke_transaction_request(AMOUNT_TO_TRANSFER)];

        // vector of tuples of steps (impersonation action, do validation, expected some error
        // message - none if its successful)
        let steps = vec![
            (Some(ImpersonationAction::ImpersonateAccount(account.address())), true, None),
            (Some(ImpersonationAction::ImpersonateAccount(account.address())), false, None),
            (None, false, None),
            (None, true, Some("invalid signature")),
            (Some(ImpersonationAction::AutoImpersonate), true, None),
            (Some(ImpersonationAction::AutoImpersonate), false, None),
        ];

        for (impersonation_action_option, do_validate, expected_error_message) in steps {
            if let Some(impersonation_action) = impersonation_action_option {
                forked_devnet.execute_impersonation_action(&impersonation_action).await.unwrap();
            }

            let simulation_result =
                account.execute_v1(invoke_calls.clone()).simulate(!do_validate, false).await;
            if let Some(error_msg) = expected_error_message {
                let simulation_err = simulation_result.expect_err("Expected simulation to fail");
                assert_contains(&format!("{:?}", simulation_err).to_lowercase(), error_msg);
            } else {
                simulation_result.expect("Expected simulation to succeed");
            }

            forked_devnet.restart().await;
        }
    }

    async fn test_declare_transaction(
        origin_devnet: &BackgroundDevnet,
        impersonation_actions: &[ImpersonationAction],
    ) -> Result<(), anyhow::Error> {
        let forked_devnet = origin_devnet.fork().await.unwrap();

        let mut account =
            get_account_interacting_with_forked_devnet(origin_devnet, &forked_devnet).await;

        for action in impersonation_actions.iter() {
            forked_devnet.execute_impersonation_action(action).await?;
        }

        let (flattened_class, compiled_class_hash) =
            get_simple_contract_in_sierra_and_compiled_class_hash();

        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        account.declare_v2(Arc::new(flattened_class), compiled_class_hash).send().await?;

        Ok(())
    }

    async fn test_invoke_transaction(
        origin_devnet: &BackgroundDevnet,
        impersonation_actions: &[ImpersonationAction],
    ) -> Result<(), anyhow::Error> {
        let forked_devnet = origin_devnet.fork().await?;

        let account =
            get_account_interacting_with_forked_devnet(origin_devnet, &forked_devnet).await;

        for action in impersonation_actions.iter() {
            forked_devnet.execute_impersonation_action(action).await?;
        }

        let forked_account_initial_balance = forked_devnet
            .get_balance_by_tag(&account.address(), FeeUnit::FRI, BlockTag::Latest)
            .await?;

        let invoke_call = get_invoke_transaction_request(AMOUNT_TO_TRANSFER);

        let result = account.execute_v1(vec![invoke_call]).send().await?;

        let receipt = forked_devnet
            .json_rpc_client
            .get_transaction_receipt(result.transaction_hash)
            .await?
            .receipt;

        assert_eq!(receipt.execution_result(), &ExecutionResult::Succeeded);

        let forked_account_balance = forked_devnet
            .get_balance_by_tag(&account.address(), FeeUnit::FRI, BlockTag::Latest)
            .await?;
        assert!(forked_account_initial_balance >= AMOUNT_TO_TRANSFER + forked_account_balance);

        Ok(())
    }

    fn assert_anyhow_error_contains_message(error: anyhow::Error, message: &str) {
        let error_string = format!("{:?}", error.root_cause()).to_lowercase();
        assert_contains(&error_string, message);
    }
}
