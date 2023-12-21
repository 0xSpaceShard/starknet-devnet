pub mod common;

mod trace_tests {
    use std::sync::Arc;

    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{Account, AccountError, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_providers::ProviderError;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{CHAIN_ID, INVALID_ACCOUNT_SIERRA_PATH};

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
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await;

        match declaration_result {
            Err(AccountError::Provider(ProviderError::StarknetError(error))) => {
                assert_eq!(error.message, "Account validation failed");
            }
            other => panic!("Unexpected result: {other:?}"),
        }
    }
}
