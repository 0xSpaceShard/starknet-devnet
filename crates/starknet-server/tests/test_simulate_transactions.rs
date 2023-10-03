pub mod common;

mod estimate_fee_tests {
    use std::sync::Arc;

    use hyper::Body;
    use serde_json::json;
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{Account, ExecutionEncoding, SingleOwnerAccount};
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::FieldElement;

    use crate::common::constants::{CHAIN_ID, RPC_PATH};
    use crate::common::devnet::BackgroundDevnet;
    use crate::common::utils::{get_json_body, get_predeployed_account_props};

    fn extract_overall_fee(simulation_result: &serde_json::Value) -> u128 {
        let fee_hex = simulation_result["fee_estimation"]["overall_fee"].as_str().unwrap();
        let fee_hex_stripped = fee_hex.strip_prefix("0x").unwrap();
        u128::from_str_radix(fee_hex_stripped, 16).unwrap()
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

        let get_request_body = |simulation_flags: &[&str]| -> Body {
            let body_json = json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "starknet_simulateTransactions",
                "params": {
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
                }
            });
            Body::from(body_json.to_string())
        };

        // no flags
        let req_body_no_flags = get_request_body(&[]);
        // devnet.custom_rpc_request(...)
        let resp_no_flags = devnet.post_json(RPC_PATH.into(), req_body_no_flags).await.unwrap();
        let resp_no_flags_body = &get_json_body(resp_no_flags).await["result"][0];
        println!("DEBUG resp_no_flags_body: {resp_no_flags_body:?}");
        assert_eq!(
            resp_no_flags_body["transaction_trace"]["validate_invocation"]["contract_address"]
                .as_str()
                .unwrap(),
            sender_address_hex
        );
        assert!(
            resp_no_flags_body["transaction_trace"]["fee_transfer_invocation"]
                .as_object()
                .is_none()
        );

        // skip validation
        let req_body_skip_validation = get_request_body(&["SKIP_VALIDATE"]);
        let resp_skip_validation =
            devnet.post_json(RPC_PATH.into(), req_body_skip_validation).await.unwrap();
        let resp_skip_validation_body = &get_json_body(resp_skip_validation).await["result"][0];

        assert!(
            resp_skip_validation_body["transaction_trace"]["validate_invocation"]
                .as_object()
                .is_none()
        );
        assert!(
            resp_skip_validation_body["transaction_trace"]["fee_transfer_invocation"]
                .as_object()
                .is_none()
        );

        // fee without flags should be > fee with skipped validation
        let no_flags_fee = extract_overall_fee(resp_no_flags_body);
        let skip_validation_fee = extract_overall_fee(resp_skip_validation_body);
        assert!(no_flags_fee.ge(&skip_validation_fee)); // TODO should be .gt, reported in https://github.com/lambdaclass/starknet_in_rust/issues/1051
    }

    #[tokio::test]
    async fn simulate_declare_v2() {
        //
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
