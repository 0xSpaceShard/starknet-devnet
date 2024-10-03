#![cfg(test)]
pub mod common;

mod trace_tests {
    use std::sync::Arc;

    use server::test_utils::assert_contains;
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{Call, Felt, InvokeTransactionResult, StarknetError};
    use starknet_rs_core::utils::get_selector_from_name;
    use starknet_rs_providers::ProviderError;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH, CHAIN_ID, INVALID_ACCOUNT_SIERRA_PATH,
    };
    use crate::common::utils::{
        declare_deploy_v1, get_flattened_sierra_contract_and_casm_hash,
        get_simple_contract_in_sierra_and_compiled_class_hash,
    };

    #[tokio::test]
    async fn test_failed_validation_with_expected_message() {
        let args = ["--account-class-custom", INVALID_ACCOUNT_SIERRA_PATH];
        let devnet = BackgroundDevnet::spawn_with_additional_args(&args).await.unwrap();

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        ));

        // get class
        let contract_json = dummy_cairo_0_contract_class();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());

        // declare class
        let declaration_result = account
            .declare_legacy(contract_artifact.clone())
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await;

        match declaration_result {
            Err(AccountError::Provider(ProviderError::StarknetError(
                StarknetError::ValidationFailure(message),
            ))) => assert_contains(&message, "FAILED VALIDATE DECLARE"),
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_declaration_rejected_if_casm_hash_not_matching() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (contract_class, _) = get_simple_contract_in_sierra_and_compiled_class_hash();
        let dummy_casm_hash = Felt::ONE;

        let declaration_result = account
            .declare_v2(Arc::new(contract_class), dummy_casm_hash)
            .nonce(Felt::ZERO)
            .send()
            .await;

        match declaration_result {
            Err(AccountError::Provider(ProviderError::StarknetError(
                StarknetError::CompiledClassHashMismatch,
            ))) => (),
            other => panic!("Unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_tx_status_content_on_failure() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (sierra, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let (_, contract_address) =
            declare_deploy_v1(account.clone(), sierra, casm_hash, &[]).await.unwrap();

        let InvokeTransactionResult { transaction_hash } = account
            .execute_v1(vec![Call {
                to: contract_address,
                selector: get_selector_from_name("create_panic").unwrap(),
                calldata: vec![],
            }])
            .max_fee(Felt::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // TODO sending a custom request until starknet-rs is adapted to include failure reason
        let tx_status = devnet
            .send_custom_rpc(
                "starknet_getTransactionStatus",
                serde_json::json!({ "transaction_hash": transaction_hash }),
            )
            .await
            .unwrap();

        assert_eq!(tx_status["finality_status"], "ACCEPTED_ON_L2");
        assert_contains(
            tx_status["failure_reason"].as_str().unwrap(),
            "Error in the called contract",
        );
        assert_eq!(tx_status["execution_status"], "REVERTED");
    }
}
