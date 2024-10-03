#![cfg(test)]
pub mod common;

mod get_transaction_receipt_by_hash_integration_tests {

    use std::sync::Arc;

    use server::test_utils::declare_v1_str;
    use starknet_core::constants::{CAIRO_0_ACCOUNT_CONTRACT_HASH, ETH_ERC20_CONTRACT_ADDRESS};
    use starknet_rs_accounts::{
        Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
    };
    use starknet_rs_contract::ContractFactory;
    use starknet_rs_core::types::{
        BroadcastedDeclareTransactionV1, Call, ExecutionResult, Felt, StarknetError,
        TransactionReceipt,
    };
    use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
    use starknet_rs_providers::{Provider, ProviderError};
    use starknet_types::felt::felt_from_prefixed_hex;

    use crate::common::background_devnet::BackgroundDevnet;
    use crate::common::constants::{self, CHAIN_ID};
    use crate::common::utils::{
        get_deployable_account_signer, get_events_contract_in_sierra_and_compiled_class_hash,
    };

    #[tokio::test]
    async fn deploy_account_transaction_receipt() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

        let signer = get_deployable_account_signer();
        let account_factory = OpenZeppelinAccountFactory::new(
            felt_from_prefixed_hex(CAIRO_0_ACCOUNT_CONTRACT_HASH).unwrap(),
            CHAIN_ID,
            signer.clone(),
            devnet.clone_provider(),
        )
        .await
        .unwrap();
        let new_account_nonce = Felt::ZERO;
        let salt = Felt::THREE;
        let deployment = account_factory.deploy_v1(salt).nonce(new_account_nonce);
        let new_account_address = deployment.address();
        devnet.mint(new_account_address, 1e18 as u128).await;

        let deploy_account_result =
            deployment.max_fee(Felt::from(1e18 as u128)).send().await.unwrap();

