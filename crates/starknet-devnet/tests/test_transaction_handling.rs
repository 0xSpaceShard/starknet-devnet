pub mod common;

mod trace_tests {
    use std::sync::Arc;

    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{Felt, StarknetError};
    use starknet_rs_providers::ProviderError;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{CHAIN_ID, INVALID_ACCOUNT_SIERRA_PATH};
    use crate::common::utils::get_simple_contract_in_sierra_and_compiled_class_hash;

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
            ))) => {
                assert_eq!(
                    message,
                    "Execution failed. Failure reason: \
                     0x4641494c45442056414c4944415445204445434c415245 ('FAILED VALIDATE DECLARE')."
                );
            }
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
            .declare(Arc::new(contract_class), dummy_casm_hash)
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
}
