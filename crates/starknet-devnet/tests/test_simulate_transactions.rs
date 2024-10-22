#![cfg(test)]
pub mod common;

mod simulation_tests {
    use std::sync::Arc;
    use std::{u128, u64};

    use serde_json::json;
    use server::test_utils::assert_contains;
    use starknet_core::constants::{
        CAIRO_0_ACCOUNT_CONTRACT_HASH, CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH,
        ETH_ERC20_CONTRACT_ADDRESS, UDC_CONTRACT_ADDRESS,
    };
    use starknet_core::utils::exported_test_utils::dummy_cairo_0_contract_class;
    use starknet_rs_accounts::{
        Account, AccountError, AccountFactory, ConnectedAccount, ExecutionEncoder,
        ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::contract::legacy::LegacyContractClass;
    use starknet_rs_core::types::{
        BlockId, BlockTag, BroadcastedDeclareTransaction, BroadcastedDeclareTransactionV3,
        BroadcastedDeployAccountTransaction, BroadcastedDeployAccountTransactionV3,
        BroadcastedInvokeTransaction, BroadcastedInvokeTransactionV3, BroadcastedTransaction, Call,
        DataAvailabilityMode, ExecuteInvocation, Felt, FunctionCall, InvokeTransactionTrace,
        ResourceBounds, ResourceBoundsMapping, SimulatedTransaction, SimulationFlag, StarknetError,
        TransactionExecutionErrorData, TransactionTrace,
    };
    use starknet_rs_core::utils::{
        cairo_short_string_to_felt, get_selector_from_name, get_udc_deployed_address, UdcUniqueness,
    };
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_rs_signers::{LocalWallet, Signer, SigningKey};
    use starknet_types::constants::QUERY_VERSION_OFFSET;
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{
        self, CAIRO_1_CONTRACT_PATH, CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH,
        CAIRO_1_VERSION_ASSERTER_SIERRA_PATH, CHAIN_ID,
    };
    use crate::common::fees::{assert_difference_if_validation, assert_fee_in_resp_at_least_equal};
    use crate::common::utils::{
        declare_v3_deploy_v3, get_deployable_account_signer,
        get_flattened_sierra_contract_and_casm_hash, get_gas_units_and_gas_price,
        get_simple_contract_in_sierra_and_compiled_class_hash, iter_to_hex_felt, to_hex_felt,
        to_num_as_hex,
    };

    #[tokio::test]
    async fn simulate_declare_v1() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        );

        let contract_json = dummy_cairo_0_contract_class();
        let contract_artifact: Arc<LegacyContractClass> =
            Arc::new(serde_json::from_value(contract_json.inner).unwrap());

        let max_fee = Felt::ZERO; // TODO try 1e18 as u128 instead
        let nonce = Felt::ZERO;

        let declaration = account
            .declare_legacy(contract_artifact.clone())
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap();
        let declaration_hash = declaration.transaction_hash(false).unwrap();
        let signature = signer.sign_hash(&declaration_hash).await.unwrap();

