pub mod common;

mod estimate_fee_tests {
    use std::sync::Arc;

    use serde_json::json;
    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ERC20_CONTRACT_ADDRESS};
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{
        Account, AccountFactory, Call, ExecutionEncoding, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::FieldElement;
    use starknet_rs_core::utils::{
        get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_signers::Signer;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{CAIRO_1_CONTRACT_PATH, CHAIN_ID};
    use crate::common::utils::{
        get_deployable_account_signer, get_flattened_sierra_contract_and_casm_hash,
        iter_to_hex_felt, to_hex_felt, to_num_as_hex,
    };

    fn extract_overall_fee(simulation_result: &serde_json::Value) -> u128 {
        let fee_hex = simulation_result["fee_estimation"]["overall_fee"].as_str().unwrap();
        let fee_hex_stripped = fee_hex.strip_prefix("0x").unwrap();
        u128::from_str_radix(fee_hex_stripped, 16).unwrap()
    }

    fn assert_fee_in_resp_greater(
        resp_no_flags: &serde_json::Value,
        resp_skip_validation: &serde_json::Value,
    ) {
        let no_flags_fee = extract_overall_fee(resp_no_flags);
        let skip_validation_fee = extract_overall_fee(resp_skip_validation);
        assert!(no_flags_fee.gt(&skip_validation_fee));
    }

    fn assert_difference_if_validation(
        resp_no_flags: &serde_json::Value,
        resp_skip_validation: &serde_json::Value,
        expected_contract_adddress: &str,
        should_skip_fee_invocation: bool,
    ) {
        let no_flags_trace = &resp_no_flags["transaction_trace"];
        assert_eq!(
            no_flags_trace["validate_invocation"]["contract_address"].as_str().unwrap(),
            expected_contract_adddress
        );
        assert!(no_flags_trace["state_diff"].as_object().is_some());

        let skip_validation_trace = &resp_skip_validation["transaction_trace"];
        assert!(skip_validation_trace["validate_invocation"].as_object().is_none());
        assert!(skip_validation_trace["state_diff"].as_object().is_some());

        assert_eq!(
            skip_validation_trace["fee_transfer_invocation"].as_object().is_none(),
            should_skip_fee_invocation
        );
        assert_eq!(
            no_flags_trace["fee_transfer_invocation"].as_object().is_none(),
            should_skip_fee_invocation
        );

        assert_fee_in_resp_greater(resp_no_flags, resp_skip_validation);
    }

    #[tokio::test]
    async fn simulate_declare_v1() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
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

        let max_fee = FieldElement::ZERO; // TODO try 1e18 as u128 instead
        let nonce = FieldElement::ZERO;

        let signature = account
            .declare_legacy(contract_artifact.clone())
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .get_declare_request(false)
            .await
            .unwrap()
            .signature;
        let signature_hex: Vec<String> = iter_to_hex_felt(&signature);

        let sender_address_hex = to_hex_felt(&account_address);

        let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
            json!({
                "block_id": "latest",
                "simulation_flags": simulation_flags,
                "transactions": [
                    {
                        "type": "DECLARE",
                        "sender_address": sender_address_hex,
                        "max_fee": to_hex_felt(&max_fee),
                        "version": "0x1",
                        "signature": signature_hex,
                        "nonce": to_num_as_hex(&nonce),
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

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            max_fee == FieldElement::ZERO,
        );
    }

    #[tokio::test]
    async fn simulate_declare_v2() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            CHAIN_ID,
            ExecutionEncoding::Legacy,
        );

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);

        let max_fee = FieldElement::ZERO;
        let nonce = FieldElement::ZERO;

        let signature = account
            .declare(Arc::new(flattened_contract_artifact.clone()), casm_hash)
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .get_declare_request(false)
            .await
            .unwrap()
            .signature;
        let signature_hex: Vec<String> = iter_to_hex_felt(&signature);

        let sender_address_hex = to_hex_felt(&account_address);

        let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
            json!({
                "block_id": "latest",
                "simulation_flags": simulation_flags,
                "transactions": [
                    {
                        "type": "DECLARE",
                        "sender_address": sender_address_hex,
                        "compiled_class_hash": to_hex_felt(&casm_hash),
                        "max_fee": to_hex_felt(&max_fee),
                        "version": "0x2",
                        "signature": signature_hex,
                        "nonce": to_num_as_hex(&nonce),
                        "contract_class": flattened_contract_artifact,
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

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            max_fee == FieldElement::ZERO,
        );
    }

    #[tokio::test]
    async fn simulate_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // define the key of the new account - dummy value
        let new_account_signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            new_account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        let nonce = FieldElement::ZERO;
        let salt_hex = "0x123";
        let max_fee = FieldElement::from(1e18 as u128);
        let deployment = account_factory
            .deploy(FieldElement::from_hex_be(salt_hex).unwrap())
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap();
        let deployment_tx_hash = deployment.transaction_hash();

        let signature = new_account_signer.sign_hash(&deployment_tx_hash).await.unwrap();
        let signature_hex: Vec<String> = iter_to_hex_felt(&[signature.r, signature.s]);
        let account_public_key = new_account_signer.get_public_key().await.unwrap().scalar();

        let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
            json!({
                "block_id": "latest",
                "simulation_flags": simulation_flags,
                "transactions": [
                    {
                        "type": "DEPLOY_ACCOUNT",
                        "max_fee": to_hex_felt(&max_fee),
                        "version": "0x1",
                        "signature": signature_hex,
                        "nonce": to_num_as_hex(&nonce),
                        "contract_address_salt": salt_hex,
                        "constructor_calldata": [to_hex_felt(&account_public_key)],
                        "class_hash": CAIRO_0_ACCOUNT_CONTRACT_HASH
                    }
                ]
            })
        };

        let account_address = deployment.address();
        let account_address_hex = to_hex_felt(&account_address);
        devnet.mint(account_address, 1e18 as u128).await;

        // no flags
        let params_no_flags = get_params(&[]);
        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags)
            .await["result"][0];

        let no_flags_trace = &resp_no_flags["transaction_trace"];
        assert_eq!(
            no_flags_trace["validate_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );
        assert_eq!(
            no_flags_trace["fee_transfer_invocation"]["contract_address"].as_str().unwrap(),
            ERC20_CONTRACT_ADDRESS.to_lowercase()
        );
        assert_eq!(
            no_flags_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );

        // skipped validation
        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await["result"][0];
        let skip_validation_trace = &resp_skip_validation["transaction_trace"];
        assert!(skip_validation_trace["validate_invocation"].as_object().is_none());
        assert_eq!(
            skip_validation_trace["fee_transfer_invocation"]["contract_address"].as_str().unwrap(),
            ERC20_CONTRACT_ADDRESS.to_lowercase()
        );
        assert_eq!(
            skip_validation_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );

        assert_fee_in_resp_greater(resp_no_flags, resp_skip_validation);

        // skipped validation and fee charging (everything)
        let params_skip_everything = get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]);
        let resp_skip_everything = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_everything)
            .await["result"][0];
        let skip_everything_trace = &resp_skip_everything["transaction_trace"];
        assert!(skip_everything_trace["validate_invocation"].as_object().is_none());
        assert!(skip_everything_trace["fee_transfer_invocation"].as_object().is_none());
        assert_eq!(
            skip_everything_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );
    }

    #[tokio::test]
    async fn simulate_invoke() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

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
        let class_hash = contract_artifact.class_hash().unwrap();

        // declare class
        let declaration_result =
            account.declare_legacy(contract_artifact.clone()).send().await.unwrap();
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
        contract_factory.deploy(constructor_calldata, salt, false).send().await.unwrap();

        // prepare the call used in simulation
        let increase_amount = FieldElement::from(100u128);
        let invoke_calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increase_amount],
        }];

        // TODO fails if max_fee too low, can be used to test reverted case
        let max_fee = FieldElement::from(1e18 as u128);
        let nonce = FieldElement::from(2_u32); // after declare+deploy
        let invoke_request = account
            .execute(invoke_calls.clone())
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .get_invoke_request(false)
            .await
            .unwrap();
        let signature_hex: Vec<String> = iter_to_hex_felt(&invoke_request.signature);

        let calldata_hex: Vec<String> = iter_to_hex_felt(&invoke_request.calldata);

        let sender_address_hex = to_hex_felt(&account.address());

        let get_params = |simulation_flags: &[&str]| -> serde_json::Value {
            json!({
                "block_id": "latest",
                "simulation_flags": simulation_flags,
                "transactions": [
                    {
                        "type": "INVOKE",
                        "max_fee": to_hex_felt(&max_fee),
                        "version": "0x1",
                        "signature": signature_hex,
                        "nonce": to_num_as_hex(&nonce),
                        "calldata": calldata_hex,
                        "sender_address": sender_address_hex,
                    }
                ]
            })
        };

        let params_no_flags = get_params(&[]);

        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags)
            .await["result"][0];
        assert_eq!(
            resp_no_flags["transaction_trace"]["execute_invocation"]["contract_address"],
            sender_address_hex
        );

        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await["result"][0];
        assert_eq!(
            resp_skip_validation["transaction_trace"]["execute_invocation"]["contract_address"],
            sender_address_hex
        );

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            max_fee == FieldElement::ZERO,
        );
    }
}
