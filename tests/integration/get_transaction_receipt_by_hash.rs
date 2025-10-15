use std::sync::Arc;

use starknet_rs_accounts::{
    Account, AccountFactory, ExecutionEncoding, OpenZeppelinAccountFactory, SingleOwnerAccount,
};
use starknet_rs_core::types::{
    Call, ExecutionResult, Felt, StarknetError, TransactionFinalityStatus, TransactionReceipt,
};
use starknet_rs_core::utils::{get_selector_from_name, get_udc_deployed_address};
use starknet_rs_providers::{Provider, ProviderError};

use crate::common::background_devnet::BackgroundDevnet;
use crate::common::constants::{
    self, CAIRO_0_ACCOUNT_CONTRACT_HASH, CHAIN_ID, ETH_ERC20_CONTRACT_ADDRESS,
    STRK_ERC20_CONTRACT_ADDRESS, UDC_CONTRACT_ADDRESS,
};
use crate::common::utils::{
    assert_contains, get_deployable_account_signer, get_events_contract_artifacts,
    new_contract_factory,
};

#[tokio::test]
async fn deploy_account_transaction_receipt() {
    let devnet = BackgroundDevnet::spawn().await.expect("Could not start Devnet");

    let signer = get_deployable_account_signer();
    let account_factory = OpenZeppelinAccountFactory::new(
        Felt::from_hex_unchecked(CAIRO_0_ACCOUNT_CONTRACT_HASH),
        CHAIN_ID,
        signer.clone(),
        devnet.clone_provider(),
    )
    .await
    .unwrap();
    let new_account_nonce = Felt::ZERO;
    let salt = Felt::THREE;
    let deployment = account_factory.deploy_v3(salt).nonce(new_account_nonce);
    let new_account_address = deployment.address();
    devnet.mint(new_account_address, 1e18 as u128).await;

    // Converting Felt to u64 for the gas parameter
    let deploy_account_result = deployment.send().await.unwrap();

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

    let (signer, account_address) = devnet.get_first_predeployed_account().await;
    let predeployed_account = Arc::new(SingleOwnerAccount::new(
        devnet.clone_provider(),
        signer,
        account_address,
        constants::CHAIN_ID,
        ExecutionEncoding::New,
    ));

    let (cairo_1_contract, casm_class_hash) = get_events_contract_artifacts();

    // declare the contract
    let declaration_result = predeployed_account
        .declare_v3(Arc::new(cairo_1_contract), casm_class_hash)
        .send()
        .await
        .unwrap();

    // deploy the contract
    let contract_factory =
        new_contract_factory(declaration_result.class_hash, predeployed_account.clone());

    let salt = Felt::ZERO;
    let constructor_args = Vec::<Felt>::new();
    let deployment_result =
        contract_factory.deploy_v3(constructor_args.clone(), salt, false).send().await.unwrap();

    let deployment_receipt = devnet
        .json_rpc_client
        .get_transaction_receipt(deployment_result.transaction_hash)
        .await
        .unwrap()
        .receipt;

    match deployment_receipt {
        TransactionReceipt::Invoke(receipt) => {
            assert_eq!(receipt.finality_status, TransactionFinalityStatus::AcceptedOnL2);

            let expected_contract_address = get_udc_deployed_address(
                salt,
                declaration_result.class_hash,
                &starknet_rs_core::utils::UdcUniqueness::NotUnique,
                &constructor_args,
            );

            assert_eq!(receipt.events.len(), 2); // UDC and STRK events

            // Assert UDC event
            let deployment_event =
                receipt.events.iter().find(|e| e.from_address == UDC_CONTRACT_ADDRESS).unwrap();
            assert_eq!(
                deployment_event.keys,
                vec![get_selector_from_name("ContractDeployed").unwrap()]
            );
            assert_eq!(deployment_event.data[0], expected_contract_address);
            assert_eq!(deployment_event.data[1], account_address);

            // Assert STRK fee charge event
            let fee_charge_event = receipt
                .events
                .iter()
                .find(|e| e.from_address == STRK_ERC20_CONTRACT_ADDRESS)
                .unwrap();
            assert_eq!(fee_charge_event.keys, vec![get_selector_from_name("Transfer").unwrap()]);
            assert_eq!(fee_charge_event.data[0], account_address);
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

    let (cairo_1_contract, casm_class_hash) = get_events_contract_artifacts();

    // declare the contract
    let declaration_result = predeployed_account
        .declare_v3(Arc::new(cairo_1_contract), casm_class_hash)
        .send()
        .await
        .unwrap();

    // try deploying with invalid constructor args - none are expected, we are providing [1]
    let contract_factory =
        new_contract_factory(declaration_result.class_hash, predeployed_account.clone());

    let salt = Felt::ZERO;
    let invalid_constructor_args = vec![Felt::ONE];
    let l1_gas = 0;
    let l1_data_gas = 1000;
    let l2_gas = 1e6 as u64;
    let invalid_deployment_result = contract_factory
        .deploy_v3(invalid_constructor_args, salt, false)
        .l1_gas(l1_gas)
        .l1_data_gas(l1_data_gas)
        .l2_gas(l2_gas)
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
            assert_eq!(receipt.finality_status, TransactionFinalityStatus::AcceptedOnL2);
            assert!(receipt.execution_resources.l1_gas <= l1_gas);
            assert!(receipt.execution_resources.l1_data_gas <= l1_data_gas);
            assert!(receipt.execution_resources.l2_gas <= l2_gas);
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

    let transfer_execution = predeployed_account.execute_v3(vec![Call {
        to: ETH_ERC20_CONTRACT_ADDRESS,
        selector: get_selector_from_name("transfer").unwrap(),
        calldata: vec![
            Felt::ONE,                 // recipient
            Felt::from(1_000_000_000), // low part of uint256
            Felt::ZERO,                // high part of uint256
        ],
    }]);

    let fee = transfer_execution.estimate_fee().await.unwrap();

    // send transaction with lower than estimated overall fee; should revert
    let transfer_result = transfer_execution
        .l1_gas(fee.l1_gas_consumed) // Using as is, ok to be 0
        // .l2_gas(u64::try_from(fee.l2_gas_consumed).unwrap() - 1) // subtract to induce failure
        .l2_gas(1156800 - 1) // TODO: determined experimentally as the actual value used minus 1
        .l1_data_gas(fee.l1_data_gas_consumed) // using as is
        .send()
        .await
        .unwrap();

    let transfer_receipt = devnet
        .json_rpc_client
        .get_transaction_receipt(transfer_result.transaction_hash)
        .await
        .unwrap()
        .receipt;

    match transfer_receipt {
        TransactionReceipt::Invoke(receipt) => {
            match receipt.execution_result {
                starknet_rs_core::types::ExecutionResult::Reverted { reason } => {
                    assert_contains(&reason, "Insufficient max L2Gas").unwrap();
                }
                _ => panic!("Invalid receipt {:?}", receipt),
            }
            // due to earlier l2_gas - 1
            assert!(receipt.actual_fee.amount <= Felt::from(fee.overall_fee - fee.l2_gas_price));
        }
        _ => panic!("Invalid receipt {:?}", transfer_receipt),
    };
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