        let sender_address_hex = account.address().to_hex_string();
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
                        "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                        "nonce": to_num_as_hex(&nonce),
                        "contract_class": contract_artifact.compress().unwrap(),
                    }
                ]
            })
        };

        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", get_params(&["SKIP_FEE_CHARGE"]))
            .await
            .unwrap()[0];

        let resp_skip_validation = &devnet
            .send_custom_rpc(
                "starknet_simulateTransactions",
                get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]),
            )
            .await
            .unwrap()[0];

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            true,
        );
    }

    #[tokio::test]
    async fn simulate_declare_v2() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        );

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_CONTRACT_PATH);

        let max_fee = Felt::ZERO;
        let nonce = Felt::ZERO;

        let declaration = account
            .declare_v2(Arc::new(flattened_contract_artifact.clone()), casm_hash)
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap();

        let declaration_hash = declaration.transaction_hash(false);
        let signature = signer.sign_hash(&declaration_hash).await.unwrap();

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
                        "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                        "nonce": to_num_as_hex(&nonce),
                        "contract_class": flattened_contract_artifact,
                    }
                ]
            })
        };

        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", get_params(&["SKIP_FEE_CHARGE"]))
            .await
            .unwrap()[0];

        let resp_skip_validation = &devnet
            .send_custom_rpc(
                "starknet_simulateTransactions",
                get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]),
            )
            .await
            .unwrap()[0];

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            true,
        );
    }

    #[tokio::test]
    async fn simulate_deploy_account() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // define the key of the new account - dummy value
        let new_account_signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            felt_from_prefixed_hex(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            new_account_signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();

        let nonce = Felt::ZERO;
        let salt_hex = "0x123";
        let max_fee = Felt::from(1e18 as u128);
        let deployment = account_factory
            .deploy_v1(felt_from_prefixed_hex(salt_hex).unwrap())
            .max_fee(max_fee)
            .nonce(nonce)
            .prepared()
            .unwrap();
        let deployment_tx_hash = deployment.transaction_hash(false);

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
            .await
            .unwrap()[0];

        let no_flags_trace = &resp_no_flags["transaction_trace"];
        assert_eq!(
            no_flags_trace["validate_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );
        assert_eq!(
            Felt::from_hex_unchecked(
                no_flags_trace["fee_transfer_invocation"]["contract_address"].as_str().unwrap()
            ),
            ETH_ERC20_CONTRACT_ADDRESS
        );
        assert_eq!(
            no_flags_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );

        // skipped validation
        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await
            .unwrap()[0];
        let skip_validation_trace = &resp_skip_validation["transaction_trace"];
        assert!(skip_validation_trace["validate_invocation"].as_object().is_none());
        assert_eq!(
            Felt::from_hex_unchecked(
                skip_validation_trace["fee_transfer_invocation"]["contract_address"]
                    .as_str()
                    .unwrap()
            ),
            ETH_ERC20_CONTRACT_ADDRESS
        );
        assert_eq!(
            skip_validation_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );

        assert_fee_in_resp_at_least_equal(resp_no_flags, resp_skip_validation);

        // skipped validation and fee charging (everything)
        let params_skip_everything = get_params(&["SKIP_VALIDATE", "SKIP_FEE_CHARGE"]);
        let resp_skip_everything = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_everything)
            .await
            .unwrap()[0];
        let skip_everything_trace = &resp_skip_everything["transaction_trace"];
        assert!(skip_everything_trace["validate_invocation"].as_object().is_none());
        assert!(skip_everything_trace["fee_transfer_invocation"].as_object().is_none());
        assert_eq!(
            skip_everything_trace["constructor_invocation"]["contract_address"].as_str().unwrap(),
            account_address_hex
        );
    }

    #[tokio::test]
    async fn simulate_invoke_v1() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer.clone(),
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
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
        let salt = Felt::from_hex_unchecked("0x123");
        let constructor_calldata = vec![];
        let contract_address = get_udc_deployed_address(
            salt,
            class_hash,
            &UdcUniqueness::NotUnique,
            &constructor_calldata,
        );
        contract_factory.deploy_v1(constructor_calldata, salt, false).send().await.unwrap();

        // prepare the call used in simulation
        let increase_amount = Felt::from(100u128);
        let calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("increase_balance").unwrap(),
            calldata: vec![increase_amount],
        }];

        // TODO fails if max_fee too low, can be used to test reverted case
        let max_fee = Felt::from(1e18 as u128);
        let nonce = Felt::TWO; // after declare+deploy
        let invoke_request =
            account.execute_v1(calls.clone()).max_fee(max_fee).nonce(nonce).prepared().unwrap();

        let signature = signer.sign_hash(&invoke_request.transaction_hash(false)).await.unwrap();
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
                        "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                        "nonce": to_num_as_hex(&nonce),
                        "calldata": iter_to_hex_felt(&account.encode_calls(&calls)),
                        "sender_address": sender_address_hex,
                    }
                ]
            })
        };

        let params_no_flags = get_params(&[]);

        let resp_no_flags = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_no_flags)
            .await
            .unwrap()[0];
        assert_eq!(
            resp_no_flags["transaction_trace"]["execute_invocation"]["contract_address"],
            sender_address_hex
        );

        let params_skip_validation = get_params(&["SKIP_VALIDATE"]);
        let resp_skip_validation = &devnet
            .send_custom_rpc("starknet_simulateTransactions", params_skip_validation)
            .await
            .unwrap()[0];
        assert_eq!(
            resp_skip_validation["transaction_trace"]["execute_invocation"]["contract_address"],
            sender_address_hex
        );

        assert_difference_if_validation(
            resp_no_flags,
            resp_skip_validation,
            &sender_address_hex,
            max_fee == Felt::ZERO,
        );

        // assert simulations haven't changed the balance property
        let final_balance = devnet
            .json_rpc_client
            .call(
                FunctionCall {
                    contract_address,
                    entry_point_selector: get_selector_from_name("get_balance").unwrap(),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .unwrap();
        assert_eq!(final_balance, vec![Felt::ZERO]);
    }

    #[tokio::test]
    async fn using_query_version_if_simulating() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer.clone(),
            account_address,
            CHAIN_ID,
            ExecutionEncoding::New,
        );

        // get class
        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_VERSION_ASSERTER_SIERRA_PATH);
        let class_hash = flattened_contract_artifact.class_hash();

        let (generated_class_hash, contract_address) =
            declare_v3_deploy_v3(&account, flattened_contract_artifact, casm_hash, &[])
                .await
                .unwrap();
        assert_eq!(generated_class_hash, class_hash);

        let calls = vec![Call {
            to: contract_address,
            selector: get_selector_from_name("assert_version").unwrap(),
            calldata: vec![QUERY_VERSION_OFFSET + Felt::ONE], // expected version
        }];

        let max_fee = Felt::from(1e18 as u128);
        let nonce = Felt::TWO; // after declare+deploy
        let invoke_request =
            account.execute_v1(calls.clone()).max_fee(max_fee).nonce(nonce).prepared().unwrap();

        let signature = signer.sign_hash(&invoke_request.transaction_hash(false)).await.unwrap();
        let invoke_simulation_body = json!({
            "block_id": "latest",
            "simulation_flags": [],
            "transactions": [
                {
                    "type": "INVOKE",
                    "max_fee": max_fee.to_hex_string(),
                    "version": "0x1",
                    "signature": iter_to_hex_felt(&[signature.r, signature.s]),
                    "nonce": to_num_as_hex(&nonce),
                    "calldata": iter_to_hex_felt(&account.encode_calls(&calls)),
                    "sender_address": account.address().to_hex_string(),
                }
            ]
        });

        devnet
            .send_custom_rpc("starknet_simulateTransactions", invoke_simulation_body)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn simulate_of_multiple_txs_shouldnt_return_an_error_if_invoke_transaction_reverts() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer.clone(),
            account_address,
            devnet.json_rpc_client.chain_id().await.unwrap(),
            ExecutionEncoding::New,
        );

        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
        let class_hash = flattened_contract_artifact.class_hash();

        let estimate_fee_resource_bounds = ResourceBoundsMapping {
            l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
            l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        };

        // call non existent method in UDC
        let calls = vec![Call {
            to: UDC_CONTRACT_ADDRESS,
            selector: get_selector_from_name("no_such_method").unwrap(),
            calldata: vec![
                class_hash,
                Felt::from_hex_unchecked("0x123"), // salt
                Felt::ZERO,
                Felt::ZERO,
            ],
        }];

        let calldata = account.encode_calls(&calls);

        let is_query = true;
        let nonce_data_availability_mode = DataAvailabilityMode::L1;
        let fee_data_availability_mode = DataAvailabilityMode::L1;

        let simulation_result = devnet
            .json_rpc_client
            .simulate_transactions(
                account.block_id(),
                [
                    BroadcastedTransaction::Declare(
                        starknet_rs_core::types::BroadcastedDeclareTransaction::V3(
                            BroadcastedDeclareTransactionV3 {
                                sender_address: account_address,
                                compiled_class_hash: casm_hash,
                                signature: vec![],
                                nonce: Felt::ZERO,
                                contract_class: Arc::new(flattened_contract_artifact.clone()),
                                resource_bounds: estimate_fee_resource_bounds.clone(),
                                tip: 0,
                                paymaster_data: vec![],
                                account_deployment_data: vec![],
                                nonce_data_availability_mode,
                                fee_data_availability_mode,
                                is_query,
                            },
                        ),
                    ),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(
                        BroadcastedInvokeTransactionV3 {
                            sender_address: account_address,
                            calldata,
                            signature: vec![],
                            nonce: Felt::ONE,
                            resource_bounds: estimate_fee_resource_bounds,
                            tip: 0,
                            paymaster_data: vec![],
                            account_deployment_data: vec![],
                            nonce_data_availability_mode,
                            fee_data_availability_mode,
                            is_query,
                        },
                    )),
                ],
                [SimulationFlag::SkipValidate, SimulationFlag::SkipFeeCharge],
            )
            .await
            .unwrap();

        match &simulation_result[1].transaction_trace {
            TransactionTrace::Invoke(InvokeTransactionTrace {
                execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
                ..
            }) => {
                assert_contains(&reverted_invocation.revert_reason, "not found in contract");
            }
            other_trace => panic!("Unexpected trace {:?}", other_trace),
        }
    }

    #[tokio::test]
    async fn simulate_of_multiple_txs_should_return_index_of_first_failing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start devnet");

        // get account
        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer.clone(),
            account_address,
            devnet.json_rpc_client.chain_id().await.unwrap(),
            ExecutionEncoding::New,
        );

        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let (flattened_contract_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);
        let class_hash = flattened_contract_artifact.class_hash();

        let estimate_fee_resource_bounds = ResourceBoundsMapping {
            l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
            l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        };

        // call non existent method in UDC
        let calls = vec![Call {
            to: UDC_CONTRACT_ADDRESS,
            selector: get_selector_from_name("no_such_method").unwrap(),
            calldata: vec![
                class_hash,
                Felt::from_hex_unchecked("0x123"), // salt
                Felt::ZERO,
                Felt::ZERO,
            ],
        }];

        let calldata = account.encode_calls(&calls);

        let is_query = true;
        let nonce_data_availability_mode = DataAvailabilityMode::L1;
        let fee_data_availability_mode = DataAvailabilityMode::L1;

        let simulation_err = devnet
            .json_rpc_client
            .simulate_transactions(
                account.block_id(),
                [
                    BroadcastedTransaction::Declare(
                        starknet_rs_core::types::BroadcastedDeclareTransaction::V3(
                            BroadcastedDeclareTransactionV3 {
                                sender_address: account_address,
                                compiled_class_hash: casm_hash,
                                signature: vec![],
                                nonce: Felt::ZERO,
                                contract_class: Arc::new(flattened_contract_artifact.clone()),
                                resource_bounds: estimate_fee_resource_bounds.clone(),
                                tip: 0,
                                paymaster_data: vec![],
                                account_deployment_data: vec![],
                                nonce_data_availability_mode,
                                fee_data_availability_mode,
                                is_query,
                            },
                        ),
                    ),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(
                        BroadcastedInvokeTransactionV3 {
                            sender_address: account_address,
                            calldata,
                            signature: vec![],
                            nonce: Felt::ONE,
                            resource_bounds: estimate_fee_resource_bounds,
                            tip: 0,
                            paymaster_data: vec![],
                            account_deployment_data: vec![],
                            nonce_data_availability_mode,
                            fee_data_availability_mode,
                            is_query,
                        },
                    )),
                ],
                [],
            )
            .await
            .unwrap_err();

        match simulation_err {
            ProviderError::StarknetError(StarknetError::TransactionExecutionError(
                TransactionExecutionErrorData { transaction_index, .. },
            )) => {
                assert_eq!(transaction_index, 0);
            }
            other_error => panic!("Unexpected error {:?}", other_error),
        }
    }

    #[tokio::test]
    async fn simulate_with_max_fee_exceeding_account_balance_returns_error_if_fee_charge_is_not_skipped()
     {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (sierra_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let declaration = account
            .declare_v3(Arc::new(sierra_artifact), casm_hash)
            .gas(u64::MAX)
            .gas_price(u128::MAX);

        match declaration.simulate(false, false).await.unwrap_err() {
            AccountError::Provider(ProviderError::StarknetError(
                StarknetError::TransactionExecutionError(TransactionExecutionErrorData {
                    execution_error,
                    ..
                }),
            )) => {
                assert_contains(
                    &execution_error,
                    "Account balance is not enough to cover the transaction cost.",
                );
            }
            other => panic!("Unexpected error {other:?}"),
        }

        // should not fail because fee transfer is skipped
        declaration.simulate(false, true).await.unwrap();
    }

    #[tokio::test]
    async fn simulate_v3_with_skip_fee_charge_deploy_account_declare_deploy_via_invoke_to_udc_happy_path()
     {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"])
            .await
            .expect("Could not start Devnet");

        let new_account_private_key = Felt::from(7777);
        let signer =
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(new_account_private_key));

        let latest = BlockId::Tag(BlockTag::Latest);

        let public_key = signer.get_public_key().await.unwrap();
        let salt = Felt::from_hex_unchecked("0x123");
        let account_class_hash = Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH);

        let resource_bounds = ResourceBoundsMapping {
            l1_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
            l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
        };

        let nonce_data_availability_mode = DataAvailabilityMode::L1;
        let fee_data_availability_mode = DataAvailabilityMode::L1;
        let is_query = true;
        let chain_id = devnet.json_rpc_client.chain_id().await.unwrap();
        let paymaster_data = vec![];
        let tip = 0;
        let gas = 0;
        let gas_price = 0;

        let account_factory = OpenZeppelinAccountFactory::new(
            account_class_hash,
            chain_id,
            &signer,
            &devnet.json_rpc_client,
        )
        .await
        .unwrap();

        let nonce = Felt::ZERO;
        let account_deployment =
            account_factory.deploy_v3(salt).nonce(nonce).gas(gas).gas_price(gas_price);

        let account_address = account_deployment.address();
        let txn_hash = account_deployment.prepared().unwrap().transaction_hash(is_query);
        let signature = signer.sign_hash(&txn_hash).await.unwrap();

        let deploy_account_transaction = BroadcastedDeployAccountTransactionV3 {
            nonce,
            signature: vec![signature.r, signature.s],
            contract_address_salt: salt,
            constructor_calldata: vec![public_key.scalar()],
            class_hash: account_class_hash,
            resource_bounds: resource_bounds.clone(),
            tip,
            paymaster_data: paymaster_data.clone(),
            nonce_data_availability_mode,
            fee_data_availability_mode,
            is_query,
        };

        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            &signer,
            account_address,
            chain_id,
            ExecutionEncoding::New,
        );

        let (sierra_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let contract_class_hash = sierra_artifact.class_hash();
        let nonce = Felt::ONE;
        let declare_txn_hash = account
            .declare_v3(Arc::new(sierra_artifact.clone()), casm_hash)
            .nonce(nonce)
            .gas(gas)
            .gas_price(gas_price)
            .prepared()
            .unwrap()
            .transaction_hash(is_query);

        let declare_signature = signer.sign_hash(&declare_txn_hash).await.unwrap();

        let declare_transaction = BroadcastedDeclareTransactionV3 {
            sender_address: account_address,
            compiled_class_hash: casm_hash,
            signature: vec![declare_signature.r, declare_signature.s],
            nonce,
            contract_class: Arc::new(sierra_artifact),
            resource_bounds: resource_bounds.clone(),
            tip,
            paymaster_data: paymaster_data.clone(),
            account_deployment_data: vec![],
            nonce_data_availability_mode,
            fee_data_availability_mode,
            is_query,
        };

        // call non existent method in UDC
        let calls = vec![Call {
            to: UDC_CONTRACT_ADDRESS,
            selector: get_selector_from_name("deployContract").unwrap(),
            calldata: vec![
                contract_class_hash,
                Felt::from_hex_unchecked("0x123"), // salt
                Felt::ZERO,
                Felt::ZERO,
            ],
        }];

        let calldata = account.encode_calls(&calls);
        let nonce = Felt::TWO;
        let invoke_transaction_hash = account
            .execute_v3(calls)
            .gas(gas)
            .gas_price(gas_price)
            .nonce(nonce)
            .prepared()
            .unwrap()
            .transaction_hash(is_query);

        let invoke_signature = signer.sign_hash(&invoke_transaction_hash).await.unwrap();

        let invoke_transaction = BroadcastedInvokeTransactionV3 {
            sender_address: account_address,
            calldata,
            signature: vec![invoke_signature.r, invoke_signature.s],
            nonce,
            resource_bounds: resource_bounds.clone(),
            tip,
            paymaster_data: paymaster_data.clone(),
            account_deployment_data: vec![],
            nonce_data_availability_mode,
            fee_data_availability_mode,
            is_query,
        };

        devnet
            .json_rpc_client
            .simulate_transactions(
                latest,
                [
                    BroadcastedTransaction::DeployAccount(BroadcastedDeployAccountTransaction::V3(
                        deploy_account_transaction,
                    )),
                    BroadcastedTransaction::Declare(BroadcastedDeclareTransaction::V3(
                        declare_transaction,
                    )),
                    BroadcastedTransaction::Invoke(BroadcastedInvokeTransaction::V3(
                        invoke_transaction,
                    )),
                ],
                [SimulationFlag::SkipFeeCharge],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn simulate_invoke_v3_with_fee_just_below_estimated_should_return_a_trace_of_reverted_transaction()
     {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (sierra_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let declare_result =
            account.declare_v3(Arc::new(sierra_artifact.clone()), casm_hash).send().await.unwrap();

        let salt = Felt::from_hex_unchecked("0x123");
        let execution = account.execute_v3(vec![Call {
            to: UDC_CONTRACT_ADDRESS,
            selector: get_selector_from_name("deployContract").unwrap(),
            calldata: vec![
                declare_result.class_hash,
                salt,
                Felt::ZERO, // is_unique
                Felt::ZERO, // constructor data length
            ],
        }]);
        let fee_estimate = execution.estimate_fee().await.unwrap();

        let (gas_units, gas_price) = get_gas_units_and_gas_price(fee_estimate);

        let SimulatedTransaction { transaction_trace, .. } =
            execution.gas(gas_units - 1).gas_price(gas_price).simulate(false, false).await.unwrap();

        match transaction_trace {
            TransactionTrace::Invoke(InvokeTransactionTrace {
                execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
                ..
            }) => assert_contains(&reverted_invocation.revert_reason, "Insufficient"),
            other => panic!("Unexpected trace {other:?}"),
        }
    }

    #[tokio::test]
    async fn simulate_invoke_declare_deploy_account_with_either_gas_or_gas_price_set_to_zero_or_both_will_revert_if_skip_fee_charge_is_not_set()
     {
        let devnet = BackgroundDevnet::spawn_with_additional_args(&["--account-class", "cairo1"])
            .await
            .expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let call = Call {
            to: ETH_ERC20_CONTRACT_ADDRESS,
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE,         // recipient
                Felt::from(1_000), // low part of uint256
                Felt::ZERO,        // high part of uint256
            ],
        };

        let calldata = account.encode_calls(&[call]);

        let new_account_private_key = Felt::from(7777);
        let signer =
            LocalWallet::from_signing_key(SigningKey::from_secret_scalar(new_account_private_key));
        let public_key = signer.get_public_key().await.unwrap().scalar();

        let (sierra_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let account_class_hash = Felt::from_hex_unchecked(CAIRO_1_ACCOUNT_CONTRACT_SIERRA_HASH);

        let nonce_data_availability_mode = DataAvailabilityMode::L1;
        let fee_data_availability_mode = DataAvailabilityMode::L1;
        let tip = 0;

        let nonce = Felt::ZERO;
        let sierra_artifact = Arc::new(sierra_artifact);
        let block_id = BlockId::Tag(BlockTag::Latest);
        let is_query = true;

        for (gas_units, gas_price) in [(0, 0), (0, 1e18 as u128), (1e18 as u64, 0)] {
            let resource_bounds = ResourceBoundsMapping {
                l1_gas: ResourceBounds { max_amount: gas_units, max_price_per_unit: gas_price },
                l2_gas: ResourceBounds { max_amount: 0, max_price_per_unit: 0 },
            };

            let invoke_transaction = BroadcastedInvokeTransactionV3 {
                sender_address: account_address,
                calldata: calldata.clone(),
                signature: vec![],
                nonce,
                resource_bounds: resource_bounds.clone(),
                tip,
                paymaster_data: vec![],
                account_deployment_data: vec![],
                nonce_data_availability_mode,
                fee_data_availability_mode,
                is_query,
            };

            let invoke_transaction = BroadcastedTransaction::Invoke(
                BroadcastedInvokeTransaction::V3(invoke_transaction),
            );

            let deploy_account_transaction = BroadcastedDeployAccountTransactionV3 {
                signature: vec![],
                nonce,
                contract_address_salt: Felt::ZERO,
                constructor_calldata: vec![public_key],
                class_hash: account_class_hash,
                resource_bounds: resource_bounds.clone(),
                tip,
                paymaster_data: vec![],
                nonce_data_availability_mode,
                fee_data_availability_mode,
                is_query,
            };

            let deploy_account_transaction = BroadcastedTransaction::DeployAccount(
                BroadcastedDeployAccountTransaction::V3(deploy_account_transaction),
            );

            let declare_transaction = BroadcastedDeclareTransactionV3 {
                sender_address: account_address,
                compiled_class_hash: casm_hash,
                signature: vec![],
                nonce,
                contract_class: sierra_artifact.clone(),
                resource_bounds,
                tip,
                paymaster_data: vec![],
                account_deployment_data: vec![],
                nonce_data_availability_mode,
                fee_data_availability_mode,
                is_query,
            };

            let declare_transaction = BroadcastedTransaction::Declare(
                BroadcastedDeclareTransaction::V3(declare_transaction),
            );

            for transaction in [deploy_account_transaction, declare_transaction, invoke_transaction]
            {
                let simulation_error = devnet
                    .json_rpc_client
                    .simulate_transaction(block_id, &transaction, [SimulationFlag::SkipValidate])
                    .await
                    .unwrap_err();

                match simulation_error {
                    ProviderError::StarknetError(StarknetError::TransactionExecutionError(
                        TransactionExecutionErrorData { execution_error, .. },
                    )) => {
                        assert_eq!(
                            execution_error,
                            "Provided max fee is not enough to cover the transaction cost."
                        );
                    }
                    other => panic!("Unexpected error: {:?}", other),
                }

                devnet
                    .json_rpc_client
                    .simulate_transaction(
                        block_id,
                        &transaction,
                        [SimulationFlag::SkipValidate, SimulationFlag::SkipFeeCharge],
                    )
                    .await
                    .unwrap();
            }
        }
    }

    #[tokio::test]
    async fn simulate_invoke_v3_with_failing_execution_should_return_a_trace_of_reverted_transaction()
     {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (sierra_artifact, casm_hash) =
            get_flattened_sierra_contract_and_casm_hash(CAIRO_1_PANICKING_CONTRACT_SIERRA_PATH);

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let (_, contract_address) =
            declare_v3_deploy_v3(&account, sierra_artifact, casm_hash, &[]).await.unwrap();

        let panic_reason = "custom little reason";

        let SimulatedTransaction { transaction_trace, .. } = account
            .execute_v3(vec![Call {
                to: contract_address,
                selector: get_selector_from_name("create_panic").unwrap(),
                calldata: vec![cairo_short_string_to_felt(panic_reason).unwrap()],
            }])
            .simulate(false, true)
            .await
            .unwrap();

        match transaction_trace {
            TransactionTrace::Invoke(InvokeTransactionTrace {
                execute_invocation: ExecuteInvocation::Reverted(reverted_invocation),
                ..
            }) => assert_contains(&reverted_invocation.revert_reason, panic_reason),
            other => panic!("Unexpected trace {other:?}"),
        }
    }

    /// Test with lower than (estimated_gas_units * gas_price) using two flags. With
    /// skip_fee_transfer shouldnt fail, without it should fail.
    #[tokio::test]
    async fn simulate_declare_v3_with_less_than_estimated_fee_should_revert_if_fee_charge_is_not_skipped()
     {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let (sierra_artifact, casm_hash) = get_simple_contract_in_sierra_and_compiled_class_hash();

        let (signer, account_address) = devnet.get_first_predeployed_account().await;

        let mut account = SingleOwnerAccount::new(
            &devnet.json_rpc_client,
            signer,
            account_address,
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );
        account.set_block_id(BlockId::Tag(BlockTag::Latest));

        let fee_estimate = account
            .declare_v3(Arc::new(sierra_artifact.clone()), casm_hash)
            .estimate_fee()
            .await
            .unwrap();

        let (gas_units, gas_price) = get_gas_units_and_gas_price(fee_estimate);

        for skip_fee_charge in [true, false] {
            let simulation_result = account
                .declare_v3(Arc::new(sierra_artifact.clone()), casm_hash)
                .gas(gas_units)
                .gas_price(gas_price - 1)
                .simulate(false, skip_fee_charge)
                .await;
            if skip_fee_charge {
                simulation_result.unwrap();
            } else {
                match simulation_result.unwrap_err() {
                    AccountError::Provider(ProviderError::StarknetError(
                        StarknetError::TransactionExecutionError(TransactionExecutionErrorData {
                            execution_error,
                            ..
                        }),
                    )) => {
                        assert_contains(&execution_error, "max fee is not enough");
                    }
                    other_error => panic!("Unexpected error {other_error:?}"),
                }
            }
        }
    }
}
