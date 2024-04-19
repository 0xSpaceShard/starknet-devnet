pub mod common;

mod impersonated_account_tests {

    use std::sync::Arc;

    use starknet_core::constants::STRK_ERC20_CONTRACT_ADDRESS;
    use starknet_rs_accounts::{
        Account, AccountError, Call, ExecutionEncoding, SingleOwnerAccount,
    };
    use starknet_rs_core::types::contract::{CompiledClass, SierraClass};
    use starknet_rs_core::types::{
        BlockId, BlockTag, ContractErrorData, ExecutionResult, FieldElement, StarknetError,
    };
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;
    use starknet_types::rpc::transaction_receipt::FeeUnit;
    use starknet_types::traits::ToHexString;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        self, CAIRO_1_CASM_PATH, CAIRO_1_CONTRACT_PATH, CASM_COMPILED_CLASS_HASH,
    };
    use crate::common::utils::spawn_forkable_devnet;

    const IMPERSONATED_ACCOUNT_PRIVATE_KEY: FieldElement = FieldElement::ONE;

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

    #[tokio::test]
    async fn non_impersonated_account_fails_to_make_a_transaction_and_receives_an_error_of_invalid_signature()
     {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let forked_devnet = origin_devnet.fork().await.unwrap();
        let (account_address, private_key) =
            get_account_for_impersonation_and_private_key(&origin_devnet).await;

        let amount_to_transfer = FieldElement::from_dec_str("100000000000").unwrap();

        let account = SingleOwnerAccount::new(
            &forked_devnet.json_rpc_client,
            private_key,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let invoke_call = Call {
            to: FieldElement::from_hex_be(STRK_ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                FieldElement::ONE,  // recipient
                amount_to_transfer, // low part of uint256
                FieldElement::ZERO, // high part of uint256
            ],
        };

        let error = account.execute(vec![invoke_call]).send().await.err().unwrap();
        match error {
            AccountError::Provider(ProviderError::StarknetError(StarknetError::ContractError(
                ContractErrorData { revert_error },
            ))) => {
                assert!(revert_error.to_lowercase().contains("invalid signature"));
            }
            _ => panic!("Expected an error of invalid signature"),
        }
    }

    #[tokio::test]
    async fn test_impersonated_account_of_a_predeployed_account_can_create_transfer() {
        let origin_devnet = spawn_forkable_devnet().await.unwrap();
        let (account_address, private_key) =
            get_account_for_impersonation_and_private_key(&origin_devnet).await;

        let forked_devnet = origin_devnet.fork().await.unwrap();
        forked_devnet
            .impersonate_account(crate::common::utils::ImpersonationAction::ImpersonateAccount(
                account_address,
            ))
            .await
            .unwrap();

        let forked_account_initial_balance =
            forked_devnet.get_balance(&account_address, FeeUnit::FRI).await.unwrap();

        let amount_to_transfer = FieldElement::from_dec_str("100000000000").unwrap();

        let account = SingleOwnerAccount::new(
            &forked_devnet.json_rpc_client,
            private_key,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let invoke_call = Call {
            to: FieldElement::from_hex_be(STRK_ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                FieldElement::ONE,  // recipient
                amount_to_transfer, // low part of uint256
                FieldElement::ZERO, // high part of uint256
            ],
        };

        let result = account.execute(vec![invoke_call]).send().await.unwrap();

        let receipt = forked_devnet
            .json_rpc_client
            .get_transaction_receipt(result.transaction_hash)
            .await
            .unwrap();

        assert_eq!(receipt.execution_result(), &ExecutionResult::Succeeded);

        let forked_account_balance =
            forked_devnet.get_balance(&account_address, FeeUnit::FRI).await.unwrap();
        assert!(forked_account_initial_balance >= amount_to_transfer + forked_account_balance);
    }

    #[tokio::test]
    async fn test_impersonated_of_a_predeployed_account_account_can_send_declare_transaction() {
        let devnet = spawn_forkable_devnet().await.expect("Could not start Devnet");
        let forked_devnet = devnet.fork().await.unwrap();
        let sierra_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), CAIRO_1_CONTRACT_PATH);
        let contract_artifact: SierraClass =
            serde_json::from_reader(std::fs::File::open(sierra_path).unwrap()).unwrap();

        let casm_path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), CAIRO_1_CASM_PATH);
        let casm_contract_definition: CompiledClass =
            serde_json::from_reader(std::fs::File::open(casm_path).unwrap()).unwrap();
        let compiled_class_hash = (casm_contract_definition.class_hash()).unwrap();
        assert_eq!(Felt::from(compiled_class_hash).to_prefixed_hex_str(), CASM_COMPILED_CLASS_HASH);

        let (_, address) = devnet.get_first_predeployed_account().await;
        forked_devnet
            .impersonate_account(crate::common::utils::ImpersonationAction::ImpersonateAccount(
                address,
            ))
            .await
            .unwrap();

        let mut account = SingleOwnerAccount::new(
            &forked_devnet.json_rpc_client,
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(
                IMPERSONATED_ACCOUNT_PRIVATE_KEY,
            )),
            address,
            constants::CHAIN_ID,
            ExecutionEncoding::Legacy,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        // We need to flatten the ABI into a string first
        let flattened_class = contract_artifact.flatten().unwrap();

        let declare_result =
            account.declare(Arc::new(flattened_class), compiled_class_hash).send().await.unwrap();

        crate::common::utils::assert_tx_successful(
            &declare_result.transaction_hash,
            &forked_devnet.json_rpc_client,
        )
        .await;
    }

    #[tokio::test]
    async fn test_auto_impersonate_and_send_invoke_transaction() {
        // Test scenario 1: Auto impersonate account and send invoke transaction
        // Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_and_send_declare_transaction() {
        // Test scenario 2: Auto impersonate account and send declare transaction
        // Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_stop_and_send_invoke_transaction() {
        // Test scenario 3: Auto impersonate account then stop impersonating and send invoke
        // transaction Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_stop_and_send_declare_transaction() {
        // Test scenario 4: Auto impersonate account then stop impersonating and send declare
        // transaction Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_impersonate_and_send_invoke_transaction() {
        // Test scenario 5: Auto impersonate account then impersonate account and send invoke
        // transaction Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_impersonate_and_send_declare_transaction() {
        // Test scenario 6: Auto impersonate account then impersonate account and send declare
        // transaction Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_impersonate_then_stop_and_send_invoke_transaction() {
        // Test scenario 7: Auto impersonate account then impersonate account then stop
        // impersonating and send invoke transaction Your test code here...
    }

    #[tokio::test]
    async fn test_auto_impersonate_then_impersonate_then_stop_and_send_declare_transaction() {
        // Test scenario 8: Auto impersonate account then impersonate account then stop
        // impersonating and send declare transaction Your test code here...
    }
}