        let deploy_account_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deploy_account_result.transaction_hash)
            .await
            .unwrap()
            .receipt;

        match deploy_account_receipt {
            TransactionReceipt::DeployAccount(receipt) => {
                assert_eq!(receipt.contract_address, new_account_address);
            }
            _ => panic!("Invalid receipt {:?}", deploy_account_receipt),
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
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        let max_fee = Felt::from(1e18 as u128);

        // declare the contract
        let declaration_result = predeployed_account
            .declare_v2(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(max_fee)
            .send()
            .await
            .unwrap();

        // deploy the contract
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());

        let salt = Felt::ZERO;
        let constructor_args = Vec::<Felt>::new();
        let deployment_result = contract_factory
            .deploy_v1(constructor_args.clone(), salt, false)
            .max_fee(max_fee)
            .send()
            .await
            .unwrap();

        let deployment_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(deployment_result.transaction_hash)
            .await
            .unwrap()
            .receipt;

        match deployment_receipt {
            TransactionReceipt::Deploy(receipt) => {
                let expected_contract_address = get_udc_deployed_address(
                    salt,
                    declaration_result.class_hash,
                    &starknet_rs_core::utils::UdcUniqueness::NotUnique,
                    &constructor_args,
                );
                assert_eq!(receipt.contract_address, expected_contract_address);
                assert!(receipt.actual_fee.amount < max_fee);
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
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        ));

        let (cairo_1_contract, casm_class_hash) =
            get_events_contract_in_sierra_and_compiled_class_hash();

        let max_fee = Felt::from(1e18 as u128);

        // declare the contract
        let declaration_result = predeployed_account
            .declare_v2(Arc::new(cairo_1_contract), casm_class_hash)
            .max_fee(max_fee)
            .send()
            .await
            .unwrap();

        // try deploying with invalid constructor args - none are expected, we are providing [1]
        let contract_factory =
            ContractFactory::new(declaration_result.class_hash, predeployed_account.clone());

        let salt = Felt::ZERO;
        let invalid_constructor_args = vec![Felt::ONE];
        let invalid_deployment_result = contract_factory
            .deploy_v1(invalid_constructor_args, salt, false)
            .max_fee(max_fee)
            .send()
            .await
            .unwrap();

        let invalid_deployment_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(invalid_deployment_result.transaction_hash)
            .await
            .unwrap()
            .receipt;
        match invalid_deployment_receipt {
            TransactionReceipt::Invoke(receipt) => {
                match receipt.execution_result {
                    ExecutionResult::Reverted { reason } => {
                        assert!(reason.contains("Input too long for arguments"));
                    }
                    other => panic!("Invalid execution result {other:?}"),
                }
                assert!(receipt.actual_fee.amount < max_fee);
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
            constants::CHAIN_ID,
            ExecutionEncoding::New,
        );

        let transfer_execution = predeployed_account.execute_v1(vec![Call {
            to: ETH_ERC20_CONTRACT_ADDRESS,
            selector: get_selector_from_name("transfer").unwrap(),
            calldata: vec![
                Felt::ONE,                 // recipient
                Felt::from(1_000_000_000), // low part of uint256
                Felt::ZERO,                // high part of uint256
            ],
        }]);

        let fee = transfer_execution.estimate_fee().await.unwrap();

        // send transaction with lower than estimated fee
        // should revert
        let max_fee = fee.overall_fee - Felt::ONE;
        let transfer_result = transfer_execution.max_fee(max_fee).send().await.unwrap();

        let transfer_receipt = devnet
            .json_rpc_client
            .get_transaction_receipt(transfer_result.transaction_hash)
            .await
            .unwrap()
            .receipt;

        match transfer_receipt {
            TransactionReceipt::Invoke(receipt) => {
                match receipt.execution_result {
                    starknet_rs_core::types::ExecutionResult::Reverted { .. } => (),
                    _ => panic!("Invalid receipt {:?}", receipt),
                }
                assert_eq!(receipt.actual_fee.amount, max_fee);
            }
            _ => panic!("Invalid receipt {:?}", transfer_receipt),
        };
    }

    #[tokio::test]
    async fn declare_v1_transaction_fails_with_insufficient_max_fee() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let json_string = declare_v1_str();
        let declare_txn_v1: BroadcastedDeclareTransactionV1 =
            serde_json::from_str(&json_string).unwrap();

        let declare_transaction_result = devnet
            .json_rpc_client
            .add_declare_transaction(starknet_rs_core::types::BroadcastedDeclareTransaction::V1(
                declare_txn_v1.clone(),
            ))
            .await;

        match declare_transaction_result {
            Err(ProviderError::StarknetError(StarknetError::InsufficientMaxFee)) => (),
            _ => panic!("Invalid result: {:?}", declare_transaction_result),
        }
    }

    #[tokio::test]
    async fn declare_v1_accepted_with_numeric_entrypoint_offset() {
        let devnet = BackgroundDevnet::spawn().await.unwrap();

        let declare_v1 = declare_v1_str();
        let mut declare_rpc_body: serde_json::Value = serde_json::from_str(&declare_v1).unwrap();

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

        let rpc_error = devnet
            .send_custom_rpc(
                "starknet_addDeclareTransaction",
                serde_json::json!({ "declare_transaction": declare_rpc_body }),
            )
            .await
            .unwrap_err();

        // We got error code corresponding to insufficient balance, which is ok;
        // it's important we didn't get failed JSON schema matching with error -32602
        assert_eq!(rpc_error.code.code(), 53);
    }

    #[tokio::test]
    async fn get_non_existing_transaction() {
        let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");
        let result = devnet.json_rpc_client.get_transaction_receipt(Felt::ZERO).await.unwrap_err();

        match result {
            ProviderError::StarknetError(StarknetError::TransactionHashNotFound) => (),
            _ => panic!("Invalid error: {result:?}"),
        }
    }
}
