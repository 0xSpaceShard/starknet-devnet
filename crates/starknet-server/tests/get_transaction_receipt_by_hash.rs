pub mod common;

mod get_transaction_receipt_by_hash_integration_tests {

    use std::sync::Arc;

    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ERC20_CONTRACT_ADDRESS};
    use starknet_rs_accounts::{
        Account, AccountFactory, Call, ExecutionEncoding, OpenZeppelinAccountFactory,
        SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::chain_id;
    use starknet_rs_core::types::{
        BroadcastedDeclareTransactionV1, ExecutionResult, FieldElement,
        MaybePendingTransactionReceipt, StarknetError, TransactionReceipt,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::{
        MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage,
    };

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::CHAIN_ID;
    use crate::common::utils::{
        get_deployable_account_signer, get_events_contract_in_sierra_and_compiled_class_hash,
    };

    #[tokio::test]
    async fn deploy_account_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            FieldElement::from_hex_be(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = FieldElement::ZERO;
        let salt = FieldElement::THREE;
        let deployment = account_factory.deploy(salt).nonce(new_account_nonce);
        let new_account_address = deployment.address();
        devnet.mint(new_account_address, 1e18 as u128).await;

        let deploy_account_result = deployment.send().await.unwrap();

        let deploy_account_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deploy_account_result.transaction_hash)
            .await
            .unwrap();

        match deploy_account_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::DeployAccount(receipt)) => {
                assert_eq!(receipt.contract_address, new_account_address);
            }
            _ => {
                panic!("Invalid receipt {:?}", deploy_account_receipt);
            }
        }
    }

    #[tokio::test]
    async fn deploy_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        ));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());

        let salt = FieldElement::ZERO;
        let constructor_args = Vec::<FieldElement>::new();
        let max_fee = FieldElement::from(1e18 as u128);
        let deployment_result = contract_factory
            .deploy(constructor_args.clone(), salt, false)
            .max_fee(max_fee)
            .send()
            .await
            .unwrap();

        let deployment_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deployment_result.transaction_hash)
            .await
            .unwrap();

        match deployment_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Deploy(receipt)) => {
                let expected_contract_address = get_udc_deployed_address(
                    salt,
                    declaration_result.class_hash,
                    &starknet_rs_core::utils::UdcUniqueness::NotUnique,
                    &constructor_args,
                );
                assert_eq!(receipt.contract_address, expected_contract_address);
                assert!(receipt.actual_fee < max_fee);
            }
            _ => panic!("Invalid receipt {:?}", deployment_receipt),
        };
    }

    #[tokio::test]
    async fn invalid_deploy_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = Arc::new(SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        ));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        // declare the contract
        let declaration_result = predeployed_account
            .declare(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(FieldElement::from(1e18 as u128))
            .send()
            .await
            .unwrap();

        // try deploying with invalid constructor args - none are expected, we are providing [1]
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());

        let salt = FieldElement::ZERO;
        let invalid_constructor_args = vec![FieldElement::ONE];
        let max_fee = FieldElement::from(1e18 as u128);
        let invalid_deployment_result = contract_factory
            .deploy(invalid_constructor_args, salt, false)
            .max_fee(max_fee)
            .send()
            .await
            .unwrap();

        let invalid_deployment_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(invalid_deployment_result.transaction_hash)
            .await
            .unwrap();
        match invalid_deployment_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => {
                match receipt.execution_result {
                    ExecutionResult::Reverted { reason } => {
                        assert!(reason.contains("Input too long for arguments"));
                    }
                    other => panic!("Invalid execution result {other:?}"),
                }
                assert!(receipt.actual_fee < max_fee);
            }
            _ => panic!("Invalid receipt {:?}", invalid_deployment_receipt),
        };
    }

    #[tokio::test]
    async fn reverted_invoke_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let (signer, account_address) = devnet.get_first_predeployed_account().await;
        let predeployed_account = SingleOwnerAccount::new(
            devnet.clone_provider(),
            signer,
            account_address,
            chain_id::TESTNET,
            ExecutionEncoding::Legacy,
        );

        let transfer_execution = predeployed_account.execute(vec![Call {
            to: FieldElement::from_hex_be(ERC20_CONTRACT_ADDRESS).unwrap(),
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                FieldElement::ONE,                                 // recipient
                FieldElement::from_dec_str("1000000000").unwrap(), // low part of uint256
                FieldElement::ZERO,                                // high part of uint256
            ],
        }]);

        let fee = transfer_execution.estimate_fee().await.unwrap();

        // send transaction with lower than estimated fee
        // should revert
        let max_fee = FieldElement::from(fee.overall_fee - 1);
        let transfer_result = transfer_execution.max_fee(max_fee).send().await.unwrap();

        let transfer_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(transfer_result.transaction_hash)
            .await
            .unwrap();

        match transfer_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => {
                match receipt.execution_result {
                    starknet_rs_core::types::ExecutionResult::Reverted { .. } => (),
                    _ => panic!("Invalid receipt {:?}", receipt),
                }
                assert_eq!(receipt.actual_fee, max_fee);
            }
            _ => panic!("Invalid receipt {:?}", transfer_receipt),
        };
    }

    #[tokio::test]
    async fn declare_v1_transaction_fails_with_insufficient_max_fee() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let declare_transaction_result = devnet
            .json_rpc_client
            .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                declare_txn_v1.clone(),
            ))
            .await;

        match declare_transaction_result {
            Err(ProviderError::StarknetError(StarknetErrorWithMessage { code, message: _ })) => {
                match code {
                    MaybeUnknownErrorCode::Known(StarknetError::InsufficientMaxFee) => (),
                    _ => panic!("Invalid error: {:?}", code),
                }
            }
            _ => {
                panic!("Invalid result: {:?}", declare_transaction_result);
            }
        }
    }

    #[tokio::test]
    async fn declare_v1_accepted_with_numeric_entrypoint_offset() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let declare_file_content = std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/rpc/declare_v1.json"
        ))
        .unwrap();
        let mut declare_rpc_body: serde_json::Value =
            serde_json::from_reader(declare_file_content).unwrap();

        let entry_points = declare_rpc_body["contract_class"]["entry_points_by_type"]["EXTERNAL"]
            .as_array_mut()
            .unwrap();
        for entry_point in entry_points {
            // We are assuming hex string format in the loaded artifact;
            // Converting it to numeric value to test that case
            let offset_hex_string = entry_point["offset"].as_str().unwrap();
            entry_point["offset"] =
                u32::from_str_radix(&offset_hex_string[2..], 16).unwrap().into();
        }

        let resp = &devnet
            .send_custom_rpc(
                "starknet_addDeclareTransaction",
                serde_json::json!({ "declare_transaction": declare_rpc_body }),
            )
            .await;

        match resp["error"]["code"].as_u64() {
            Some(53) => {
                // We got error code corresponding to insufficient balance, which is ok;
                // it's important we didn't get failed JSON schema matching with error -32602
            }
            _ => panic!("Unexpected response: {resp}"),
        }
    }

    #[tokio::test]
    async fn get_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet
            .json_rpc_client
            .get_transaction_receipt(FieldElement::from_hex_be("0x0").unwrap())
            .await
            .unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                ..
            }) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
