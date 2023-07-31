pub mod common;

mod get_events_integration_tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use futures::TryStreamExt;
    use starknet_core::constants::CAIRO_0_ACCOUNT_CONTRACT_HASH;
    use starknet_rs_accounts::{Account, SingleOwnerAccount};
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{BlockId, BlockTag, FieldElement, StarknetError};
    use starknet_rs_providers::{Provider, ProviderError, SequencerGatewayProvider};
    use starknet_rs_signers::{LocalWallet, SigningKey};
    use starknet_types::felt::Felt;

    use crate::common::constants::PREDEPLOYED_ACCOUNT_ADDRESS;
    use crate::common::util::{get_json_body, BackgroundDevnet};

    #[tokio::test]
    async fn get_events() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let predeployed_accounts_response =
            devnet.get("/predeployed_accounts", None).await.unwrap();

        let predeployed_accounts_json = get_json_body(predeployed_accounts_response).await;
        let first_account = predeployed_accounts_json.as_array().unwrap().get(0).unwrap();

        let account_address =
            Felt::from_prefixed_hex_str(first_account["address"].as_str().unwrap()).unwrap();
        let private_key =
            Felt::from_prefixed_hex_str(first_account["private_key"].as_str().unwrap()).unwrap();

        let current_project_path = Path::new(env!("CARGO_MANIFEST_DIR"));
        let parent_to_current_project = current_project_path.parent().unwrap();
        let cairo_0_artifact_path =
            parent_to_current_project.join("starknet/test_artifacts/cairo_0_test.json");
        assert!(cairo_0_artifact_path.exists());

        let contract_artifact: LegacyContractClass =
            serde_json::from_reader(std::fs::File::open(cairo_0_artifact_path).unwrap()).unwrap();

        let provider = SequencerGatewayProvider::starknet_alpha_goerli();

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(private_key.into()));
        let address = FieldElement::from(account_address);

        let mut account = SingleOwnerAccount::new(provider, signer, address, chain_id::TESTNET);

        // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
        // block. Optionally change the target block to pending with the following line:
        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        let result = account.declare_legacy(Arc::new(contract_artifact)).send().await.unwrap();

        assert!(false);
    }
}
