pub mod common;

mod estimate_fee_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::contract::SierraClass;
    use starknet_rs_core::types::FieldElement;

    use crate::common::constants::{CAIRO_1_CONTRACT_PATH, CASM_COMPILED_CLASS_HASH, CHAIN_ID};
    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{get_predeployed_account_props, load_json, resolve_path};

    fn extract_overall_fee(simulation_result: &serde_json::Value) -> u128 {
        let fee_hex = simulation_result["fee_estimation"]["overall_fee"].as_str().unwrap();
        let fee_hex_stripped = fee_hex.strip_prefix("0x").unwrap();
        u128::from_str_radix(fee_hex_stripped, 16).unwrap()
    }

    fn make_declaration_assertions(
        resp_no_flags: &serde_json::Value,
        resp_skip_validation: &serde_json::Value,
        sender_address_hex: &str,
    ) {
        let no_flags_trace = &resp_no_flags["transaction_trace"];
        assert_eq!(
            no_flags_trace["validate_invocation"]["contract_address"].as_str().unwrap(),
            sender_address_hex
        );
        assert!(no_flags_trace["fee_transfer_invocation"].as_object().is_none());

        let skip_validation_trace = &resp_skip_validation["transaction_trace"];
        assert!(skip_validation_trace["validate_invocation"].as_object().is_none());
        assert!(skip_validation_trace["fee_transfer_invocation"].as_object().is_none());

        let no_flags_fee = extract_overall_fee(resp_no_flags);
        let skip_validation_fee = extract_overall_fee(resp_skip_validation);
        assert!(no_flags_fee.ge(&skip_validation_fee)); // TODO should be .gt, reported in https://github.com/lambdaclass/starknet_in_rust/issues/1051
    }

    #[tokio::test]
    async fn simulate_declare_v1() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        );

        let contract_json = dummy_cairo_0_contract_class();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());

        let max_fee = FieldElement::ZERO;
        let nonce = FieldElement::ZERO;

        let signature = account
            .declare_legacy(contract_artifact.clone())
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .get_declare_request()
            .await
            .unwrap()
            .signature;
        let signature_hex: Vec<String> = signature.iter().map(|s| format!("{s:#x}")).collect();

        let sender_address_hex = format!("{account_address:#x}");

        let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
            json!({
                "block_id": "latest",
                "simulation_flags": simulation_flags,
                "transactions": [
                    {
                        "type": "DECLARE",
                        "sender_address": sender_address_hex,
                        "max_fee": format!("{max_fee:#x}"),
                        "version": "0x1",
                        "signature": signature_hex,
                        "nonce": format!("{nonce:#x}"),
                        "contract_class": contract_artifact.compress().unwrap(),
                    }
                ]
            })
        };

        let params_no_flags = get_params(&[]);
        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags)
            .await["result"][0];

        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await["result"][0];

        make_declaration_assertions(resp_no_flags, resp_skip_validation, &sender_address_hex);
    }

    #[tokio::test]
    async fn simulate_declare_v2() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = get_predeployed_account_props();
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        );

        let contract_artifact_path = resolve_path(CAIRO_1_CONTRACT_PATH);
        let contract_artifact: SierraClass = load_json(&contract_artifact_path);
        let flattened_contract_artifact = contract_artifact.clone().flatten().unwrap();
        let compiled_class_hash = FieldElement::from_hex_be(CASM_COMPILED_CLASS_HASH).unwrap();

        let max_fee = FieldElement::ZERO;
        let nonce = FieldElement::ZERO;

        let signature = account
            .declare(Arc::new(flattened_contract_artifact), compiled_class_hash)
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .get_declare_request()
            .await
            .unwrap()
            .signature;
        let signature_hex: Vec<String> = signature.iter().map(|s| format!("{s:#x}")).collect();

        let sender_address_hex = format!("{account_address:#x}");

        let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
            json!({
                "block_id": "latest",
                "simulation_flags": simulation_flags,
                "transactions": [
                    {
                        "type": "DECLARE",
                        "sender_address": sender_address_hex,
                        "compiled_class_hash": format!("{compiled_class_hash:#x}"),
                        "max_fee": format!("{max_fee:#x}"),
                        "version": "0x2",
                        "signature": signature_hex,
                        "nonce": format!("{nonce:#x}"),
                        "contract_class": contract_artifact,
                    }
                ]
            })
        };

        let params_no_flags = get_params(&[]);
        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags)
            .await["result"][0];

        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await["result"][0];

        make_declaration_assertions(resp_no_flags, resp_skip_validation, &sender_address_hex);
    }

    #[tokio::test]
    async fn simulate_deploy_account() {
        //
    }

    #[tokio::test]
    async fn simulate_invoke() {
        //
    }
}
